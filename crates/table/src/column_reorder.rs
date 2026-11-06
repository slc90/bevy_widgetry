use crate::runtime_error::{RuntimeResult, required};

use crate::{
    WidgetryTable, WidgetryTableColumnHeader, WidgetryTableColumnId, WidgetryTableLayout,
    WidgetryTableModel, WidgetryTableViewState,
};
use bevy::picking::{
    backend::HitData,
    events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart},
    hover::PointerCaptureMap,
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
};
use bevy::prelude::*;
use bevy::ui::{CalculatedClip, InteractionDisabled, ScrollPosition, UiGlobalTransform};
use bevy_widgetry_core::{pointer::WidgetryPointerQuery, scene::spawn_scene};
use bevy_widgetry_cursor::CursorLease;
use std::sync::Arc;

#[derive(Component, Default, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableColumnReordering {
    pub enabled: bool,
}

#[derive(Component, Clone)]
struct ColumnDrag {
    header: Entity,
    column: WidgetryTableColumnId,
    source: Entity,
    instance: Arc<()>,
    pointer: PointerId,
    camera: Entity,
    location: Location,
    start: Vec2,
    active: bool,
    fresh: bool,
    indicator: Option<Entity>,
}

#[derive(Component)]
pub(crate) struct SuppressClick;

pub(crate) fn install(app: &mut App) {
    app.register_type::<WidgetryTableColumnReordering>()
        .add_observer(release)
        .add_systems(Last, clear_suppression);
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

fn clear_suppression(
    roots: Query<Entity, (With<SuppressClick>, Without<ColumnDrag>)>,
    mut commands: Commands,
) {
    for root in &roots {
        commands.entity(root).remove::<SuppressClick>();
    }
}

fn release(
    event: On<Remove<ColumnDrag>>,
    sessions: Query<&ColumnDrag>,
    mut captures: ResMut<PointerCaptureMap>,
    mut commands: Commands,
) {
    let Ok(session) = sessions.get(event.entity) else {
        return;
    };
    if captures
        .get(&session.pointer)
        .is_some_and(|(owner, _)| owner == event.entity)
    {
        captures.release(session.pointer);
    }
    let indicator = session.indicator;
    let root = event.entity;
    commands.queue(move |world: &mut World| {
        if let Some(indicator) = indicator
            && world.entities().contains(indicator)
        {
            world.despawn(indicator);
        }
        if let Ok(mut entity) = world.get_entity_mut(root) {
            entity.remove::<CursorLease>();
        }
    });
}

pub(crate) fn active(world: &World, root: Entity) -> bool {
    world.get::<ColumnDrag>(root).is_some()
}

fn take(world: &mut World, root: Entity) -> Option<ColumnDrag> {
    let session = world.get_entity_mut(root).ok()?.take::<ColumnDrag>()?;
    if session.active {
        world.entity_mut(root).insert(SuppressClick);
    }
    world.flush();
    Some(session)
}

pub(crate) fn cancel(world: &mut World, root: Entity) {
    take(world, root);
}

fn header(world: &World, root: Entity, target: Entity) -> Option<(Entity, WidgetryTableColumnId)> {
    let mut current = target;
    let mut found = None;
    loop {
        if world.get::<InteractionDisabled>(current).is_some() {
            return None;
        }
        if current == root {
            return found;
        }
        if world.get::<crate::WidgetryTableState>(current).is_some()
            || world.get::<crate::resize::ResizeHandle>(current).is_some()
            || world
                .get::<crate::WidgetryTableHeaderAction>(current)
                .is_some()
            || world.get::<bevy::ui_widgets::Button>(current).is_some()
            || world.get::<bevy::text::EditableText>(current).is_some()
        {
            return None;
        }
        if let Some(header) = world.get::<WidgetryTableColumnHeader>(current) {
            found = Some((current, header.column));
        }
        current = world.get::<ChildOf>(current)?.parent();
    }
}

fn allowed<T: Send + Sync + 'static>(world: &World, root: Entity, session: &ColumnDrag) -> bool {
    world
        .get::<WidgetryTableColumnReordering>(root)
        .is_some_and(|config| config.enabled)
        && world
            .get::<WidgetryTable<T>>(root)
            .is_some_and(|view| view.source() == session.source)
        && world
            .get::<WidgetryTableModel<T>>(session.source)
            .is_some_and(|model| {
                Arc::ptr_eq(model.instance(), &session.instance)
                    && model.column_index(session.column).is_some()
            })
        && header(world, root, session.header) == Some((session.header, session.column))
        && crate::resize::active_column(world, root).is_none()
        && world.get::<Camera>(session.camera).is_some()
        && world
            .get_resource::<PointerCaptureMap>()
            .and_then(|map| map.get(&session.pointer))
            .is_none_or(|(owner, _)| owner == root)
}

