use crate::layout::TableGeometry;
use crate::projection::required;
use crate::{
    WidgetryTable, WidgetryTableColumnId, WidgetryTableColumnWidth, WidgetryTableEvent,
    WidgetryTableEventKind, WidgetryTableLayout, WidgetryTableModel, WidgetryTableState,
};
use bevy::camera::NormalizedRenderTarget;
use bevy::picking::{
    events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart},
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::pointer::WidgetryPointerQuery;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_cursor::CursorLease;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Component)]
pub(crate) struct ResizeHandle {
    column: WidgetryTableColumnId,
}

#[derive(Component, Clone)]
struct ResizeSession {
    handle: Entity,
    column: WidgetryTableColumnId,
    pointer: PointerId,
    target: NormalizedRenderTarget,
    fresh: bool,
    start_width: f32,
    width: f32,
    inverse_ui_scale: f32,
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        PreUpdate,
        finish_invalid_sessions.after(bevy::picking::PickingSystems::Last),
    );
}

fn finish_invalid_sessions(
    pointers: WidgetryPointerQuery,
    mut sessions: Query<(Entity, &mut ResizeSession)>,
    mut inputs: MessageReader<PointerInput>,
    mut commands: Commands,
) {
    let inputs = inputs.read().collect::<Vec<_>>();
    for (root, mut session) in &mut sessions {
        let invalid = pointers
            .location(session.pointer)
            .is_none_or(|location| location.target != session.target);
        let mut terminal = None;
        for input in &inputs {
            if input.pointer_id != session.pointer || input.location.target != session.target {
                continue;
            }
            match input.action {
                PointerAction::Cancel => terminal = Some(true),
                PointerAction::Release(PointerButton::Primary) => terminal = Some(false),
                PointerAction::Press(PointerButton::Primary) if session.fresh => terminal = None,
                _ => {}
            }
        }
        if invalid {
            terminal = Some(true);
        }
        if session.fresh {
            session.fresh = false;
        }
        if let Some(cancelled) = terminal {
            commands.queue(move |world: &mut World| finish(world, root, cancelled));
        }
    }
}

pub(crate) fn ensure_handle(
    world: &mut World,
    header: Entity,
    column: WidgetryTableColumnId,
) -> Result<(), BevyError> {
    if world.get::<Children>(header).is_some_and(|children| {
        children.iter().any(|entity| {
            world
                .get::<ResizeHandle>(entity)
                .is_some_and(|handle| handle.column == column)
        })
    }) {
        return Ok(());
    }
    let entity=spawn_scene(world,bsn! {
        template(move |_|Ok(ResizeHandle {column}))
        template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::ColResize)))
        Node {position_type:PositionType::Absolute,right:px(0),top:px(0),width:px(6),height:percent(100)}
        ZIndex(1)
    }).map_err(BevyError::error)?;
    required(world.get_entity_mut(header).ok())?.add_child(entity);
    Ok(())
}

