use crate::interaction::{clicked, valid};
use crate::layout::TableGeometry;
use crate::projection::{TableRuntime, required};
use crate::{
    WidgetryTable, WidgetryTableCell, WidgetryTableModel, WidgetryTableSelection,
    WidgetryTableViewState,
};
use bevy::camera::NormalizedRenderTarget;
use bevy::picking::{
    backend::HitData,
    events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart},
    hover::PointerCaptureMap,
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition, UiGlobalTransform};
use bevy_widgetry_core::pointer::WidgetryPointerQuery;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Component, Clone)]
struct SelectionDrag {
    anchor: WidgetryTableCell,
    end: WidgetryTableCell,
    body: Entity,
    pointer: PointerId,
    target: NormalizedRenderTarget,
    camera: Entity,
    start: Vec2,
    position: Vec2,
    active: bool,
}

#[derive(Component)]
pub(crate) struct SuppressClick;

pub(crate) fn install(app: &mut App) {
    app.add_observer(release_capture)
        .add_systems(Last, clear_click_suppression);
}

pub(crate) fn install_typed<T: Send + Sync + 'static>(app: &mut App) {
    app.add_observer(on_start::<T>)
        .add_observer(on_drag::<T>)
        .add_observer(on_end::<T>)
        .add_observer(on_cancel::<T>)
        .add_systems(
            PreUpdate,
            track::<T>.after(bevy::picking::PickingSystems::Last),
        );
}

fn release_capture(
    event: On<Remove<SelectionDrag>>,
    sessions: Query<&SelectionDrag>,
    mut captures: ResMut<PointerCaptureMap>,
) {
    if let Ok(session) = sessions.get(event.entity)
        && captures
            .get(&session.pointer)
            .is_some_and(|(owner, _)| owner == session.body)
    {
        captures.release(session.pointer);
    }
}

fn clear_click_suppression(
    roots: Query<Entity, (With<SuppressClick>, Without<SelectionDrag>)>,
    mut commands: Commands,
) {
    for root in &roots {
        commands.entity(root).remove::<SuppressClick>();
    }
}

pub(crate) fn finish(world: &mut World, root: Entity) {
    if let Some(session) = world.get::<SelectionDrag>(root) {
        let active = session.active;
        world.entity_mut(root).remove::<SelectionDrag>();
        if active {
            world.entity_mut(root).insert(SuppressClick);
        }
    }
}

fn allowed<T: Send + Sync + 'static>(world: &World, root: Entity, session: &SelectionDrag) -> bool {
    let Some(source) = world
        .get::<WidgetryTable<T>>(root)
        .map(WidgetryTable::source)
    else {
        return false;
    };
    let Some(model) = world.get::<WidgetryTableModel<T>>(source) else {
        return false;
    };
    let Some(view) = world.get::<WidgetryTableViewState>(root) else {
        return false;
    };
    world.get::<InteractionDisabled>(root).is_none()
        && world.get::<InteractionDisabled>(session.body).is_none()
        && world
            .get::<ChildOf>(session.body)
            .is_some_and(|parent| parent.parent() == root)
        && valid(
            model,
            view,
            WidgetryTableSelection::Cells {
                anchor: session.anchor,
                end: session.end,
            },
        )
        && world.get::<Camera>(session.camera).is_some()
}

fn on_start<T: Send + Sync + 'static>(
    event: On<PointerDragStart>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let target = event.original_event_target();
    let pointer = event.pointer.id;
    let location = event.pointer.location();
    let camera = event.hit.camera;
    commands.queue(move |world: &mut World| -> Result {
        begin::<T>(world, root, target, pointer, location, camera)
            .inspect_err(|error| widgetry_error!(?root, %error, "Table 拖选开始失败"))
    });
}

