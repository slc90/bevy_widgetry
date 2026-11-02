use crate::{
    TreeError, TreeViewFailure, TreeViewFault,
    error::{failure, view_failure},
};
use crate::{WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeState, WidgetryTreeView};
use bevy::prelude::*;
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
    NodeMoved {
        movement: crate::WidgetryTreeMove,
        old_parent: Entity,
        new_parent: Entity,
    },
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
    let mut errors = Vec::new();
    for view in views {
        let result = crate::view::validate_source(world, view).and_then(|()| {
            let result = sync_view_fault(world, view);
            crate::view::observe_projection(world, view, result)
        });
        if let Err(error) = result {
            errors.push(error.into_bevy_error());
        }
    }
    crate::error::dispatch(world, errors)
}
fn sync_view_fault(world: &mut World, view: Entity) -> Result<(), TreeViewFault> {
    let list = crate::view::internal_list_entity(world, view)
        .ok_or_else(|| part(view, crate::TreeViewPart::List))?;
    let root = world
        .get::<WidgetryTreeView>(view)
        .and_then(|view| world.get::<WidgetryTreeModel>(view.source()))
        .map(WidgetryTreeModel::root)
        .ok_or_else(|| target_fault(world, view, None))?;
    let previous = world
        .get::<WidgetryTreeState>(view)
        .ok_or_else(|| part(view, crate::TreeViewPart::State))?;
    let (items, reachable) = previous.projection(world, root)?;
    if previous.visible_items != items
        || previous
            .expanded
            .iter()
            .any(|node| !reachable.contains(node))
    {
        let mut next = previous.clone();
        next.apply_projection(items, &reachable);
        world.entity_mut(view).insert(next);
        world
            .entity_mut(list)
            .remove::<bevy_widgetry_list_view::WidgetryListProjection<Entity>>();
    }
    Ok(())
}
pub(crate) fn sync_view(world: &mut World, view: Entity) -> Result<(), TreeViewFailure> {
    sync_view_fault(world, view).map_err(|fault| {
        crate::error::projection_failure(world, view, fault, crate::error::ProjectionRequest::Sync)
    })
}
fn part(view: Entity, part: crate::TreeViewPart) -> TreeViewFault {
    TreeViewFault::Part {
        view: Some(view),
        node: None,
        entity: view,
        part,
    }
}
fn set_state(
    world: &mut World,
    view: Entity,
    next: WidgetryTreeState,
) -> Result<(), TreeViewFailure> {
    let list = crate::view::internal_list(world, view)?;
    world.entity_mut(view).insert(next);
    world
        .entity_mut(list)
        .remove::<bevy_widgetry_list_view::WidgetryListProjection<Entity>>();
    Ok(())
}
fn target_fault(world: &World, view: Entity, node: Option<Entity>) -> TreeViewFault {
    TreeViewFault::Target {
        view,
        source: world
            .get::<WidgetryTreeView>(view)
            .map(WidgetryTreeView::source),
        node,
    }
}
fn invalid(world: &World, view: Entity, node: Option<Entity>) -> TreeViewFailure {
    view_failure(target_fault(world, view, node), "tree.target")
}
fn tree_root(world: &World, view: Entity) -> Result<Entity, TreeViewFailure> {
    world
        .get::<WidgetryTreeView>(view)
        .and_then(|view| world.get::<WidgetryTreeModel>(view.source()))
        .map(WidgetryTreeModel::root)
        .ok_or_else(|| invalid(world, view, None))
}
fn validate_target(
    world: &World,
    view: Entity,
    node: Option<Entity>,
) -> Result<(), TreeViewFailure> {
    let source = world
        .get::<WidgetryTreeView>(view)
        .map(WidgetryTreeView::source);
    if source.is_none_or(|source| {
        world.get::<WidgetryTreeModel>(source).is_none()
            || node.is_some_and(|node| !contains_node(world, source, node))
    }) {
        return Err(invalid(world, view, node));
    }
    crate::view::internal_list(world, view)?;
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
    pub fn set_loaded(world: &mut World, node: Entity) -> Result<bool, TreeViewFailure> {
        let state = world.get::<Self>(node).copied();
        if world.get::<WidgetryTreeNode>(node).is_none()
            || !matches!(state, Some(Self::Loading | Self::Loaded))
        {
            return Err(TreeViewFailure::new(failure(
                TreeError::Loader { node, state },
                "tree.set_loaded",
            )));
        }
        if state == Some(Self::Loaded) {
            return Ok(false);
        }
        world.entity_mut(node).insert(Self::Loaded);
        Ok(true)
    }
}