fn endpoint(
    world: &World,
    root: Entity,
    session: &ColumnDrag,
) -> RuntimeResult<Option<(usize, f32)>> {
    let camera = required(
        world.get::<Camera>(session.camera),
        session.camera,
        "Camera",
    )?;
    let Some(scale) = camera.target_scaling_factor() else {
        return Ok(None);
    };
    let mut physical = session.location.position * scale;
    if let Some(viewport) = camera.physical_viewport_rect() {
        physical -= viewport.min.as_vec2();
    }
    let computed = required(world.get::<ComputedNode>(root), root, "ComputedNode")?;
    if !physical.is_finite()
        || world
            .get::<CalculatedClip>(root)
            .is_some_and(|clip| !clip.contains_point(physical))
    {
        return Ok(None);
    }
    let Some(inverse) = required(
        world.get::<UiGlobalTransform>(root),
        root,
        "UiGlobalTransform",
    )?
    .try_inverse() else {
        return Ok(None);
    };
    let local = (inverse.transform_point2(physical) - computed.content_box().min)
        * computed.inverse_scale_factor();
    let layout = required(
        world.get::<WidgetryTableLayout>(root),
        root,
        "WidgetryTableLayout",
    )?;
    let point = local - Vec2::new(layout.row_header_width, 0.0);
    let view = required(
        world.get::<WidgetryTableViewState>(root),
        root,
        "WidgetryTableViewState",
    )?;
    let width = crate::projection::viewport_width(world, root, layout)?;
    if !point.is_finite()
        || point.x < 0.0
        || point.x >= width
        || point.y < 0.0
        || point.y >= layout.column_header_height
        || view.ordered_columns().is_empty()
    {
        return Ok(None);
    }
    let geometry = layout
        .resolve(view.ordered_columns().to_vec(), view.len(), width)
        .map_err(|fault| {
            crate::error::layout_failure(
                fault,
                root,
                Some(session.source),
                "table.column_drag.layout",
            )
            .into_bevy_error()
        })?;
    let runtime = required(
        world.get::<crate::projection::TableRuntime>(root),
        root,
        "crate::projection::TableRuntime",
    )?;
    let body = required(
        world.get::<ComputedNode>(runtime.body),
        runtime.body,
        "ComputedNode",
    )
    .map_err(|error| error.at(crate::TableRegion::Body))?;
    let inverse = body.inverse_scale_factor();
    let offset = (required(
        world.get::<ScrollPosition>(runtime.body),
        runtime.body,
        "ScrollPosition",
    )
    .map_err(|error| error.at(crate::TableRegion::Body))?
    .0
    .x / inverse)
        .floor()
        * inverse;
    let x = point.x
        + if point.x >= geometry.frozen_width {
            offset
        } else {
            0.0
        };
    let slot = geometry
        .columns
        .partition_point(|column| column.left + column.width * 0.5 < x);
    let boundary = geometry
        .columns
        .get(slot)
        .map_or(geometry.width, |column| column.left);
    let frozen_count = geometry.columns.partition_point(|column| column.frozen);
    let marker = (boundary - if slot <= frozen_count { 0.0 } else { offset }).clamp(0.0, width);
    let Some(old) = view.column_display_index(session.column) else {
        return Ok(None);
    };
    Ok(Some((slot - usize::from(old < slot), marker)))
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
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        crate::runtime_error::run_request_at::<T, _>(
            world,
            root,
            target,
            "table.column_reorder.input",
            |world| {
                (|| -> RuntimeResult {
                    if active(world, root)
                        || crate::resize::active_column(world, root).is_some()
                        || world
                            .get::<WidgetryTableColumnReordering>(root)
                            .is_none_or(|config| !config.enabled)
                        || world
                            .get_resource::<PointerCaptureMap>()
                            .is_some_and(|map| map.is_captured(&pointer))
                    {
                        return Ok(());
                    }
                    let Some((header, column)) = header(world, root, target) else {
                        return Ok(());
                    };
                    if !location.position.is_finite() || world.get::<Camera>(camera).is_none() {
                        return Ok(());
                    }
                    crate::view_state::refresh::<T>(world, root)
                        .map_err(crate::TableFailure::into_bevy_error)?;
                    let source = required(
                        world.get::<WidgetryTable<T>>(root),
                        root,
                        "WidgetryTable<T>",
                    )?
                    .source();
                    let instance = required(
                        world.get::<WidgetryTableModel<T>>(source),
                        source,
                        "WidgetryTableModel<T>",
                    )?
                    .instance()
                    .clone();
                    world.entity_mut(root).insert(ColumnDrag {
                        header,
                        column,
                        source,
                        instance,
                        pointer,
                        camera,
                        start: location.position,
                        location,
                        active: false,
                        fresh: true,
                        indicator: None,
                    });
                    Ok(())
                })()
            },
        )
    });
}

