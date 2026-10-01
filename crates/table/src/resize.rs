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

/// 右侧 strip 属于Header shell，不属于renderer Content。
#[derive(Component)]
pub(crate) struct ResizeHandle {
    column: WidgetryTableColumnId,
}

/// root持有gesture，Content变化/Column删除可结束它；不依赖pointer始终命中handle。
#[derive(Component, Clone, Copy)]
struct ResizeSession {
    handle: Entity,
    column: WidgetryTableColumnId,
    pointer: PointerId,
    start_width: f32,
    width: f32,
    inverse_ui_scale: f32,
}

/// Header Content replacement后重建strip，否则保留原pointer target。
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

/// 沿当前hierarchy定位handle；遇到其他Table拒绝跨root输入。
fn handle(world: &World, root: Entity, target: Entity) -> Option<Entity> {
    let mut entity = target;
    let mut found = None;
    loop {
        if entity == root {
            return found;
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

/// 当前 gesture 的 Column 用于在 Header replacement 前判定 handle 的失效。
pub(crate) fn active_column(world: &World, root: Entity) -> Option<WidgetryTableColumnId> {
    world
        .get::<ResizeSession>(root)
        .map(|session| session.column)
}

/// 当前root仍存在时发出一次End；root despawn由ECS释放session而不通知。
pub(crate) fn finish(world: &mut World, root: Entity) {
    if let Ok(mut entity) = world.get_entity_mut(root)
        && let Some(session) = entity.take::<ResizeSession>()
    {
        world.trigger(WidgetryTableEvent {
            entity: root,
            kind: WidgetryTableEventKind::ColumnResizeEnd(session.column),
        });
        // End observer 可排队修改 Model 或销毁 root；调用方继续消费前必须看见这些副作用。
        world.flush();
    }
}

/// Model/Content lifecycle或disabled中断一次gesture，保留最后width。
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
        finish(world, root);
    }
}

/// disabled立即覆盖picking并结束gesture，后续reconcile也处理新增Content。
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
            finish(world, root);
        }
    });
}

/// 恢复时精确还原原Pickable，不清空logical state。
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

/// primary DragStart只有命中owned handle且没有现有gesture时接受。
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

/// 起始width取当前实际求解结果，flexible resize随后只修改这个View的Fixed width。
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
    // Winit 已将 Pointer position 转为 window logical px，只消除额外的 UiScale。
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

/// 忽略非有限delta及不同pointer，累计distance始终相对于本次gesture起点。
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

/// 当次Drag提交width，事件使用当前ColumnId而不是physical index。
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

/// End应用最终distance后结束；重复End或其他pointer不能结束当前gesture。
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
        finish(world, root);
        Ok(())
    });
}

/// Cancel保留最后width，发出一次End并解除gesture ownership。
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
            finish(world, root);
        }
    });
}
