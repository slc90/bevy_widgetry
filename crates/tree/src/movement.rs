use crate::{TreeError, TreeFailure, TreeMoveReason, TreeMoveRejection};
use crate::{WidgetryTreeChildrenState, WidgetryTreeModel, WidgetryTreeView};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_diagnostics::ErrorFrame;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryTreeDropPlacement {
    Before,
    Inside,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryTreeMove {
    pub moving_entity: Entity,
    pub target_entity: Entity,
    pub placement: WidgetryTreeDropPlacement,
}

impl crate::WidgetryTreeView {
    pub fn draggable(world: &World, view: Entity) -> Result<bool, TreeFailure> {
        Ok(drag_list(world, view)?.1.draggable())
    }
    pub fn set_draggable(
        world: &mut World,
        view: Entity,
        enabled: bool,
    ) -> Result<bool, TreeFailure> {
        let (list, _) = drag_list(world, view)?;
        bevy_widgetry_list_view::WidgetryListView::<crate::WidgetryTreeVisibleItem,Entity>::set_draggable_in_world(world,list,enabled)
            .map_err(|error|error.map_kind(|source|TreeError::List{source:Box::new(source)})
                .with_context(ErrorFrame::new("tree.set_draggable").field("view",view).field("list",list).field("enabled",enabled)))
    }
    pub fn queue_set_draggable(commands: &mut Commands, view: Entity, enabled: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_draggable(world, view, enabled)
                .map(|_| ())
                .map_err(TreeFailure::into_bevy_error)
        });
    }

    pub fn move_node(
        world: &mut World,
        view: Entity,
        movement: WidgetryTreeMove,
    ) -> Result<bool, TreeFailure> {
        let plan = resolve(world, view, movement).map_err(|rejection| {
            failure(
                TreeError::Move {
                    rejection: Box::new(rejection),
                },
                "tree.move_node",
            )
        })?;
        if plan.old_parent == plan.new_parent && plan.old_index == plan.index {
            return Ok(false);
        }
        world
            .entity_mut(plan.new_parent)
            .insert_child(plan.index, movement.moving_entity);
        world.trigger(crate::WidgetryTreeEvent {
            entity: view,
            kind: crate::WidgetryTreeEventKind::NodeMoved {
                movement,
                old_parent: plan.old_parent,
                new_parent: plan.new_parent,
            },
        });
        Ok(true)
    }

    pub fn queue_move_node(commands: &mut Commands, view: Entity, movement: WidgetryTreeMove) {
        commands.queue(move |world: &mut World| {
            Self::move_node(world, view, movement)
                .map(|_| ())
                .map_err(TreeFailure::into_bevy_error)
        });
    }
}

struct MovePlan {
    old_parent: Entity,
    new_parent: Entity,
    old_index: usize,
    index: usize,
}