fn apply<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    pointer: PointerId,
    location: Location,
    distance: Option<Vec2>,
) -> RuntimeResult {
    let Some(mut session) = world.get::<ColumnDrag>(root).cloned() else {
        return Ok(());
    };
    if session.pointer != pointer || session.location.target != location.target {
        return Ok(());
    }
    if !allowed::<T>(world, root, &session) {
        cancel(world, root);
        return Ok(());
    }
    if !location.position.is_finite() || distance.is_some_and(|d| !d.is_finite()) {
        return Ok(());
    }
    crate::view_state::refresh::<T>(world, root).map_err(crate::TableFailure::into_bevy_error)?;
    session.location = location;
    session.active |= distance
        .unwrap_or(session.location.position - session.start)
        .length_squared()
        >= 16.0;
    if session.active {
        world.entity_mut(root).insert(SuppressClick);
        let destination = endpoint(world, root, &session)?;
        if let Some((_, marker)) = destination {
            let layout = required(
                world.get::<WidgetryTableLayout>(root),
                root,
                "WidgetryTableLayout",
            )?;
            let computed = required(world.get::<ComputedNode>(root), root, "ComputedNode")?;
            let origin = computed.content_box().min + computed.size() * 0.5;
            let position = origin * computed.inverse_scale_factor()
                + Vec2::new(layout.row_header_width + marker, 0.0);
            let node = Node {
                position_type: PositionType::Absolute,
                left: px(position.x - 1.0),
                top: px(position.y),
                width: px(2),
                height: px(layout.column_header_height),
                ..default()
            };
            let color = world
                .resource::<bevy_widgetry_theme::WidgetryTheme>()
                .table
                .column_header
                .selected
                .border;
            if let Some(indicator) = session.indicator {
                required(world.get_mut::<Node>(indicator), indicator, "Node")?.set_if_neq(node);
                required(
                    world.get_mut::<BackgroundColor>(indicator),
                    indicator,
                    "BackgroundColor",
                )?
                .set_if_neq(BackgroundColor(color));
            } else {
                let indicator=spawn_scene(world,bsn!{template(move |_|Ok(node.clone())) BackgroundColor({color}) ZIndex(2) Pickable::IGNORE}).map_err(bevy_widgetry_core::scene::SceneFailure::into_bevy_error)?;
                world.entity_mut(root).add_child(indicator);
                session.indicator = Some(indicator);
            }
        } else if let Some(indicator) = session.indicator.take() {
            world.despawn(indicator);
        }
        if let Some(mut captures) = world.get_resource_mut::<PointerCaptureMap>()
            && !captures.is_captured(&pointer)
        {
            captures.capture(pointer, root, HitData::new(session.camera, 0.0, None, None));
        }
        if let Some(lease) = CursorLease::dragging(
            pointer,
            &session.location,
            PointerButton::Primary,
            bevy::window::SystemCursorIcon::Grabbing,
        ) {
            world.entity_mut(root).insert(lease);
        }
    }
    world.entity_mut(root).insert(session);
    Ok(())
}