fn handle(world: &World, root: Entity, target: Entity) -> Option<Entity> {
    let mut entity = target;
    let mut found = None;
    loop {
        if world.get::<InteractionDisabled>(entity).is_some() {
            return None;
        }
        if entity == root {
            return found.filter(|&target| {
                let Some(column) = world
                    .get::<ResizeHandle>(target)
                    .map(|handle| handle.column)
                else {
                    return false;
                };
                world
                    .get::<ChildOf>(target)
                    .and_then(|parent| {
                        world.get::<crate::WidgetryTableColumnHeader>(parent.parent())
                    })
                    .is_some_and(|header| header.column == column)
            });
        }
        if world.get::<WidgetryTableState>(entity).is_some() {
            return None;
        }
        if world.get::<ResizeHandle>(entity).is_some() {
            found = Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

pub(crate) fn active_column(world: &World, root: Entity) -> Option<WidgetryTableColumnId> {
    world
        .get::<ResizeSession>(root)
        .map(|session| session.column)
}

fn finish(world: &mut World, root: Entity, cancelled: bool) {
    if let Ok(mut entity) = world.get_entity_mut(root)
        && let Some(session) = entity.take::<ResizeSession>()
    {
        entity.remove::<CursorLease>();
        world.trigger(WidgetryTableEvent {
            entity: root,
            kind: if cancelled {
                WidgetryTableEventKind::ColumnResizeCancel(session.column)
            } else {
                WidgetryTableEventKind::ColumnResizeEnd(session.column)
            },
        });
        // Termination observers may queue Model changes or despawn the root.
        // Flush those effects before the caller continues consuming state.
        world.flush();
    }
}

pub(crate) fn cancel(world: &mut World, root: Entity) {
    finish(world, root, true);
}

pub(crate) fn sync<T: Send + Sync + 'static>(world: &mut World, root: Entity, source: Entity) {
    let Some(session) = world.get::<ResizeSession>(root).cloned() else {
        return;
    };
    if world.get::<InteractionDisabled>(root).is_some()
        || world
            .get::<WidgetryTableModel<T>>(source)
            .is_none_or(|model| model.column_index(session.column).is_none())
        || handle(world, root, session.handle) != Some(session.handle)
    {
        cancel(world, root);
    }
}

pub(crate) fn on_disabled_added<T: Send + Sync + 'static>(
    event: On<Add<InteractionDisabled>>,
    views: Query<(), With<WidgetryTable<T>>>,
    states: Query<(), With<WidgetryTableState>>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let Some(root) = std::iter::once(event.entity)
        .chain(parents.iter_ancestors(event.entity))
        .find(|&entity| states.contains(entity))
        .filter(|&entity| views.contains(entity))
    else {
        return;
    };
    commands.queue(move |world: &mut World| {
        if let Some(view) = world.get::<WidgetryTable<T>>(root) {
            sync::<T>(world, root, view.source());
        }
    });
}

pub(crate) fn on_start<T: Send + Sync + 'static>(
    mut event: On<PointerDragStart>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if event.button != PointerButton::Primary || !views.contains(root) {
        return;
    }
    let target = event.original_event_target();
    let pointer = event.pointer.id;
    let context = event.pointer.location();
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        start::<T>(world, root, target, pointer, context)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table resize start 失败"))
    });
}

fn start<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    target: Entity,
    pointer: PointerId,
    context: Location,
) -> Result<(), BevyError> {
    if world.get::<WidgetryTable<T>>(root).is_none()
        || world.get::<InteractionDisabled>(root).is_some()
        || world.get::<ResizeSession>(root).is_some()
    {
        return Ok(());
    }
    let Some(target) = handle(world, root, target) else {
        return Ok(());
    };
    let column = required(world.get::<ResizeHandle>(target))?.column;
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    if required(world.get::<WidgetryTableModel<T>>(source))?
        .column_index(column)
        .is_none()
    {
        return Ok(());
    }
    let geometry = required(world.get::<TableGeometry>(root))?;
    let width = required(geometry.columns.iter().find(|item| item.id == column))?.width;
    // Winit already converts Pointer distance to window logical pixels.
    // Divide only by the additional UiScale because applying native DPI again would shorten the drag distance.
    let inverse_ui_scale = world
        .get_resource::<UiScale>()
        .map_or(1.0, |scale| 1.0 / scale.0);
    if let Some(lease) = CursorLease::dragging(
        pointer,
        &context,
        PointerButton::Primary,
        bevy::window::SystemCursorIcon::ColResize,
    ) {
        world.entity_mut(root).insert(lease);
    }
    world.entity_mut(root).insert(ResizeSession {
        handle: target,
        column,
        pointer,
        target: context.target,
        fresh: true,
        start_width: width,
        width,
        inverse_ui_scale,
    });
    world.trigger(WidgetryTableEvent {
        entity: root,
        kind: WidgetryTableEventKind::ColumnResizeStart(column),
    });
    Ok(())
}

fn apply<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    target: Entity,
    pointer: PointerId,
    distance: Vec2,
    context: &NormalizedRenderTarget,
) -> Result<(), BevyError> {
    let Some(mut session) = world.get::<ResizeSession>(root).cloned() else {
        return Ok(());
    };
    if session.pointer != pointer
        || session.target != *context
        || handle(world, root, target) != Some(session.handle)
        || !distance.is_finite()
    {
        return Ok(());
    }
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    sync::<T>(world, root, source);
    if world.get::<ResizeSession>(root).is_none() {
        return Ok(());
    }
    let minimum = required(world.get::<WidgetryTableLayout>(root))?.min_column_width;
    let width = (session.start_width as f64 + distance.x as f64 * session.inverse_ui_scale as f64)
        .max(minimum as f64);
    if !width.is_finite() || width > f32::MAX as f64 {
        return Ok(());
    }
    let width = width as f32;
    if session.width == width {
        return Ok(());
    }
    required(world.get_mut::<WidgetryTableLayout>(root))?
        .columns
        .insert(session.column, WidgetryTableColumnWidth::Fixed(width));
    session.width = width;
    let column = session.column;
    world.entity_mut(root).insert(session);
    world.trigger(WidgetryTableEvent {
        entity: root,
        kind: WidgetryTableEventKind::ColumnResized { column, width },
    });
    Ok(())
}