fn begin<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    target: Entity,
    pointer: PointerId,
    location: Location,
    camera: Entity,
) -> Result {
    let mut current = target;
    while current != root {
        if world.get::<bevy::ui_widgets::Button>(current).is_some()
            || world.get::<bevy::text::EditableText>(current).is_some()
        {
            return Ok(());
        }
        let Some(parent) = world.get::<ChildOf>(current) else {
            return Ok(());
        };
        current = parent.parent();
    }
    if world.get::<SelectionDrag>(root).is_some()
        || crate::resize::active_column(world, root).is_some()
        || world
            .get_resource::<PointerCaptureMap>()
            .is_some_and(|map| map.is_captured(&pointer))
    {
        return Ok(());
    }
    let Some(WidgetryTableSelection::Cell { row, column }) = clicked(world, root, target) else {
        return Ok(());
    };
    if world.get::<Camera>(camera).is_none() || !location.position.is_finite() {
        return Ok(());
    }
    crate::view_state::refresh::<T>(world, root)?;
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let view = required(world.get::<WidgetryTableViewState>(root))?;
    if !valid(model, view, WidgetryTableSelection::Cell { row, column }) {
        return Ok(());
    }
    let body = required(world.get::<TableRuntime>(root))?.body;
    let anchor = WidgetryTableCell { row, column };
    world.entity_mut(root).insert(SelectionDrag {
        anchor,
        end: anchor,
        body,
        pointer,
        target: location.target,
        camera,
        start: location.position,
        position: location.position,
        active: false,
    });
    Ok(())
}

fn endpoint(world: &World, root: Entity, session: &SelectionDrag) -> Option<WidgetryTableCell> {
    let camera = world.get::<Camera>(session.camera)?;
    let scale = camera.target_scaling_factor()?;
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let mut physical = session.position * scale;
    if let Some(viewport) = camera.physical_viewport_rect() {
        physical -= viewport.min.as_vec2();
    }
    let computed = world.get::<ComputedNode>(session.body)?;
    let size = computed.size() * computed.inverse_scale_factor();
    if !size.is_finite() || size.min_element() <= 0.0 {
        return None;
    }
    let local = world
        .get::<UiGlobalTransform>(session.body)?
        .try_inverse()?
        .transform_point2(physical);
    let point = (local + computed.size() * 0.5) * computed.inverse_scale_factor();
    if !point.is_finite() {
        return None;
    }
    let point = point.clamp(Vec2::ZERO, (size - Vec2::splat(0.001)).max(Vec2::ZERO))
        + world.get::<ScrollPosition>(session.body)?.0;
    let view = world.get::<WidgetryTableViewState>(root)?;
    let geometry = world.get::<TableGeometry>(root)?;
    if view.is_empty() || geometry.columns.is_empty() {
        return None;
    }
    let row = ((point.y / geometry.row_height).floor() as usize).min(view.len() - 1);
    let column = geometry
        .columns
        .partition_point(|column| column.left + column.width <= point.x)
        .min(geometry.columns.len() - 1);
    Some(WidgetryTableCell {
        row: view.row_id(row)?,
        column: geometry.columns[column].id,
    })
}

