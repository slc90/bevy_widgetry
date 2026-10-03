use crate::layout::TableGeometry;
use crate::projection::required;
use crate::{
    WidgetryTable, WidgetryTableColumnId, WidgetryTableColumnWidth, WidgetryTableEvent,
    WidgetryTableEventKind, WidgetryTableLayout, WidgetryTableModel, WidgetryTableState,
};
use bevy::picking::{
    events::{Cancel, Drag, DragEnd, DragStart, Pointer},
    pointer::{PointerButton, PointerId},
};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_log::widgetry_error;

#[derive(Component)]
pub(crate) struct ResizeHandle {
    column: WidgetryTableColumnId,
}

#[derive(Component, Clone, Copy)]
struct ResizeSession {
    handle: Entity,
    column: WidgetryTableColumnId,
    pointer: PointerId,
    start_width: f32,
    width: f32,
    inverse_ui_scale: f32,
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
        world.trigger(WidgetryTableEvent {
            entity: root,
            kind: if cancelled {
                WidgetryTableEventKind::ColumnResizeCancel(session.column)
            } else {
                WidgetryTableEventKind::ColumnResizeEnd(session.column)
            },
        });
        // terminal observer 可排队修改 Model 或销毁 root。
        // 调用方继续消费前必须看见这些副作用。
        world.flush();
    }
}

pub(crate) fn cancel(world: &mut World, root: Entity) {
    finish(world, root, true);
}

pub(crate) fn sync<T: Send + Sync + 'static>(world: &mut World, root: Entity, source: Entity) {
    let Some(session) = world.get::<ResizeSession>(root).copied() else {
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
    event: On<Add, InteractionDisabled>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if !views.contains(root) {
        return;
    }
    commands.queue(move |world: &mut World| {
        if world.get::<WidgetryTable<T>>(root).is_some() {
            crate::projection::project_disabled(world, root);
            cancel(world, root);
        }
    });
}

pub(crate) fn on_disabled_removed<T: Send + Sync + 'static>(
    event: On<Remove, InteractionDisabled>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if !views.contains(root) {
        return;
    }
    commands.queue(move |world: &mut World| {
        if world.get::<WidgetryTable<T>>(root).is_some() {
            crate::projection::project_disabled(world, root);
        }
    });
}

pub(crate) fn on_start<T: Send + Sync + 'static>(
    mut event: On<Pointer<DragStart>>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if event.button != PointerButton::Primary || !views.contains(root) {
        return;
    }
    let target = event.original_event_target();
    let pointer = event.pointer_id;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        start::<T>(world, root, target, pointer)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table resize start 失败"))
    });
}

fn start<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    target: Entity,
    pointer: PointerId,
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
    // Winit 已把 Pointer distance 转成 window logical px。
    // 再除 native DPI 会缩短 drag 距离，因此这里只除额外 UiScale。
    let inverse_ui_scale = world
        .get_resource::<UiScale>()
        .map_or(1.0, |scale| 1.0 / scale.0);
    world.entity_mut(root).insert(ResizeSession {
        handle: target,
        column,
        pointer,
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
) -> Result<(), BevyError> {
    let Some(mut session) = world.get::<ResizeSession>(root).copied() else {
        return Ok(());
    };
    if session.pointer != pointer
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
    world.entity_mut(root).insert(session);
    world.trigger(WidgetryTableEvent {
        entity: root,
        kind: WidgetryTableEventKind::ColumnResized {
            column: session.column,
            width,
        },
    });
    Ok(())
}

pub(crate) fn on_drag<T: Send + Sync + 'static>(
    mut event: On<Pointer<Drag>>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if event.button != PointerButton::Primary || !views.contains(root) {
        return;
    }
    let target = event.original_event_target();
    let pointer = event.pointer_id;
    let distance = event.distance;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        apply::<T>(world, root, target, pointer, distance)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table resize drag 失败"))
    });
}

pub(crate) fn on_end<T: Send + Sync + 'static>(
    mut event: On<Pointer<DragEnd>>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if event.button != PointerButton::Primary || !views.contains(root) {
        return;
    }
    let target = event.original_event_target();
    let pointer = event.pointer_id;
    let distance = event.distance;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        let accepted = world.get::<ResizeSession>(root).is_some_and(|session| {
            session.pointer == pointer && handle(world, root, target) == Some(session.handle)
        });
        if !accepted {
            return Ok(());
        }
        apply::<T>(world, root, target, pointer, distance)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table resize end 失败"))?;
        // 最终 width observer 的排队修改也可能中断 gesture，必须先提交再判定 terminal。
        world.flush();
        if let Some(view) = world.get::<WidgetryTable<T>>(root) {
            sync::<T>(world, root, view.source());
        }
        finish(world, root, false);
        Ok(())
    });
}

pub(crate) fn on_cancel<T: Send + Sync + 'static>(
    event: On<Pointer<Cancel>>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if !views.contains(root) {
        return;
    }
    let pointer = event.pointer_id;
    let target = event.original_event_target();
    commands.queue(move |world: &mut World| {
        if world.get::<ResizeSession>(root).is_some_and(|session| {
            session.pointer == pointer && handle(world, root, target) == Some(session.handle)
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
    let computed = required(world.get::<ComputedNode>(body))?;
    let ids = (0..model.column_count())
        .map(|index| required(model.column_id(index)))
        .collect::<Result<Vec<_>, _>>()?;
    let geometry = layout.resolve(
        ids,
        model.row_count(),
        computed.size().x * computed.inverse_scale_factor(),
    )?;
    let old = required(geometry.columns.iter().find(|item| item.id == column))?.width;
    let width = width.max(layout.min_column_width);
    if old == width {
        return Ok(false);
    }
    // 同时验证请求后的完整范围，避免单个合法 f32 引起累计几何 overflow。
    let mut candidate = layout.clone();
    candidate
        .columns
        .insert(column, WidgetryTableColumnWidth::Fixed(width));
    let ids = geometry.columns.iter().map(|item| item.id).collect();
    candidate.resolve(
        ids,
        model.row_count(),
        computed.size().x * computed.inverse_scale_factor(),
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