#[track_caller]
fn failure(kind: TreeError, operation: &'static str) -> TreeFailure {
    let frame = ErrorFrame::new(operation).field("failure", format_args!("{kind:?}"));
    TreeFailure::new(kind, frame)
}
type TreeList = bevy_widgetry_list_view::WidgetryListView<crate::WidgetryTreeVisibleItem, Entity>;
fn internal_list(world: &World, view: Entity) -> Option<(Entity, &TreeList)> {
    world
        .get::<Children>(view)?
        .iter()
        .find_map(|entity| world.get::<TreeList>(entity).map(|list| (entity, list)))
}
fn drag_list(world: &World, view: Entity) -> Result<(Entity, &TreeList), TreeFailure> {
    let source = world
        .get::<WidgetryTreeView>(view)
        .ok_or_else(|| failure(TreeError::InvalidView { view }, "tree.drag_config"))?
        .source();
    internal_list(world, view)
        .ok_or_else(|| failure(TreeError::MissingList { view, source }, "tree.drag_config"))
}
fn enabled_path(world: &World, node: Entity, root: Entity) -> Result<(), TreeMoveReason> {
    let mut current = node;
    let mut seen = HashSet::new();
    while seen.insert(current) {
        if world.get::<InteractionDisabled>(current).is_some() {
            return Err(TreeMoveReason::Disabled { entity: current });
        }
        if current == root {
            return Ok(());
        }
        current = world
            .get::<ChildOf>(current)
            .ok_or(TreeMoveReason::MissingParent { entity: current })?
            .parent();
    }
    Err(TreeMoveReason::ParentCycle { entity: current })
}
fn membership_rejection(world: &World, root: Entity, node: Entity) -> TreeMoveReason {
    // Only diagnose a failed membership check; healthy previews keep their existing traversal work.
    let mut current = node;
    let mut seen = HashSet::new();
    while current != root && seen.insert(current) {
        if world.get::<crate::WidgetryTreeNode>(current).is_none() {
            return TreeMoveReason::NotInTree { node, at: current };
        }
        let Some(parent) = world.get::<ChildOf>(current) else {
            return TreeMoveReason::MissingParent { entity: current };
        };
        current = parent.parent();
    }
    if current != root {
        TreeMoveReason::ParentCycle { entity: current }
    } else {
        TreeMoveReason::NotInTree { node, at: root }
    }
}
fn child_index(world: &World, parent: Entity, node: Entity) -> Result<usize, TreeMoveReason> {
    world
        .get::<Children>(parent)
        .and_then(|children| children.iter().position(|entity| entity == node))
        .ok_or(TreeMoveReason::MissingIndex { parent, node })
}
fn resolve(
    world: &World,
    view: Entity,
    movement: WidgetryTreeMove,
) -> Result<MovePlan, TreeMoveRejection> {
    let source = world
        .get::<WidgetryTreeView>(view)
        .map(WidgetryTreeView::source);
    let root = source
        .and_then(|source| world.get::<WidgetryTreeModel>(source))
        .map(WidgetryTreeModel::root);
    let moving = movement.moving_entity;
    let target = movement.target_entity;
    let reject = |reason| TreeMoveRejection {
        view,
        source,
        model_root: root,
        movement,
        old_parent: world.get::<ChildOf>(moving).map(ChildOf::parent),
        new_parent: match movement.placement {
            WidgetryTreeDropPlacement::Inside => Some(target),
            _ => world.get::<ChildOf>(target).map(ChildOf::parent),
        },
        reason,
    };
    let source = source.ok_or_else(|| reject(TreeMoveReason::InvalidView))?;
    let root = root.ok_or_else(|| reject(TreeMoveReason::InvalidSource))?;
    if internal_list(world, view).is_none() {
        return Err(reject(TreeMoveReason::MissingList));
    }
    if moving == target {
        return Err(reject(TreeMoveReason::SelfMove));
    }
    for node in [moving, target] {
        if !crate::behavior::contains_node(world, source, node) {
            return Err(reject(membership_rejection(world, root, node)));
        }
    }
    enabled_path(world, moving, root).map_err(reject)?;
    enabled_path(world, target, root).map_err(reject)?;
    let old_parent = world
        .get::<ChildOf>(moving)
        .ok_or_else(|| reject(TreeMoveReason::MissingParent { entity: moving }))?
        .parent();
    let old_index = child_index(world, old_parent, moving).map_err(reject)?;
    let (new_parent, insertion) = match movement.placement {
        WidgetryTreeDropPlacement::Inside => {
            if let Some(state) = world
                .get::<WidgetryTreeChildrenState>(target)
                .copied()
                .filter(|state| *state != WidgetryTreeChildrenState::Loaded)
            {
                return Err(reject(TreeMoveReason::UnloadedChildren { target, state }));
            }
            (
                target,
                world.get::<Children>(target).map_or(0, Children::len),
            )
        }
        placement => {
            let parent = world
                .get::<ChildOf>(target)
                .ok_or_else(|| reject(TreeMoveReason::MissingParent { entity: target }))?
                .parent();
            let index = child_index(world, parent, target).map_err(reject)?;
            (
                parent,
                index + usize::from(placement == WidgetryTreeDropPlacement::After),
            )
        }
    };
    let mut current = new_parent;
    let mut seen = HashSet::new();
    loop {
        if current == moving || !seen.insert(current) {
            return Err(reject(TreeMoveReason::ParentCycle { entity: current }));
        }
        if current == root {
            break;
        }
        current = world
            .get::<ChildOf>(current)
            .ok_or_else(|| reject(TreeMoveReason::DestinationDetached { entity: current }))?
            .parent();
    }
    let index = insertion - usize::from(old_parent == new_parent && old_index < insertion);
    Ok(MovePlan {
        old_parent,
        new_parent,
        old_index,
        index,
    })
}

