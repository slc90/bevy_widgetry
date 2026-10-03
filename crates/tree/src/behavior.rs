use crate::{WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeVisibleItem};
use bevy::prelude::*;
use bevy_widgetry_list_view::WidgetryListModel;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::collections::{HashMap, HashSet};

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[component(immutable)]
pub enum WidgetryTreeChildrenState {
    #[default]
    Unknown,
    Loading,
    Loaded,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WidgetryTreeEvent {
    pub entity: Entity,
    pub kind: WidgetryTreeEventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryTreeEventKind {
    Expanded(Entity),
    Collapsed(Entity),
    Selected(Option<Entity>),
    ChildrenRequested(Entity),
}

pub(crate) fn sync_models(world: &mut World) -> Result<(), BevyError> {
    let sources = world
        .query_filtered::<Entity, With<WidgetryTreeModel>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for source in sources {
        if let Err(error) = sync_source(world, source)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn reconcile_list(
    list: &mut WidgetryListModel<WidgetryTreeVisibleItem>,
    items: &[WidgetryTreeVisibleItem],
) -> Result<(), BevyError> {
    let desired: HashSet<_> = items.iter().map(|item| item.entity).collect();
    for index in (0..list.len()).rev() {
        if list
            .get(index)
            .is_some_and(|item| !desired.contains(&item.entity))
        {
            invariant(list.remove(index))?;
        }
    }
    let ids: HashMap<_, _> = (0..list.len())
        .map(|index| {
            Ok((
                invariant(list.get(index))?.entity,
                invariant(list.id(index))?,
            ))
        })
        .collect::<Result<_, BevyError>>()?;
    for (index, item) in items.iter().enumerate() {
        if list.get(index).is_none_or(|old| old.entity != item.entity) {
            if let Some(id) = ids.get(&item.entity) {
                let from = invariant(list.index_of(*id))?;
                if !list.move_item(from, index) {
                    invariant::<()>(None)?;
                }
            } else {
                list.insert(index, *item)?;
            }
        }
        if list.get(index) != Some(item) {
            *invariant(list.get_mut(index)?)? = *item;
        }
    }
    Ok(())
}

fn invariant<T>(value: Option<T>) -> Result<T, BevyError> {
    value.ok_or_else(|| BevyError::error("WidgetryTree source invariant failed"))
}

fn sync_source(world: &mut World, source: Entity) -> Result<(), BevyError> {
    let result = sync_source_inner(world, source);
    if let Some(mut tree) = world.get_mut::<WidgetryTreeModel>(source) {
        tree.diagnostics.observe(
            result,
            |error| widgetry_error!(?source, %error, "Tree source projection 同步失败"),
            || widgetry_info!(?source, "Tree source projection 恢复正常"),
        )
    } else {
        result
    }
}

fn sync_source_inner(world: &mut World, source: Entity) -> Result<(), BevyError> {
    let Some(tree) = world.get::<WidgetryTreeModel>(source) else {
        return Ok(());
    };
    invariant(world.get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source))?;
    let (items, reachable) = tree.projection(world)?;
    let list = invariant(world.get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source))?;
    let unchanged = list.len() == items.len()
        && items
            .iter()
            .enumerate()
            .all(|(index, item)| list.get(index) == Some(item));
    if !unchanged {
        let mut list =
            invariant(world.get_mut::<WidgetryListModel<WidgetryTreeVisibleItem>>(source))?;
        reconcile_list(&mut list, &items)?;
    }
    invariant(world.get_mut::<WidgetryTreeModel>(source))?.apply_projection(items, &reachable);
    Ok(())
}

fn contains_node(world: &World, source: Entity, node: Entity) -> bool {
    let Some(tree) = world.get::<WidgetryTreeModel>(source) else {
        return false;
    };
    let mut current = node;
    let mut seen = HashSet::new();
    while current != tree.root() && seen.insert(current) {
        if world.get::<WidgetryTreeNode>(current).is_none() {
            return false;
        }
        let Some(parent) = world.get::<ChildOf>(current) else {
            return false;
        };
        current = parent.parent();
    }
    node != tree.root() && current == tree.root() && world.get_entity(current).is_ok()
}

fn validate_target(world: &World, source: Entity, node: Option<Entity>) -> Result<(), BevyError> {
    if world.get::<WidgetryTreeModel>(source).is_none()
        || node.is_some_and(|node| !contains_node(world, source, node))
    {
        widgetry_error!(?source, ?node, "Tree 更新目标缺少 Model 或 node 不可达");
        return Err(BevyError::error(
            "Tree requires a live Model source and reachable node",
        ));
    }
    Ok(())
}

impl WidgetryTreeChildrenState {
    pub fn set_loaded(world: &mut World, node: Entity) -> Result<bool, BevyError> {
        let state = world.get::<Self>(node).copied();
        if world.get::<WidgetryTreeNode>(node).is_none()
            || !matches!(state, Some(Self::Loading | Self::Loaded))
        {
            widgetry_error!(?node, ?state, "Tree loader 完成目标失效或尚未请求");
            return Err(BevyError::error(
                "Tree loader requires a live requested node",
            ));
        }
        if state == Some(Self::Loaded) {
            return Ok(false);
        }
        world.entity_mut(node).insert(Self::Loaded);
        Ok(true)
    }
}

impl WidgetryTreeModel {
    pub fn expand(world: &mut World, source: Entity, node: Entity) -> Result<bool, BevyError> {
        validate_target(world, source, Some(node))?;
        let previous = world.get::<Self>(source).map(|tree| {
            (
                tree.state.clone(),
                tree.visible_items.clone(),
                tree.entity_to_index.clone(),
            )
        });
        let lazy = world.get::<WidgetryTreeChildrenState>(node).copied();
        let has_children = world.get::<Children>(node).is_some_and(|children| {
            children
                .iter()
                .any(|child| world.get::<WidgetryTreeNode>(child).is_some())
        });
        if !has_children && lazy.is_none_or(|state| state == WidgetryTreeChildrenState::Loaded) {
            return Ok(false);
        }
        if !invariant(world.get_mut::<Self>(source))?
            .state
            .expanded
            .insert(node)
        {
            return Ok(false);
        }
        let request = lazy == Some(WidgetryTreeChildrenState::Unknown);
        if let Err(error) = sync_source(world, source) {
            if let Some((state, items, indices)) = previous
                && let Some(mut tree) = world.get_mut::<Self>(source)
            {
                tree.state = state;
                tree.visible_items = items;
                tree.entity_to_index = indices;
            }
            return Err(error);
        }
        if request {
            world
                .entity_mut(node)
                .insert(WidgetryTreeChildrenState::Loading);
        }

        world.trigger(WidgetryTreeEvent {
            entity: source,
            kind: WidgetryTreeEventKind::Expanded(node),
        });
        if request {
            world.trigger(WidgetryTreeEvent {
                entity: source,
                kind: WidgetryTreeEventKind::ChildrenRequested(node),
            });
        }
        Ok(true)
    }

    pub fn collapse(world: &mut World, source: Entity, node: Entity) -> Result<bool, BevyError> {
        validate_target(world, source, Some(node))?;
        let previous = world.get::<Self>(source).map(|tree| {
            (
                tree.state.clone(),
                tree.visible_items.clone(),
                tree.entity_to_index.clone(),
            )
        });
        if !invariant(world.get_mut::<Self>(source))?
            .state
            .expanded
            .remove(&node)
        {
            return Ok(false);
        }
        if let Err(error) = sync_source(world, source) {
            if let Some((state, items, indices)) = previous
                && let Some(mut tree) = world.get_mut::<Self>(source)
            {
                tree.state = state;
                tree.visible_items = items;
                tree.entity_to_index = indices;
            }
            return Err(error);
        }

        world.trigger(WidgetryTreeEvent {
            entity: source,
            kind: WidgetryTreeEventKind::Collapsed(node),
        });
        Ok(true)
    }

    pub fn toggle_expand(
        world: &mut World,
        source: Entity,
        node: Entity,
    ) -> Result<bool, BevyError> {
        if world
            .get::<Self>(source)
            .is_some_and(|tree| tree.state.is_expanded(node))
        {
            Self::collapse(world, source, node)
        } else {
            Self::expand(world, source, node)
        }
    }

    pub fn select(
        world: &mut World,
        source: Entity,
        node: Option<Entity>,
    ) -> Result<bool, BevyError> {
        validate_target(world, source, node)?;
        let mut tree = invariant(world.get_mut::<Self>(source))?;
        if tree.state.selected == node {
            return Ok(false);
        }
        tree.state.selected = node;
        world.trigger(WidgetryTreeEvent {
            entity: source,
            kind: WidgetryTreeEventKind::Selected(node),
        });
        Ok(true)
    }
}
