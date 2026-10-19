use crate::{WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeState, WidgetryTreeView};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use bevy_widgetry_list_view::{WidgetryListSelectionState, WidgetryListView};
use std::{collections::HashSet, sync::Arc};

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[component(immutable)]
pub enum WidgetryTreeChildrenState {
    #[default]
    Unknown,
    Loading,
    Loaded,
}

#[derive(EntityEvent, Clone, Debug)]
pub struct WidgetryTreeEvent {
    pub entity: Entity,
    pub kind: WidgetryTreeEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetryTreeEventKind {
    Expanded(Entity),
    Collapsed(Entity),
    SelectionChanged {
        selected: Arc<HashSet<Entity>>,
        added: Vec<Entity>,
        removed: Vec<Entity>,
    },
    ExpansionChanged {
        expanded: Vec<Entity>,
        collapsed: Vec<Entity>,
    },
    ChildrenRequested(Entity),
}

pub(crate) fn sync_views(world: &mut World) -> Result<(), BevyError> {
    let views = world
        .query_filtered::<Entity, With<WidgetryTreeView>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for view in views {
        if let Err(error) = sync_view(world, view)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn sync_view(world: &mut World, view: Entity) -> Result<(), BevyError> {
    crate::view::validate_source(world, view)?;
    let result = (|| {
        crate::view::internal_list(world, view)?;
        let root = tree_root(world, view)?;
        let previous = WidgetryTreeView::state(world, view)?;
        let (items, reachable) = previous.projection(world, root)?;
        if previous.visible_items != items
            || previous
                .expanded
                .iter()
                .any(|node| !reachable.contains(node))
        {
            let mut next = previous.clone();
            next.apply_projection(items, &reachable);
            set_state(world, view, next)?;
        }
        Ok(())
    })();
    crate::view::observe_projection(world, view, result)
}

fn set_state(world: &mut World, view: Entity, next: WidgetryTreeState) -> Result<(), BevyError> {
    let list = crate::view::internal_list(world, view).map_err(|error| {
        widgetry_error!(?view, %error, "TreeView 状态提交缺少内部列表");
        error
    })?;
    world.entity_mut(view).insert(next);
    // Multiple state commits can share one Bevy tick, so the source tick alone cannot invalidate the list cache.
    world
        .entity_mut(list)
        .remove::<bevy_widgetry_list_view::WidgetryListProjection<Entity>>();
    Ok(())
}

fn invalid(view: Entity, node: Option<Entity>) -> BevyError {
    widgetry_error!(?view, ?node, "TreeView 更新目标失效或 node 不可达");
    BevyError::error("TreeView requires a live view, Model source and reachable node")
}

fn tree_root(world: &World, view: Entity) -> Result<Entity, BevyError> {
    world
        .get::<WidgetryTreeView>(view)
        .and_then(|view| world.get::<WidgetryTreeModel>(view.source()))
        .map(WidgetryTreeModel::root)
        .ok_or_else(|| invalid(view, None))
}

fn validate_target(world: &World, view: Entity, node: Option<Entity>) -> Result<(), BevyError> {
    let source = world
        .get::<WidgetryTreeView>(view)
        .map(WidgetryTreeView::source);
    if source.is_none_or(|source| {
        world.get::<WidgetryTreeModel>(source).is_none()
            || node.is_some_and(|node| !contains_node(world, source, node))
    }) {
        return Err(invalid(view, node));
    }
    crate::view::internal_list(world, view).map_err(|error| {
        widgetry_error!(?view, %error, "TreeView 更新目标内部列表失效");
        error
    })?;
    Ok(())
}

pub(crate) fn contains_node(world: &World, source: Entity, node: Entity) -> bool {
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

impl WidgetryTreeView {
    pub fn state(world: &World, view: Entity) -> Result<&WidgetryTreeState, BevyError> {
        if world.get::<Self>(view).is_none() {
            return Err(invalid(view, None));
        }
        world
            .get::<WidgetryTreeState>(view)
            .ok_or_else(|| invalid(view, None))
    }

    pub fn selection(
        world: &World,
        view: Entity,
    ) -> Result<&WidgetryListSelectionState<Entity>, BevyError> {
        let list = crate::view::internal_list(world, view)?;
        WidgetryListView::<crate::WidgetryTreeVisibleItem, Entity>::selection(world, list)
    }

    pub fn expand(world: &mut World, view: Entity, node: Entity) -> Result<bool, BevyError> {
        validate_target(world, view, Some(node))?;
        let lazy = world.get::<WidgetryTreeChildrenState>(node).copied();
        let has_children = world.get::<Children>(node).is_some_and(|children| {
            children
                .iter()
                .any(|child| world.get::<WidgetryTreeNode>(child).is_some())
        });
        if !has_children && lazy.is_none_or(|state| state == WidgetryTreeChildrenState::Loaded) {
            return Ok(false);
        }
        let mut next = Self::state(world, view)?.clone();
        if !next.expanded.insert(node) {
            return Ok(false);
        }
        let (items, reachable) =
            next.projection(world, tree_root(world, view)?)
                .map_err(|error| {
                    widgetry_error!(?view, ?node, %error, "TreeView 展开状态投影失败");
                    error
                })?;
        next.apply_projection(items, &reachable);
        set_state(world, view, next)?;
        let request = lazy == Some(WidgetryTreeChildrenState::Unknown);
        if request {
            world
                .entity_mut(node)
                .insert(WidgetryTreeChildrenState::Loading);
        }
        world.trigger(WidgetryTreeEvent {
            entity: view,
            kind: WidgetryTreeEventKind::Expanded(node),
        });
        if request {
            world.trigger(WidgetryTreeEvent {
                entity: view,
                kind: WidgetryTreeEventKind::ChildrenRequested(node),
            });
        }
        Ok(true)
    }

    pub fn collapse(world: &mut World, view: Entity, node: Entity) -> Result<bool, BevyError> {
        validate_target(world, view, Some(node))?;
        let mut next = Self::state(world, view)?.clone();
        if !next.expanded.remove(&node) {
            return Ok(false);
        }
        let (items, reachable) =
            next.projection(world, tree_root(world, view)?)
                .map_err(|error| {
                    widgetry_error!(?view, ?node, %error, "TreeView 展开状态投影失败");
                    error
                })?;
        next.apply_projection(items, &reachable);
        set_state(world, view, next)?;
        world.trigger(WidgetryTreeEvent {
            entity: view,
            kind: WidgetryTreeEventKind::Collapsed(node),
        });
        Ok(true)
    }

    pub fn toggle_expand(world: &mut World, view: Entity, node: Entity) -> Result<bool, BevyError> {
        if Self::state(world, view)?.is_expanded(node) {
            Self::collapse(world, view, node)
        } else {
            Self::expand(world, view, node)
        }
    }

    pub fn select(
        world: &mut World,
        view: Entity,
        node: Option<Entity>,
    ) -> Result<bool, BevyError> {
        validate_target(world, view, node)?;
        sync_view(world, view)?;
        let list = crate::view::internal_list(world, view)?;
        let result = if let Some(node) = node {
            WidgetryListView::<crate::WidgetryTreeVisibleItem, Entity>::select_id_in_world(
                world, list, node,
            )
        } else {
            WidgetryListView::<crate::WidgetryTreeVisibleItem, Entity>::clear_selection_in_world(
                world, list,
            )
        };
        world.flush();
        result
    }

    pub fn replace_selection(
        world: &mut World,
        view: Entity,
        nodes: impl IntoIterator<Item = Entity>,
    ) -> Result<bool, BevyError> {
        validate_target(world, view, None)?;
        sync_view(world, view)?;
        let list = crate::view::internal_list(world, view)?;
        let result =
            WidgetryListView::<crate::WidgetryTreeVisibleItem, Entity>::replace_selection_in_world(
                world, list, nodes,
            );
        world.flush();
        result
    }

    pub fn select_visible(world: &mut World, view: Entity) -> Result<bool, BevyError> {
        validate_target(world, view, None)?;
        sync_view(world, view)?;
        let list = crate::view::internal_list(world, view)?;
        let result =
            WidgetryListView::<crate::WidgetryTreeVisibleItem, Entity>::select_all_in_world(
                world, list,
            );
        world.flush();
        result
    }

    pub fn expand_all(world: &mut World, view: Entity) -> Result<bool, BevyError> {
        validate_target(world, view, None)?;
        sync_view(world, view)?;
        let root = tree_root(world, view)?;
        let mut next = Self::state(world, view)?.clone();
        let mut stack = world
            .get::<Children>(root)
            .map(|children| children.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut seen = HashSet::new();
        while let Some(node) = stack.pop() {
            if !seen.insert(node) || world.get::<WidgetryTreeNode>(node).is_none() {
                continue;
            }
            if world
                .get::<WidgetryTreeChildrenState>(node)
                .is_some_and(|state| *state != WidgetryTreeChildrenState::Loaded)
            {
                continue;
            }
            let children = world
                .get::<Children>(node)
                .map(|children| {
                    children
                        .iter()
                        .filter(|child| world.get::<WidgetryTreeNode>(*child).is_some())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if !children.is_empty() {
                next.expanded.insert(node);
            }
            stack.extend(children);
        }
        commit_batch_expansion(world, view, next)
    }

    pub fn collapse_all(world: &mut World, view: Entity) -> Result<bool, BevyError> {
        validate_target(world, view, None)?;
        sync_view(world, view)?;
        let mut next = Self::state(world, view)?.clone();
        next.expanded.clear();
        commit_batch_expansion(world, view, next)
    }

    pub fn queue_replace_selection(
        commands: &mut Commands,
        view: Entity,
        nodes: impl IntoIterator<Item = Entity>,
    ) {
        let nodes = nodes.into_iter().collect::<Vec<_>>();
        commands.queue(move |world: &mut World| {
            Self::replace_selection(world, view, nodes).map(|_| ())
        });
    }
    pub fn queue_select_visible(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| Self::select_visible(world, view).map(|_| ()));
    }
    pub fn queue_expand_all(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| Self::expand_all(world, view).map(|_| ()));
    }
    pub fn queue_collapse_all(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| Self::collapse_all(world, view).map(|_| ()));
    }

    pub fn queue_expand(commands: &mut Commands, view: Entity, node: Entity) {
        commands.queue(move |world: &mut World| Self::expand(world, view, node).map(|_| ()));
    }
    pub fn queue_collapse(commands: &mut Commands, view: Entity, node: Entity) {
        commands.queue(move |world: &mut World| Self::collapse(world, view, node).map(|_| ()));
    }
    pub fn queue_toggle_expand(commands: &mut Commands, view: Entity, node: Entity) {
        commands.queue(move |world: &mut World| Self::toggle_expand(world, view, node).map(|_| ()));
    }
    pub fn queue_select(commands: &mut Commands, view: Entity, node: Option<Entity>) {
        commands.queue(move |world: &mut World| Self::select(world, view, node).map(|_| ()));
    }
}

fn commit_batch_expansion(
    world: &mut World,
    view: Entity,
    mut next: WidgetryTreeState,
) -> Result<bool, BevyError> {
    let previous = WidgetryTreeView::state(world, view)?;
    let expanded = next
        .expanded
        .difference(&previous.expanded)
        .copied()
        .collect::<Vec<_>>();
    let collapsed = previous
        .expanded
        .difference(&next.expanded)
        .copied()
        .collect::<Vec<_>>();
    if expanded.is_empty() && collapsed.is_empty() {
        return Ok(false);
    }
    let (items, reachable) = next
        .projection(world, tree_root(world, view)?)
        .map_err(|error| {
            widgetry_error!(?view, %error, "TreeView 批量展开状态投影失败");
            error
        })?;
    next.apply_projection(items, &reachable);
    set_state(world, view, next)?;
    world.trigger(WidgetryTreeEvent {
        entity: view,
        kind: WidgetryTreeEventKind::ExpansionChanged {
            expanded,
            collapsed,
        },
    });
    Ok(true)
}
