use crate::{WidgetryTreeChildrenState, WidgetryTreeModel, WidgetryTreeView};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
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
    pub fn draggable(world: &World, view: Entity) -> Result<bool, BevyError> {
        let list = crate::view::internal_list(world, view)?;
        world.get::<bevy_widgetry_list_view::WidgetryListView<crate::WidgetryTreeVisibleItem,Entity>>(list)
            .map(|v| v.reorderable()).ok_or_else(|| BevyError::error("TreeView internal list missing"))
    }

    pub fn set_draggable(
        world: &mut World,
        view: Entity,
        enabled: bool,
    ) -> Result<bool, BevyError> {
        if world.get::<Self>(view).is_none() {
            bevy_widgetry_internal_log::widgetry_error!(?view, "TreeView 拖放配置目标无效");
            return Err(BevyError::error(
                "TreeView drag configuration requires a live view",
            ));
        }
        let list = crate::view::internal_list(world,view).map_err(|error| {
            bevy_widgetry_internal_log::widgetry_error!(?view,%error,"TreeView 拖放配置缺少内部列表");
            error
        })?;
        bevy_widgetry_list_view::WidgetryListView::<crate::WidgetryTreeVisibleItem,Entity>::set_reorderable_in_world(world,list,enabled)
    }

    pub fn queue_set_draggable(commands: &mut Commands, view: Entity, enabled: bool) {
        commands
            .queue(move |world: &mut World| Self::set_draggable(world, view, enabled).map(|_| ()));
    }

    pub fn move_node(
        world: &mut World,
        view: Entity,
        movement: WidgetryTreeMove,
    ) -> Result<bool, BevyError> {
        let plan = resolve(world, view, movement).map_err(|reason| {
            bevy_widgetry_internal_log::widgetry_error!(
                ?view,
                ?movement,
                reason,
                "Tree 节点移动被拒绝"
            );
            BevyError::error(format!("Tree node move rejected: {reason}"))
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
        commands.queue(move |world: &mut World| Self::move_node(world, view, movement).map(|_| ()));
    }
}

struct MovePlan {
    old_parent: Entity,
    new_parent: Entity,
    old_index: usize,
    index: usize,
}

fn enabled_path(world: &World, node: Entity, root: Entity) -> bool {
    let mut current = node;
    let mut seen = HashSet::new();
    while seen.insert(current) {
        if world.get::<InteractionDisabled>(current).is_some() {
            return false;
        }
        if current == root {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(current) else {
            return false;
        };
        current = parent.parent();
    }
    false
}

fn resolve(
    world: &World,
    view: Entity,
    movement: WidgetryTreeMove,
) -> Result<MovePlan, &'static str> {
    let source = world
        .get::<WidgetryTreeView>(view)
        .ok_or("invalid view")?
        .source();
    let root = world
        .get::<WidgetryTreeModel>(source)
        .ok_or("invalid source")?
        .root();
    if crate::view::internal_list(world, view).is_err() {
        return Err("missing internal list");
    }
    let moving = movement.moving_entity;
    let target = movement.target_entity;
    if moving == target {
        return Err("self drop");
    }
    if !crate::behavior::contains_node(world, source, moving)
        || !crate::behavior::contains_node(world, source, target)
    {
        return Err("nodes must belong to the same live tree");
    }
    if !enabled_path(world, moving, root) || !enabled_path(world, target, root) {
        return Err("disabled node or parent");
    }
    let old_parent = world
        .get::<ChildOf>(moving)
        .ok_or("missing moving parent")?
        .parent();
    let old_index = world
        .get::<Children>(old_parent)
        .and_then(|c| c.iter().position(|e| e == moving))
        .ok_or("invalid moving hierarchy")?;
    let (new_parent, insertion) = match movement.placement {
        WidgetryTreeDropPlacement::Inside => {
            if world
                .get::<WidgetryTreeChildrenState>(target)
                .is_some_and(|s| *s != WidgetryTreeChildrenState::Loaded)
            {
                return Err("inside requires loaded children");
            }
            (
                target,
                world.get::<Children>(target).map_or(0, Children::len),
            )
        }
        placement => {
            let parent = world
                .get::<ChildOf>(target)
                .ok_or("missing target parent")?
                .parent();
            let index = world
                .get::<Children>(parent)
                .and_then(|c| c.iter().position(|e| e == target))
                .ok_or("invalid target hierarchy")?;
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
            return Err("cyclic parent chain");
        }
        if current == root {
            break;
        }
        current = world
            .get::<ChildOf>(current)
            .ok_or("invalid destination hierarchy")?
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

impl bevy_widgetry_list_view::WidgetryListDragSource<Entity> for crate::source::TreeSource {
    fn allows_inside() -> bool {
        true
    }

    fn can_drop(
        world: &World,
        source: Entity,
        movement: bevy_widgetry_list_view::WidgetryListMove<Entity>,
    ) -> bool {
        resolve(world, source, movement.into()).is_ok()
    }

    fn commit(
        world: &mut World,
        source: Entity,
        movement: bevy_widgetry_list_view::WidgetryListMove<Entity>,
    ) -> Result<bool, BevyError> {
        // Revalidate at the write boundary, even after a successful hover check.
        WidgetryTreeView::move_node(world, source, movement.into())
    }
}

impl From<bevy_widgetry_list_view::WidgetryListMove<Entity>> for WidgetryTreeMove {
    fn from(value: bevy_widgetry_list_view::WidgetryListMove<Entity>) -> Self {
        use bevy_widgetry_list_view::WidgetryListDropPlacement as P;
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