impl bevy_widgetry_list_view::internal::TreeDragSource<Entity> for crate::source::TreeSource {
    fn allows_inside() -> bool {
        true
    }

    fn can_drop(
        world: &World,
        source: Entity,
        movement: bevy_widgetry_list_view::internal::WidgetryListMove<Entity>,
    ) -> bool {
        resolve(world, source, movement.into()).is_ok()
    }

    fn commit(
        world: &mut World,
        source: Entity,
        movement: bevy_widgetry_list_view::internal::WidgetryListMove<Entity>,
    ) -> Result<bool, BevyError> {
        // Revalidate at the write boundary, even after a successful hover check.
        WidgetryTreeView::move_node(world, source, movement.into())
            .map_err(TreeFailure::into_bevy_error)
    }
}

impl From<bevy_widgetry_list_view::internal::WidgetryListMove<Entity>> for WidgetryTreeMove {
    fn from(value: bevy_widgetry_list_view::internal::WidgetryListMove<Entity>) -> Self {
        use bevy_widgetry_list_view::internal::WidgetryListDropPlacement as P;
        Self {
            moving_entity: value.moving,
            target_entity: value.target,
            placement: match value.placement {
                P::Before => WidgetryTreeDropPlacement::Before,
                P::Inside => WidgetryTreeDropPlacement::Inside,
                P::After => WidgetryTreeDropPlacement::After,
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_list_view::internal::{
        TreeDragSource, WidgetryListDropPlacement, WidgetryListMove,
    };
    #[test]
    fn preview_rejections_and_legal_noop_do_not_create_diagnostics() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(crate::WidgetryTreePlugin);
        let root = app.world_mut().spawn_empty().id();
        let a = app
            .world_mut()
            .spawn((crate::WidgetryTreeNode, ChildOf(root)))
            .id();
        let b = app
            .world_mut()
            .spawn((
                crate::WidgetryTreeNode,
                ChildOf(root),
                WidgetryTreeChildrenState::Unknown,
            ))
            .id();
        let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
        let view = bevy_widgetry_core::scene::spawn_scene(
            app.world_mut(),
            bsn! {@WidgetryTreeView{@source:source}},
        )
        .unwrap();
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        logs.run(|| {
            for (moving, target) in [(a, a), (a, b)] {
                assert!(!crate::source::TreeSource::can_drop(
                    app.world(),
                    view,
                    WidgetryListMove {
                        moving,
                        target,
                        placement: WidgetryListDropPlacement::Inside
                    }
                ));
            }
            assert!(
                !WidgetryTreeView::move_node(
                    app.world_mut(),
                    view,
                    WidgetryTreeMove {
                        moving_entity: a,
                        target_entity: b,
                        placement: WidgetryTreeDropPlacement::Before
                    }
                )
                .unwrap()
            );
        });
        assert!(logs.records().is_empty());
    }
    #[test]
    fn missing_child_index_preserves_parent_and_node() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let node = world.spawn(crate::WidgetryTreeNode).id();
        assert_eq!(
            child_index(&world, parent, node),
            Err(TreeMoveReason::MissingIndex { parent, node })
        );
        world.entity_mut(node).insert(ChildOf(parent));
        assert_eq!(child_index(&world, parent, node), Ok(0));
    }
}
