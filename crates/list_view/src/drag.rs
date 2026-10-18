use crate::source::DataSource;
use crate::{
    WidgetryListItemId, WidgetryListKey, WidgetryListModel, WidgetryListProjection,
    WidgetryListView, WidgetryListViewItem, WidgetryListViewState,
};
use bevy::picking::{
    events::Pointer,
    pointer::{PointerAction, PointerButton, PointerId, PointerInput},
};
use bevy::ui::InteractionDisabled;
use bevy::{camera::NormalizedRenderTarget, prelude::*};
use bevy_widgetry_core::pointer::WidgetryPointerQuery;
use std::marker::PhantomData;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edge {
    Before,
    After,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct DropHint(pub(crate) Edge);

#[derive(Clone, Copy)]
pub(crate) struct MoveIntent<K> {
    pub(crate) moving: K,
    pub(crate) anchor: K,
    pub(crate) edge: Edge,
}

#[derive(Resource)]
pub(crate) struct Commit<T: Send + Sync + 'static, K: WidgetryListKey> {
    apply: fn(&mut World, Entity, MoveIntent<K>) -> Result<bool, BevyError>,
    marker: PhantomData<fn() -> T>,
}

#[derive(Component, Clone)]
struct DragState<K: WidgetryListKey> {
    source: Entity,
    moving: K,
    row: Entity,
    dragged: Entity,
    pointer: PointerId,
    target: NormalizedRenderTarget,
    dragging: bool,
    cancelled: bool,
    drop: Option<(Entity, MoveIntent<K>)>,
}

pub(crate) fn register_model<T: Send + Sync + 'static>(app: &mut App) {
    app.insert_resource(Commit::<T, WidgetryListItemId> {
        apply: move_model::<T>,
        marker: PhantomData,
    });
}

fn move_model<T: Send + Sync + 'static>(
    world: &mut World,
    source: Entity,
    intent: MoveIntent<WidgetryListItemId>,
) -> Result<bool, BevyError> {
    let Some(model) = world.get::<WidgetryListModel<T>>(source) else {
        return Ok(false);
    };
    let (Some(from), Some(anchor)) = (model.index_of(intent.moving), model.index_of(intent.anchor))
    else {
        return Ok(false);
    };
    if from == anchor
        || model.is_disabled(from) != Some(false)
        || model.is_disabled(anchor) != Some(false)
    {
        return Ok(false);
    }
    let insertion = anchor + usize::from(intent.edge == Edge::After);
    let to = insertion - usize::from(from < insertion);
    if from == to {
        return Ok(false);
    }
    if let Some(mut model) = world.get_mut::<WidgetryListModel<T>>(source) {
        model.move_item(from, to);
    }
    Ok(true)
}

pub(crate) fn install<T: Send + Sync + 'static, K: WidgetryListKey>(app: &mut App) {
    app.add_observer(on_press::<T, K>)
        .add_observer(on_start::<T, K>)
        .add_observer(on_over::<T, K>)
        .add_observer(on_drop::<T, K>)
        .add_observer(on_leave::<T, K>)
        .add_observer(on_end::<T, K>)
        .add_observer(on_cancel::<T, K>)
        .add_observer(on_disabled::<T, K>);
    app.add_systems(
        PreUpdate,
        cleanup::<T, K>.after(bevy::picking::PickingSystems::Last),
    );
    app.add_systems(
        PostUpdate,
        repair::<T, K>
            .after(crate::WidgetryListViewSystems::Reconcile)
            .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
    );
}

fn row_at<K: WidgetryListKey>(world: &World, root: Entity, mut target: Entity) -> Option<Entity> {
    let mut row = None;
    loop {
        if world.get::<InteractionDisabled>(target).is_some() {
            return None;
        }
        if target == root {
            return row;
        }
        if world.get::<WidgetryListViewState<K>>(target).is_some() {
            return None;
        }
        if row.is_none() && world.get::<WidgetryListViewItem<K>>(target).is_some() {
            row = Some(target);
        }
        target = world.get::<ChildOf>(target)?.parent();
    }
}

fn valid<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &World,
    root: Entity,
    state: &DragState<K>,
) -> bool {
    !state.cancelled
        && world
            .get::<WidgetryListView<T, K>>(root)
            .is_some_and(|v| v.reorderable() && v.source() == state.source)
        && row_at::<K>(world, root, state.row) == Some(state.row)
        && world
            .get::<WidgetryListViewItem<K>>(state.row)
            .is_some_and(|i| i.id == state.moving)
        && world
            .get::<WidgetryListProjection<K>>(root)
            .is_some_and(|p| p.index_of(state.moving).is_some())
        && world
            .get_resource::<DataSource<T, K>>()
            .and_then(|a| (a.row)(world, state.source, state.moving))
            .is_some_and(|r| !r.disabled)
}

fn clear_hint<K: WidgetryListKey>(world: &mut World, root: Entity) {
    let hint = world
        .get::<DragState<K>>(root)
        .and_then(|s| s.drop.as_ref().map(|(row, _)| *row));
    if let Some(row) = hint
        && let Ok(mut e) = world.get_entity_mut(row)
    {
        e.remove::<DropHint>();
    }
    if let Some(mut s) = world.get_mut::<DragState<K>>(root)
        && s.drop.is_some()
    {
        s.drop = None;
    }
}

