use crate::ui::WidgetryUiSystems;
use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::input_focus::InputFocusSystems;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, InteractionDisabled, UiSystems};
use bevy::ui_widgets::ScrollArea;

#[derive(Component, Reflect, Clone, Copy, Default, PartialEq, Eq, Debug)]
#[component(immutable)]
#[reflect(Component)]
pub struct WidgetryEffectiveDisabled(bool);

impl WidgetryEffectiveDisabled {
    pub fn is_disabled(&self) -> bool {
        self.0
    }
}

#[derive(Clone, Copy, Default)]
struct LocalDisabled {
    explicit: bool,
    intrinsic: bool,
    effective: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Projection {
    Insert,
    Remove,
}

#[derive(Resource, Default)]
struct DisabledRuntime {
    local: EntityHashMap<LocalDisabled>,
    dirty: EntityHashSet,
    queued: bool,
    reconciling: bool,
    projection: Option<(Entity, Projection)>,
    #[cfg(test)]
    visits: usize,
}

#[derive(Component)]
struct SuspendedWheel;

pub(crate) fn install(app: &mut App) {
    app.init_resource::<DisabledRuntime>()
        .register_type::<WidgetryEffectiveDisabled>()
        .add_observer(on_insert)
        .add_observer(on_remove)
        .add_observer(on_ui_added)
        .add_observer(on_ui_removed)
        .add_observer(on_parent_inserted)
        .add_observer(on_parent_removed)
        .add_observer(on_wheel_added)
        .add_observer(on_despawn)
        .configure_sets(
            PreUpdate,
            WidgetryUiSystems::Disabled
                .before(PickingSystems::ProcessInput)
                .before(InputFocusSystems::Dispatch),
        )
        .configure_sets(
            PostUpdate,
            WidgetryUiSystems::Disabled
                .after(WidgetryUiSystems::Materialize)
                .before(UiSystems::Propagate)
                .before(bevy::text::EditableTextSystems),
        )
        .add_systems(PreUpdate, reconcile.in_set(WidgetryUiSystems::Disabled))
        .add_systems(PostUpdate, reconcile.in_set(WidgetryUiSystems::Disabled));
    let world = app.world_mut();
    let nodes = world
        .query_filtered::<Entity, With<ComputedNode>>()
        .iter(world)
        .collect::<Vec<_>>();
    world.resource_mut::<DisabledRuntime>().dirty.extend(nodes);
    reconcile(world);
}

pub fn set_intrinsic_disabled(world: &mut World, entity: Entity, disabled: bool) {
    if !world.entities().contains(entity) {
        return;
    }
    let explicit = world.get::<InteractionDisabled>(entity).is_some();
    let mut runtime = world.resource_mut::<DisabledRuntime>();
    let local = runtime.local.entry(entity).or_insert(LocalDisabled {
        explicit,
        ..default()
    });
    if local.intrinsic == disabled {
        return;
    }
    local.intrinsic = disabled;
    runtime.dirty.insert(entity);
    if !runtime.queued {
        runtime.queued = true;
        world.commands().queue(reconcile);
    }
    world.flush();
}

pub fn queue_intrinsic_disabled(commands: &mut Commands, entity: Entity, disabled: bool) {
    commands.queue(move |world: &mut World| set_intrinsic_disabled(world, entity, disabled));
}

fn invalidate(entity: Entity, runtime: &mut DisabledRuntime, commands: &mut Commands) {
    runtime.dirty.insert(entity);
    if !runtime.queued {
        runtime.queued = true;
        commands.queue(reconcile);
    }
}

fn request(
    entity: Entity,
    projection: Projection,
    runtime: &mut DisabledRuntime,
    commands: &mut Commands,
) {
    if runtime.projection == Some((entity, projection)) {
        return;
    }
    runtime.local.entry(entity).or_default().explicit = projection == Projection::Insert;
    invalidate(entity, runtime, commands);
}

fn on_insert(
    event: On<Insert<InteractionDisabled>>,
    mut runtime: ResMut<DisabledRuntime>,
    mut commands: Commands,
) {
    request(
        event.entity,
        Projection::Insert,
        &mut runtime,
        &mut commands,
    );
}

fn on_remove(
    event: On<Remove<InteractionDisabled>>,
    mut runtime: ResMut<DisabledRuntime>,
    mut commands: Commands,
) {
    request(
        event.entity,
        Projection::Remove,
        &mut runtime,
        &mut commands,
    );
}

fn on_ui_added(
    event: On<Add<ComputedNode>>,
    mut runtime: ResMut<DisabledRuntime>,
    mut commands: Commands,
) {
    invalidate(event.entity, &mut runtime, &mut commands);
}

fn on_ui_removed(
    event: On<Remove<ComputedNode>>,
    mut runtime: ResMut<DisabledRuntime>,
    mut commands: Commands,
) {
    invalidate(event.entity, &mut runtime, &mut commands);
}

fn on_parent_inserted(
    event: On<Insert<ChildOf>>,
    nodes: Query<(), With<ComputedNode>>,
    mut runtime: ResMut<DisabledRuntime>,
    mut commands: Commands,
) {
    if nodes.contains(event.entity) || runtime.local.contains_key(&event.entity) {
        invalidate(event.entity, &mut runtime, &mut commands);
    }
}

fn on_parent_removed(
    event: On<Remove<ChildOf>>,
    nodes: Query<(), With<ComputedNode>>,
    mut runtime: ResMut<DisabledRuntime>,
    mut commands: Commands,
) {
    if nodes.contains(event.entity) || runtime.local.contains_key(&event.entity) {
        invalidate(event.entity, &mut runtime, &mut commands);
    }
}

// 官方 ScrollArea 不检查 InteractionDisabled。
// 仅暂停 wheel marker，保留 ScrollPosition 和 layout，避免把 disabled 误当作禁止程序滚动。
fn on_wheel_added(
    event: On<Add<ScrollArea>>,
    nodes: Query<&WidgetryEffectiveDisabled, With<ComputedNode>>,
    mut runtime: ResMut<DisabledRuntime>,
    mut commands: Commands,
) {
    if nodes
        .get(event.entity)
        .is_ok_and(|state| state.is_disabled())
    {
        invalidate(event.entity, &mut runtime, &mut commands);
    }
}

fn on_despawn(
    event: On<Despawn<(ComputedNode, ChildOf, Children, InteractionDisabled)>>,
    mut runtime: ResMut<DisabledRuntime>,
) {
    runtime.local.remove(&event.entity);
}

fn local(world: &mut World, entity: Entity) -> LocalDisabled {
    let explicit = world.get::<InteractionDisabled>(entity).is_some();
    *world
        .resource_mut::<DisabledRuntime>()
        .local
        .entry(entity)
        .or_insert(LocalDisabled {
            explicit,
            ..default()
        })
}

fn ancestor_disabled(world: &mut World, entity: Entity) -> bool {
    let mut path = Vec::new();
    let mut parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
    let mut effective = false;
    while let Some(entity) = parent {
        if let Some(local) = world.resource::<DisabledRuntime>().local.get(&entity) {
            effective = local.effective;
            break;
        }
        path.push(entity);
        parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    for entity in path.into_iter().rev() {
        let state = local(world, entity);
        effective |= state.explicit || state.intrinsic;
        if let Some(state) = world
            .resource_mut::<DisabledRuntime>()
            .local
            .get_mut(&entity)
        {
            state.effective = effective;
        }
    }
    effective
}

fn dirty_roots(world: &World, dirty: &EntityHashSet) -> Vec<Entity> {
    if dirty.len() == 1 {
        return dirty
            .iter()
            .copied()
            .filter(|entity| world.entities().contains(*entity))
            .collect();
    }
    let mut covered = EntityHashMap::<bool>::default();
    let mut roots = Vec::new();
    for &entity in dirty {
        if !world.entities().contains(entity) {
            continue;
        }
        let mut path = Vec::new();
        let mut parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
        let mut inherited = false;
        while let Some(ancestor) = parent {
            if dirty.contains(&ancestor) {
                inherited = true;
                break;
            }
            if let Some(&known) = covered.get(&ancestor) {
                inherited = known;
                break;
            }
            path.push(ancestor);
            parent = world.get::<ChildOf>(ancestor).map(ChildOf::parent);
        }
        covered.extend(path.into_iter().map(|ancestor| (ancestor, inherited)));
        if !inherited {
            roots.push(entity);
        }
    }
    roots
}

fn project(world: &mut World, entity: Entity, disabled: bool) {
    if !world.entities().contains(entity) {
        return;
    }
    if world.get::<ComputedNode>(entity).is_some()
        && world
            .get::<WidgetryEffectiveDisabled>(entity)
            .is_none_or(|state| state.0 != disabled)
    {
        world
            .entity_mut(entity)
            .insert(WidgetryEffectiveDisabled(disabled));
    } else if world.get::<ComputedNode>(entity).is_none()
        && world.get::<WidgetryEffectiveDisabled>(entity).is_some()
    {
        world
            .entity_mut(entity)
            .remove::<WidgetryEffectiveDisabled>();
    }
    // Insert/Remove 可以同步运行消费者的 despawn command，后续结构写入必须重新检查存活。
    if !world.entities().contains(entity) {
        return;
    }
    if disabled && world.get::<ScrollArea>(entity).is_some() {
        world.entity_mut(entity).remove::<ScrollArea>();
        if !world.entities().contains(entity) {
            return;
        }
        world.entity_mut(entity).insert(SuspendedWheel);
    } else if !disabled && world.entity_mut(entity).take::<SuspendedWheel>().is_some() {
        if !world.entities().contains(entity) {
            return;
        }
        world.entity_mut(entity).insert(ScrollArea);
    }
    if !world.entities().contains(entity) {
        return;
    }
    if world.get::<InteractionDisabled>(entity).is_some() == disabled {
        return;
    }
    // EntityWorldMut 的结构写入会自动 flush observer commands。
    // guard 必须覆盖整个写入，并且只匹配当前 entity 与操作，其他 entity 的真实请求仍需记录。
    let previous = world.resource::<DisabledRuntime>().projection;
    world.resource_mut::<DisabledRuntime>().projection = Some((
        entity,
        if disabled {
            Projection::Insert
        } else {
            Projection::Remove
        },
    ));
    if disabled {
        world.entity_mut(entity).insert(InteractionDisabled);
    } else {
        world.entity_mut(entity).remove::<InteractionDisabled>();
    }
    world.resource_mut::<DisabledRuntime>().projection = previous;
}

fn reconcile(world: &mut World) {
    if world.resource::<DisabledRuntime>().reconciling {
        return;
    }
    world.resource_mut::<DisabledRuntime>().reconciling = true;
    #[cfg(test)]
    {
        world.resource_mut::<DisabledRuntime>().visits = 0;
    }
    loop {
        let dirty = std::mem::take(&mut world.resource_mut::<DisabledRuntime>().dirty);
        if dirty.is_empty() {
            break;
        }
        let roots = dirty_roots(world, &dirty);
        for root in roots {
            let inherited = ancestor_disabled(world, root);
            let mut tree = Vec::new();
            let mut search = vec![root];
            while let Some(entity) = search.pop() {
                tree.push(entity);
                if let Some(children) = world.get::<Children>(entity) {
                    search.extend(children.iter());
                }
            }
            let mut needed = EntityHashSet::default();
            for &entity in tree.iter().rev() {
                if world.get::<ComputedNode>(entity).is_some() {
                    needed.insert(entity);
                }
                if needed.contains(&entity)
                    && let Some(parent) = world.get::<ChildOf>(entity)
                {
                    needed.insert(parent.parent());
                }
            }
            let mut pending = vec![(root, inherited)];
            while let Some((entity, inherited)) = pending.pop() {
                if !world.entities().contains(entity) {
                    world
                        .resource_mut::<DisabledRuntime>()
                        .local
                        .remove(&entity);
                    continue;
                }
                if !needed.contains(&entity) {
                    if let Some(state) = world
                        .resource::<DisabledRuntime>()
                        .local
                        .get(&entity)
                        .copied()
                    {
                        project(world, entity, state.explicit || state.intrinsic);
                        world
                            .resource_mut::<DisabledRuntime>()
                            .local
                            .remove(&entity);
                    }
                    if let Some(children) = world.get::<Children>(entity) {
                        pending.extend(children.iter().map(|child| (child, false)));
                    }
                    continue;
                }
                #[cfg(test)]
                {
                    world.resource_mut::<DisabledRuntime>().visits += 1;
                }
                let state = local(world, entity);
                let disabled = inherited || state.explicit || state.intrinsic;
                if let Some(state) = world
                    .resource_mut::<DisabledRuntime>()
                    .local
                    .get_mut(&entity)
                {
                    state.effective = disabled;
                }
                project(world, entity, disabled);
                if let Some(children) = world.get::<Children>(entity) {
                    pending.extend(children.iter().map(|child| (child, disabled)));
                }
            }
        }
    }
    let mut runtime = world.resource_mut::<DisabledRuntime>();
    runtime.reconciling = false;
    runtime.queued = false;
}

// 测试通过断言保护遍历工作量，仅在本 module 允许测试所需的 panic lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    //! State 为物化树的浅/深形状、祖先 enabled/disabled 与 idle。
    //! Ancestor 切换访问每个受影响 UI 一次，idle 不访问 UI，也不沿每个后代重走 ancestry。
    //! 深树使用迭代遍历，不依赖递归 stack。

    use super::*;

    #[test]
    fn affected_tree_visits_are_linear_and_stable_frames_visit_no_nodes() {
        for count in [100, 1000, 10000] {
            for deep in [false, true] {
                let mut app = App::new();
                app.add_plugins(MinimalPlugins);
                install(&mut app);
                let root = app.world_mut().spawn(Node::default()).id();
                let mut last = root;
                for _ in 1..count {
                    let parent = if deep { last } else { root };
                    last = app
                        .world_mut()
                        .spawn((Node::default(), ChildOf(parent)))
                        .id();
                }
                app.world_mut().flush();
                for disabled in [true, false] {
                    if disabled {
                        app.world_mut().entity_mut(root).insert(InteractionDisabled);
                    } else {
                        app.world_mut()
                            .entity_mut(root)
                            .remove::<InteractionDisabled>();
                    }
                    app.world_mut().flush();
                    assert_eq!(app.world().resource::<DisabledRuntime>().visits, count);
                    assert_eq!(
                        app.world()
                            .get::<WidgetryEffectiveDisabled>(last)
                            .unwrap()
                            .is_disabled(),
                        disabled
                    );
                }
                app.update();
                assert_eq!(app.world().resource::<DisabledRuntime>().visits, 0);
                println!(
                    "disabled-work nodes={count} deep={deep} toggle_visits={count} idle_visits=0"
                );
            }
        }
    }
}