impl WidgetryTreeView {
    pub fn state(world: &World, view: Entity) -> Result<&WidgetryTreeState, TreeViewFailure> {
        if world.get::<Self>(view).is_none() {
            return Err(invalid(world, view, None));
        }
        world
            .get::<WidgetryTreeState>(view)
            .ok_or_else(|| view_failure(part(view, crate::TreeViewPart::State), "tree.state"))
    }

    pub fn selection(
        world: &World,
        view: Entity,
    ) -> Result<&WidgetryListSelectionState<Entity>, TreeViewFailure> {
        let list = crate::view::internal_list(world, view)?;
        WidgetryListView::<crate::WidgetryTreeVisibleItem, Entity>::selection(world, list)
            .map_err(|error| crate::error::list(error, world, view, None, list, "tree.selection"))
    }

    pub fn expand(world: &mut World, view: Entity, node: Entity) -> Result<bool, TreeViewFailure> {
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
                .map_err(|fault| {
                    crate::error::projection_failure(
                        world,
                        view,
                        fault,
                        crate::error::ProjectionRequest::Expand(node),
                    )
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

    pub fn collapse(
        world: &mut World,
        view: Entity,
        node: Entity,
    ) -> Result<bool, TreeViewFailure> {
        validate_target(world, view, Some(node))?;
        let mut next = Self::state(world, view)?.clone();
        if !next.expanded.remove(&node) {
            return Ok(false);
        }
        let (items, reachable) =
            next.projection(world, tree_root(world, view)?)
                .map_err(|fault| {
                    crate::error::projection_failure(
                        world,
                        view,
                        fault,
                        crate::error::ProjectionRequest::Collapse(node),
                    )
                })?;
        next.apply_projection(items, &reachable);
        set_state(world, view, next)?;
        world.trigger(WidgetryTreeEvent {
            entity: view,
            kind: WidgetryTreeEventKind::Collapsed(node),
        });
        Ok(true)
    }

    pub fn toggle_expand(
        world: &mut World,
        view: Entity,
        node: Entity,
    ) -> Result<bool, TreeViewFailure> {
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
    ) -> Result<bool, TreeViewFailure> {
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
        result.map_err(|error| crate::error::list(error, world, view, node, list, "tree.select"))
    }

    pub fn replace_selection(
        world: &mut World,
        view: Entity,
        nodes: impl IntoIterator<Item = Entity>,
    ) -> Result<bool, TreeViewFailure> {
        validate_target(world, view, None)?;
        sync_view(world, view)?;
        let list = crate::view::internal_list(world, view)?;
        let result =
            WidgetryListView::<crate::WidgetryTreeVisibleItem, Entity>::replace_selection_in_world(
                world, list, nodes,
            );
        world.flush();
        result.map_err(|error| crate::error::list(error, world, view, None, list, "tree.selection"))
    }

    pub fn select_visible(world: &mut World, view: Entity) -> Result<bool, TreeViewFailure> {
        validate_target(world, view, None)?;
        sync_view(world, view)?;
        let list = crate::view::internal_list(world, view)?;
        let result =
            WidgetryListView::<crate::WidgetryTreeVisibleItem, Entity>::select_all_in_world(
                world, list,
            );
        world.flush();
        result.map_err(|error| crate::error::list(error, world, view, None, list, "tree.selection"))
    }

    pub fn expand_all(world: &mut World, view: Entity) -> Result<bool, TreeViewFailure> {
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
        commit_batch_expansion(
            world,
            view,
            next,
            crate::error::ProjectionRequest::ExpandAll,
        )
    }

    pub fn collapse_all(world: &mut World, view: Entity) -> Result<bool, TreeViewFailure> {
        validate_target(world, view, None)?;
        sync_view(world, view)?;
        let mut next = Self::state(world, view)?.clone();
        next.expanded.clear();
        commit_batch_expansion(
            world,
            view,
            next,
            crate::error::ProjectionRequest::CollapseAll,
        )
    }

    pub fn queue_replace_selection(
        commands: &mut Commands,
        view: Entity,
        nodes: impl IntoIterator<Item = Entity>,
    ) {
        let nodes = nodes.into_iter().collect::<Vec<_>>();
        commands.queue(move |world: &mut World| {
            Self::replace_selection(world, view, nodes)
                .map(|_| ())
                .map_err(TreeViewFailure::into_bevy_error)
        });
    }
    pub fn queue_select_visible(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| {
            Self::select_visible(world, view)
                .map(|_| ())
                .map_err(TreeViewFailure::into_bevy_error)
        });
    }
    pub fn queue_expand_all(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| {
            Self::expand_all(world, view)
                .map(|_| ())
                .map_err(TreeViewFailure::into_bevy_error)
        });
    }
    pub fn queue_collapse_all(commands: &mut Commands, view: Entity) {
        commands.queue(move |world: &mut World| {
            Self::collapse_all(world, view)
                .map(|_| ())
                .map_err(TreeViewFailure::into_bevy_error)
        });
    }

    pub fn queue_expand(commands: &mut Commands, view: Entity, node: Entity) {
        commands.queue(move |world: &mut World| {
            Self::expand(world, view, node)
                .map(|_| ())
                .map_err(TreeViewFailure::into_bevy_error)
        });
    }
    pub fn queue_collapse(commands: &mut Commands, view: Entity, node: Entity) {
        commands.queue(move |world: &mut World| {
            Self::collapse(world, view, node)
                .map(|_| ())
                .map_err(TreeViewFailure::into_bevy_error)
        });
    }
    pub fn queue_toggle_expand(commands: &mut Commands, view: Entity, node: Entity) {
        commands.queue(move |world: &mut World| {
            Self::toggle_expand(world, view, node)
                .map(|_| ())
                .map_err(TreeViewFailure::into_bevy_error)
        });
    }
    pub fn queue_select(commands: &mut Commands, view: Entity, node: Option<Entity>) {
        commands.queue(move |world: &mut World| {
            Self::select(world, view, node)
                .map(|_| ())
                .map_err(TreeViewFailure::into_bevy_error)
        });
    }
}

fn commit_batch_expansion(
    world: &mut World,
    view: Entity,
    mut next: WidgetryTreeState,
    request: crate::error::ProjectionRequest,
) -> Result<bool, TreeViewFailure> {
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
        .map_err(|fault| crate::error::projection_failure(world, view, fault, request))?;
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

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    #[test]
    fn actual_depth_overflow_keeps_view_source_and_expand_request_without_committing() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut app = bevy_widgetry_test_utils::scene_app();
        let world = app.world_mut();
        let root = world.spawn_empty().id();
        let mut parent = root;
        let mut nodes = Vec::new();
        for _ in 0..=usize::from(u16::MAX) + 1 {
            parent = world.spawn((WidgetryTreeNode, ChildOf(parent))).id();
            nodes.push(parent);
        }
        let source = world.spawn(WidgetryTreeModel::new(root)).id();
        app.add_plugins(crate::WidgetryTreePlugin);
        let world = app.world_mut();
        let mut views = Vec::new();
        for _ in 0..2 {
            views.push(
                bevy_widgetry_core::scene::spawn_scene(
                    world,
                    bsn! {@WidgetryTreeView{@source:source}},
                )
                .unwrap(),
            );
        }
        let mut state = WidgetryTreeState::default();
        state
            .expanded
            .extend(nodes.iter().take(usize::from(u16::MAX)).copied());
        world.entity_mut(views[0]).insert(state);
        let target = nodes[usize::from(u16::MAX)];
        let error = WidgetryTreeView::expand(world, views[0], target).unwrap_err();
        let assert_context = |error: &TreeViewFailure, view: Entity, request: &str| {
            let diagnostic = error.diagnostics().next().unwrap();
            assert!(
                matches!(diagnostic.source().unwrap().downcast_ref::<TreeError>(),Some(TreeError::View{fault:TreeViewFault::Depth{root:actual,node,depth}}) if *actual==root&&*node==nodes[usize::from(u16::MAX)+1]&&*depth==usize::from(u16::MAX)+1)
            );
            let fields = diagnostic
                .contexts()
                .iter()
                .flat_map(|frame| frame.fields())
                .collect::<Vec<_>>();
            for (key, value) in [
                ("view", view.to_string()),
                ("source", format!("{source:?}")),
                ("request", request.to_owned()),
            ] {
                assert!(
                    fields
                        .iter()
                        .any(|(name, actual)| name == key && actual.contains(&value)),
                    "{fields:?}"
                );
            }
        };
        assert_context(&error, views[0], "Expand(");
        assert!(
            !WidgetryTreeView::state(world, views[0])
                .unwrap()
                .is_expanded(target)
        );
        assert_context(
            &WidgetryTreeView::expand_all(world, views[0]).unwrap_err(),
            views[0],
            "ExpandAll",
        );
        let mut state = WidgetryTreeState::default();
        state.expanded.extend(nodes.iter().copied());
        world.entity_mut(views[1]).insert(state);
        assert_context(&sync_view(world, views[1]).unwrap_err(), views[1], "Sync");
        let result = sync_view_fault(world, views[1]);
        let error = crate::view::observe_projection(world, views[1], result).unwrap_err();
        assert_context(&error, views[1], "Sync");
    }
}