fn clear<K: WidgetryListKey>(world: &mut World, root: Entity) {
    clear_hint::<K>(world, root);
    if let Ok(mut e) = world.get_entity_mut(root) {
        e.remove::<DragState<K>>();
    }
}

fn cancel<K: WidgetryListKey>(world: &mut World, root: Entity) {
    clear_hint::<K>(world, root);
    if let Some(mut state) = world.get_mut::<DragState<K>>(root) {
        // Picking still emits Click for this press after a cancelled data move.
        state.cancelled = true;
    }
}

pub(crate) fn suppress_click<K: WidgetryListKey>(
    world: &World,
    root: Entity,
    pointer: PointerId,
) -> bool {
    world
        .get::<DragState<K>>(root)
        .is_some_and(|s| s.dragging && s.pointer == pointer)
}

fn on_press<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<PointerPress>,
    views: Query<&WidgetryListView<T, K>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if event.button != PointerButton::Primary
        || !views.get(root).is_ok_and(WidgetryListView::reorderable)
    {
        return;
    }
    let target = event.original_event_target();
    let pointer = event.pointer.clone();
    commands.queue(move |world: &mut World| {
        if world.get::<DragState<K>>(root).is_some_and(|s| s.dragging) {
            return;
        }
        clear::<K>(world, root);
        crate::projection::refresh_root::<T, K>(world, root);
        let Some(row) = row_at::<K>(world, root, target) else {
            return;
        };
        let Some(item) = world.get::<WidgetryListViewItem<K>>(row) else {
            return;
        };
        let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
            return;
        };
        let state = DragState {
            source: view.source(),
            moving: item.id,
            row,
            dragged: target,
            pointer: pointer.id,
            target: pointer.target,
            dragging: false,
            cancelled: false,
            drop: None,
        };
        if valid::<T, K>(world, root, &state) {
            world.entity_mut(root).insert(state);
        }
    });
}

fn on_start<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<PointerDragStart>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if !views.contains(event.entity) || event.button != PointerButton::Primary {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    let pointer = event.pointer.clone();
    commands.queue(move |world: &mut World| {
        crate::projection::refresh_root::<T, K>(world, root);
        let Some(state) = world.get::<DragState<K>>(root).cloned() else {
            return;
        };
        if state.dragged != target || state.pointer != pointer.id || state.target != pointer.target
        {
            return;
        }
        if let Some(mut state) = world.get_mut::<DragState<K>>(root) {
            state.dragging = true;
        }
        if !valid::<T, K>(world, root, &state) {
            cancel::<K>(world, root);
        }
    });
}

fn candidate<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &World,
    root: Entity,
    target: Entity,
    pointer: &Pointer,
    dragged: Entity,
    camera: Entity,
) -> Option<(Entity, MoveIntent<K>)> {
    let state = world.get::<DragState<K>>(root)?;
    if !state.dragging
        || state.dragged != dragged
        || state.pointer != pointer.id
        || state.target != pointer.target
        || !valid::<T, K>(world, root, state)
    {
        return None;
    }
    let row = row_at::<K>(world, root, target)?;
    let anchor = world.get::<WidgetryListViewItem<K>>(row)?.id;
    if anchor == state.moving
        || world
            .get::<WidgetryListProjection<K>>(root)?
            .index_of(anchor)
            .is_none()
        || (world.resource::<DataSource<T, K>>().row)(world, state.source, anchor)?.disabled
    {
        return None;
    }
    let node = world.get::<ComputedNode>(row)?;
    let camera = world.get::<Camera>(camera)?;
    let scale = camera.target_scaling_factor().unwrap_or(1.0);
    if !scale.is_finite() || scale <= 0.0 || node.size().min_element() <= 0.0 {
        return None;
    }
    let mut position = pointer.position * scale;
    if let Some(viewport) = camera.physical_viewport_rect() {
        if !viewport.as_rect().contains(position) {
            return None;
        }
        position -= viewport.min.as_vec2();
    }
    let local = world
        .get::<UiGlobalTransform>(row)?
        .try_inverse()?
        .transform_point2(position);
    if !local.is_finite() {
        return None;
    }
    Some((
        row,
        MoveIntent {
            moving: state.moving,
            anchor,
            edge: if local.y < 0.0 {
                Edge::Before
            } else {
                Edge::After
            },
        },
    ))
}

fn on_over<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<PointerDragOver>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if !views.contains(event.entity) || event.button != PointerButton::Primary {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    let pointer = event.pointer.clone();
    let dragged = event.dragged;
    let camera = event.hit.camera;
    commands.queue(move |world: &mut World| {
        if !world.get::<DragState<K>>(root).is_some_and(|s| {
            s.pointer == pointer.id && s.target == pointer.target && s.dragged == dragged
        }) {
            return;
        }
        crate::projection::refresh_root::<T, K>(world, root);
        let next = candidate::<T, K>(world, root, target, &pointer, dragged, camera);
        clear_hint::<K>(world, root);
        if let Some((row, intent)) = next {
            world.entity_mut(row).insert(DropHint(intent.edge));
            if let Some(mut state) = world.get_mut::<DragState<K>>(root) {
                state.drop = Some((row, intent));
            }
        }
    });
}