fn apply<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    pointer: PointerId,
    location: &Location,
    distance: Option<Vec2>,
) -> Result {
    let Some(mut session) = world.get::<SelectionDrag>(root).cloned() else {
        return Ok(());
    };
    if session.pointer != pointer || session.target != location.target {
        return Ok(());
    }
    if !location.position.is_finite() || distance.is_some_and(|distance| !distance.is_finite()) {
        return Ok(());
    }
    if world
        .get::<WidgetryTable<T>>(root)
        .is_none_or(|view| world.get::<WidgetryTableModel<T>>(view.source()).is_none())
    {
        finish(world, root);
        return Ok(());
    }
    crate::view_state::refresh::<T>(world, root)?;
    if !allowed::<T>(world, root, &session) {
        finish(world, root);
        return Ok(());
    }
    if world
        .get_resource::<PointerCaptureMap>()
        .and_then(|map| map.get(&pointer))
        .is_some_and(|(owner, _)| owner != session.body)
    {
        finish(world, root);
        return Ok(());
    }
    session.position = location.position;
    let distance = distance.unwrap_or(session.position - session.start);
    session.active |= distance.length_squared() >= 16.0;
    let end = if session.active {
        endpoint(world, root, &session)
    } else {
        None
    };
    let active = session.active;
    let body = session.body;
    let camera = session.camera;
    let anchor = session.anchor;
    if let Some(end) = end {
        session.end = end;
    }
    world.entity_mut(root).insert(session);
    if active {
        world.entity_mut(root).insert(SuppressClick);
        if let Some(mut map) = world.get_resource_mut::<PointerCaptureMap>()
            && map.get(&pointer).is_none()
        {
            map.capture(pointer, body, HitData::new(camera, 0.0, None, None));
        }
        if let Some(end) = end {
            WidgetryTable::<T>::set_focused_cell(world, root, Some(end))?;
            WidgetryTable::<T>::set_selection(
                world,
                root,
                WidgetryTableSelection::Cells { anchor, end },
            )?;
        }
    }
    Ok(())
}

fn on_drag<T: Send + Sync + 'static>(
    event: On<PointerDrag>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let pointer = event.pointer.id;
    let location = event.pointer.location();
    let distance = event.distance;
    commands.queue(move |world: &mut World| -> Result {
        apply::<T>(world, root, pointer, &location, Some(distance))
            .inspect_err(|error| widgetry_error!(?root, %error, "Table 拖选更新失败"))
    });
}

fn on_end<T: Send + Sync + 'static>(
    event: On<PointerDragEnd>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary || !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let pointer = event.pointer.id;
    let location = event.pointer.location();
    commands.queue(move |world: &mut World| -> Result {
        if world
            .get::<SelectionDrag>(root)
            .is_some_and(|session| session.pointer == pointer && session.target == location.target)
        {
            apply::<T>(world, root, pointer, &location, None)?;
            finish(world, root);
        }
        Ok(())
    });
}

fn on_cancel<T: Send + Sync + 'static>(
    event: On<PointerCancel>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    if !views.contains(event.entity) {
        return;
    }
    let root = event.entity;
    let pointer = event.pointer.id;
    commands.queue(move |world: &mut World| {
        if world
            .get::<SelectionDrag>(root)
            .is_some_and(|session| session.pointer == pointer)
        {
            finish(world, root);
        }
    });
}

fn track<T: Send + Sync + 'static>(
    pointers: WidgetryPointerQuery,
    sessions: Query<(Entity, &SelectionDrag), With<WidgetryTable<T>>>,
    mut inputs: MessageReader<PointerInput>,
    mut commands: Commands,
) {
    let inputs = inputs.read().collect::<Vec<_>>();
    for (root, session) in &sessions {
        let location = pointers.location(session.pointer).cloned();
        let pointer = session.pointer;
        let invalid = location
            .as_ref()
            .is_none_or(|location| location.target != session.target);
        let terminal = inputs.iter().any(|input| {
            input.pointer_id == pointer
                && input.location.target == session.target
                && matches!(
                    input.action,
                    PointerAction::Release(PointerButton::Primary) | PointerAction::Cancel
                )
        });
        commands.queue(move |world: &mut World| -> Result {
            if !invalid && let Some(location) = location {
                apply::<T>(world, root, pointer, &location, None)?;
            }
            if terminal || invalid {
                finish(world, root);
            }
            Ok(())
        });
    }
}

pub(crate) fn refresh<T: Send + Sync + 'static>(world: &mut World, root: Entity) -> Result {
    let Some(session) = world.get::<SelectionDrag>(root).cloned() else {
        return Ok(());
    };
    let location = Location {
        target: session.target,
        position: session.position,
    };
    apply::<T>(world, root, session.pointer, &location, None)
}