fn complete<T: Send + Sync + 'static>(world: &mut World, root: Entity) -> RuntimeResult {
    let destination = world
        .get::<ColumnDrag>(root)
        .filter(|session| session.active && allowed::<T>(world, root, session))
        .map(|session| endpoint(world, root, session))
        .transpose()?
        .flatten();
    let Some(session) = take(world, root) else {
        return Ok(());
    };
    if let Some((index, _)) = destination
        && allowed::<T>(world, root, &session)
    {
        crate::view_state::move_column::<T>(world, root, session.column, index)
            .map_err(crate::TableFailure::into_bevy_error)?;
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
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        crate::runtime_error::run_request::<T, _>(
            world,
            root,
            "table.column_reorder.input",
            |world| apply::<T>(world, root, pointer, location, Some(distance)),
        )
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
    let target = event.original_event_target();
    let pointer = event.pointer.id;
    let location = event.pointer.location();
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        crate::runtime_error::run_request_at::<T, _>(
            world,
            root,
            target,
            "table.column_reorder.input",
            |world| {
                (|| -> RuntimeResult {
                    if world.get::<ColumnDrag>(root).is_some_and(|s| {
                        s.pointer == pointer && s.location.target == location.target
                    }) {
                        apply::<T>(world, root, pointer, location, None)?;
                        complete::<T>(world, root)?;
                    }
                    Ok(())
                })()
            },
        )
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
            .get::<ColumnDrag>(root)
            .is_some_and(|s| s.pointer == pointer)
        {
            cancel(world, root);
        }
    });
}

pub(crate) fn refresh<T: Send + Sync + 'static>(world: &mut World, root: Entity) -> RuntimeResult {
    if let Some(session) = world.get::<ColumnDrag>(root).cloned() {
        apply::<T>(world, root, session.pointer, session.location, None)?;
    }
    Ok(())
}

fn track<T: Send + Sync + 'static>(
    mut sessions: Query<(Entity, &mut ColumnDrag), With<WidgetryTable<T>>>,
    pointers: WidgetryPointerQuery,
    mut inputs: MessageReader<PointerInput>,
    mut commands: Commands,
) {
    let inputs = inputs.read().collect::<Vec<_>>();
    for (root, mut session) in &mut sessions {
        let mut location = session.location.clone();
        let mut terminal = None;
        for input in &inputs {
            if input.pointer_id != session.pointer
                || input.location.target != session.location.target
            {
                continue;
            }
            location = input.location.clone();
            match input.action {
                PointerAction::Cancel => terminal = Some(true),
                PointerAction::Release(PointerButton::Primary) => terminal = Some(false),
                PointerAction::Press(PointerButton::Primary) if session.fresh => terminal = None,
                _ => {}
            }
        }
        if pointers
            .location(session.pointer)
            .is_none_or(|current| current.target != session.location.target)
        {
            terminal = Some(true);
        }
        session.fresh = false;
        let pointer = session.pointer;
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            crate::runtime_error::run_request::<T, _>(
                world,
                root,
                "table.column_reorder.input",
                |world| {
                    (|| -> RuntimeResult {
                        if terminal == Some(true) {
                            cancel(world, root);
                            return Ok(());
                        }
                        apply::<T>(world, root, pointer, location, None)?;
                        if terminal == Some(false) {
                            complete::<T>(world, root)?;
                        }
                        Ok(())
                    })()
                },
            )
        });
    }
}