fn on_drop<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<PointerDragDrop>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if !views.contains(event.entity) || event.button != PointerButton::Primary {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    let pointer = event.pointer.clone();
    let dropped = event.dropped;
    let camera = event.hit.camera;
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        crate::projection::refresh_root::<T, K>(world, root);
        let Some(state) = world.get::<DragState<K>>(root).cloned() else {
            return Ok(());
        };
        if state.pointer != pointer.id || state.target != pointer.target || state.dragged != dropped
        {
            return Ok(());
        }
        let next = candidate::<T, K>(world, root, target, &pointer, dropped, camera);
        let intended = next.filter(|(row, intent)| {
            state
                .drop
                .as_ref()
                .is_some_and(|(old, prior)| old == row && prior.anchor == intent.anchor)
        });
        cancel::<K>(world, root);
        if let Some((_, intent)) = intended {
            let Some(commit) = world.get_resource::<Commit<T, K>>().map(|c| c.apply) else {
                bevy_widgetry_internal_log::widgetry_error!(?root, "ListView 拖放执行器缺失");
                return Err(BevyError::error("ListView drag commit missing"));
            };
            commit(world, state.source, intent)?;
        }
        Ok(())
    });
}

fn on_leave<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<PointerDragLeave>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if !views.contains(event.entity) || event.button != PointerButton::Primary {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    let pointer = event.pointer.clone();
    let dragged = event.dragged;
    commands.queue(move |world: &mut World| {
        if world.get::<DragState<K>>(root).is_some_and(|s| {
            s.pointer == pointer.id
                && s.target == pointer.target
                && s.dragged == dragged
                && s.drop
                    .as_ref()
                    .is_some_and(|(row, _)| row_at::<K>(world, root, target) == Some(*row))
        }) {
            clear_hint::<K>(world, root);
        }
    });
}

fn on_disabled<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<Add<InteractionDisabled>>,
    states: Query<(Entity, &DragState<K>), With<WidgetryListView<T, K>>>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    for (root, _) in &states {
        if event.entity == root || parents.iter_ancestors(event.entity).any(|e| e == root) {
            commands.queue(move |world: &mut World| cancel::<K>(world, root));
        }
    }
}

fn on_end<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<PointerDragEnd>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if !views.contains(event.entity) || event.button != PointerButton::Primary {
        return;
    }
    let root = event.entity;
    let pointer = event.pointer.clone();
    commands.queue(move |world: &mut World| {
        if world
            .get::<DragState<K>>(root)
            .is_some_and(|s| s.pointer == pointer.id && s.target == pointer.target)
        {
            clear::<K>(world, root);
        }
    });
}

fn on_cancel<T: Send + Sync + 'static, K: WidgetryListKey>(
    event: On<PointerCancel>,
    views: Query<(), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    if !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let pointer = event.pointer.clone();
    commands.queue(move |world: &mut World| {
        if world
            .get::<DragState<K>>(root)
            .is_some_and(|s| s.pointer == pointer.id && s.target == pointer.target)
        {
            clear::<K>(world, root);
        }
    });
}

fn cleanup<T: Send + Sync + 'static, K: WidgetryListKey>(
    mut input: MessageReader<PointerInput>,
    pointers: WidgetryPointerQuery,
    states: Query<(Entity, &DragState<K>), With<WidgetryListView<T, K>>>,
    mut commands: Commands,
) {
    let events = input.read().collect::<Vec<_>>();
    for (root, state) in &states {
        if pointers
            .location(state.pointer)
            .is_none_or(|p| p.target != state.target)
            || events.iter().any(|i| {
                i.pointer_id == state.pointer
                    && i.location.target == state.target
                    && matches!(
                        i.action,
                        PointerAction::Cancel | PointerAction::Release(PointerButton::Primary)
                    )
            })
        {
            commands.queue(move |world: &mut World| clear::<K>(world, root));
        }
    }
}

fn repair<T: Send + Sync + 'static, K: WidgetryListKey>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, (With<DragState<K>>, With<WidgetryListView<T, K>>)>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let Some(state) = world.get::<DragState<K>>(root).cloned() else {
            continue;
        };
        if !valid::<T, K>(world, root, &state) {
            cancel::<K>(world, root);
            continue;
        }
        if state.drop.is_some_and(|(row, intent)| {
            row_at::<K>(world, root, row) != Some(row)
                || world
                    .get::<WidgetryListViewItem<K>>(row)
                    .is_none_or(|i| i.id != intent.anchor)
                || world
                    .get_resource::<DataSource<T, K>>()
                    .and_then(|a| (a.row)(world, state.source, intent.anchor))
                    .is_none_or(|r| r.disabled)
        }) {
            clear_hint::<K>(world, root);
        }
    }
}