pub(crate) fn on_drag<T: Send + Sync + 'static>(
    mut event: On<PointerDrag>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if event.button != PointerButton::Primary || !views.contains(root) {
        return;
    }
    let target = event.original_event_target();
    let pointer = event.pointer.id;
    let context = event.pointer.target.clone();
    let distance = event.distance;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        apply::<T>(world, root, target, pointer, distance, &context)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table resize drag 失败"))
    });
}

pub(crate) fn on_end<T: Send + Sync + 'static>(
    mut event: On<PointerDragEnd>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if event.button != PointerButton::Primary || !views.contains(root) {
        return;
    }
    let target = event.original_event_target();
    let pointer = event.pointer.id;
    let context = event.pointer.target.clone();
    let distance = event.distance;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        let accepted = world.get::<ResizeSession>(root).is_some_and(|session| {
            session.pointer == pointer
                && session.target == context
                && handle(world, root, target) == Some(session.handle)
        });
        if !accepted {
            return Ok(());
        }
        apply::<T>(world, root, target, pointer, distance, &context)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table resize end 失败"))?;
        // Final width observers may queue changes that interrupt the gesture, so apply them before deciding termination.
        world.flush();
        if let Some(view) = world.get::<WidgetryTable<T>>(root) {
            sync::<T>(world, root, view.source());
        }
        finish(world, root, false);
        Ok(())
    });
}

pub(crate) fn on_cancel<T: Send + Sync + 'static>(
    event: On<PointerCancel>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if !views.contains(root) {
        return;
    }
    let pointer = event.pointer.id;
    let context = event.pointer.target.clone();
    let target = event.original_event_target();
    commands.queue(move |world: &mut World| {
        if world.get::<ResizeSession>(root).is_some_and(|session| {
            session.pointer == pointer
                && session.target == context
                && handle(world, root, target) == Some(session.handle)
        }) {
            cancel(world, root);
        }
    });
}

pub(crate) fn set_width<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    column: WidgetryTableColumnId,
    width: f32,
) -> Result<bool, BevyError> {
    if !width.is_finite() || width <= 0.0 {
        return Err(BevyError::error(
            "Table Column width must be finite and positive",
        ));
    }
    let source = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table width requires a matching live View"))?
        .source();
    crate::view_state::refresh::<T>(world, root)?;
    let model = world
        .get::<WidgetryTableModel<T>>(source)
        .ok_or_else(|| BevyError::error("Table width requires a matching live Model source"))?;
    if model.column_index(column).is_none() {
        return Err(BevyError::error(
            "Table width contains stale source-local Column identity",
        ));
    }
    let layout = required(world.get::<WidgetryTableLayout>(root))?;
    let body = required(world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|&entity| world.get::<crate::WidgetryTableBody>(entity).is_some())
    }))?;
    required(world.get::<ComputedNode>(body))?;
    let viewport = crate::projection::viewport_width(world, root, layout)?;
    let ids = (0..model.column_count())
        .map(|index| required(model.column_id(index)))
        .collect::<Result<Vec<_>, _>>()?;
    let geometry = layout.resolve(
        ids,
        required(world.get::<crate::WidgetryTableViewState>(root))?.len(),
        viewport,
    )?;
    let old = required(geometry.columns.iter().find(|item| item.id == column))?.width;
    let width = width.max(layout.min_column_width);
    if old == width {
        return Ok(false);
    }
    // Validate the entire requested range because one valid f32 can still overflow accumulated geometry.
    let mut candidate = layout.clone();
    candidate
        .columns
        .insert(column, WidgetryTableColumnWidth::Fixed(width));
    let ids = geometry.columns.iter().map(|item| item.id).collect();
    candidate.resolve(
        ids,
        required(world.get::<crate::WidgetryTableViewState>(root))?.len(),
        viewport,
    )?;
    required(world.get_mut::<WidgetryTableLayout>(root))?
        .columns
        .insert(column, WidgetryTableColumnWidth::Fixed(width));
    if let Some(mut session) = world.get_mut::<ResizeSession>(root)
        && session.column == column
    {
        session.width = width;
    }
    world.trigger(WidgetryTableEvent {
        entity: root,
        kind: WidgetryTableEventKind::ColumnResized { column, width },
    });
    Ok(true)
}
