use crate::lease::{CursorLease, LeaseMode};
use bevy::{
    camera::NormalizedRenderTarget,
    ecs::entity::{EntityHashMap, EntityHashSet},
    math::FloatOrd,
    picking::{
        backend::PointerHits,
        cursor::{CursorIconPlugin, EntityCursor},
        hover::{HoverMap, PointerCaptureMap},
        pointer::{PointerAction, PointerButton, PointerId, PointerInput, PointerLocation},
    },
    prelude::*,
    ui::InteractionDisabled,
    window::{CursorIcon, CursorSystems, SystemCursorIcon, WindowClosed, WindowFocused},
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

pub struct WidgetryCursorPlugin;

#[derive(Resource)]
struct CursorWriterEnabled(bool);

impl Plugin for WidgetryCursorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CursorWriterEnabled(true))
            .add_message::<PointerInput>()
            .add_message::<PointerHits>()
            .add_message::<WindowFocused>()
            .add_message::<WindowClosed>()
            .add_systems(
                Last,
                coordinate
                    .before(CursorSystems::Update)
                    .run_if(writer_enabled),
            );
        widgetry_info!("WidgetryCursorPlugin 注册完成");
    }

    fn finish(&self, app: &mut App) {
        if app.is_plugin_added::<CursorIconPlugin>() {
            app.world_mut().resource_mut::<CursorWriterEnabled>().0 = false;
            app.add_systems(Startup, reject_duplicate_writer);
        }
    }
}

fn writer_enabled(enabled: Res<CursorWriterEnabled>) -> bool {
    enabled.0
}

fn reject_duplicate_writer() -> Result {
    widgetry_error!("WidgetryCursorPlugin 不可与 CursorIconPlugin 同时安装");
    Err(BevyError::error(
        "WidgetryCursorPlugin conflicts with CursorIconPlugin",
    ))
}

fn coordinate(
    mut inputs: MessageReader<PointerInput>,
    mut hits: MessageReader<PointerHits>,
    mut focused: MessageReader<WindowFocused>,
    mut closed: MessageReader<WindowClosed>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    hover: Option<Res<HoverMap>>,
    capture: Option<Res<PointerCaptureMap>>,
    nodes: Query<
        (
            Option<&ChildOf>,
            Option<&EntityCursor>,
            Has<InteractionDisabled>,
        ),
        Without<Window>,
    >,
    mut leases: Query<(Entity, &mut CursorLease)>,
    mut windows: Query<(Entity, &Window, Option<&mut CursorIcon>)>,
    mut commands: Commands,
    mut raw: Local<Vec<PointerInput>>,
    mut inactive: Local<EntityHashSet>,
    mut active: Local<EntityHashMap<(Entity, LeaseMode)>>,
) {
    raw.clear();
    raw.extend(
        inputs
            .read()
            .filter(|input| input.pointer_id == PointerId::Mouse)
            .cloned(),
    );
    inactive.clear();
    inactive.extend(
        focused
            .read()
            .filter(|event| !event.focused)
            .map(|event| event.window),
    );
    inactive.extend(closed.read().map(|event| event.window));
    active.clear();
    let mouse_pointer = pointers.iter().find(|(id, _)| **id == PointerId::Mouse);
    let target = mouse_pointer.and_then(|(_, pointer)| {
        let location = pointer.location.as_ref()?;
        let NormalizedRenderTarget::Window(window) = location.target else {
            return None;
        };
        location.position.is_finite().then_some(window.entity())
    });
    for (owner, mut lease) in &mut leases {
        let invalid = mouse_pointer.is_none()
            || windows.get(lease.window).is_err()
            || inactive.contains(&lease.window)
            || windows
                .get(lease.window)
                .is_ok_and(|(_, window, _)| !window.focused)
            || disabled_owner(owner, &nodes)
            || match lease.mode {
                LeaseMode::Dragging(_) => target != Some(lease.window),
                // native 操作会触发 PointerOut，不能将 location 缺失当作结束。
                LeaseMode::Native => target.is_some_and(|target| target != lease.window),
            };
        let mut ended = !lease.fresh
            && mouse
                .as_ref()
                .is_some_and(|mouse| mouse.just_released(mouse_button(lease.button)));
        for input in raw.iter() {
            if matches!(input.action, PointerAction::Cancel) {
                ended = true;
                continue;
            }
            let NormalizedRenderTarget::Window(window) = input.location.target else {
                continue;
            };
            if window.entity() != lease.window {
                continue;
            }
            match input.action {
                PointerAction::Release(button) if button == lease.button => ended = true,
                PointerAction::Press(button) if button == lease.button && lease.fresh => {
                    ended = false
                }
                // Windows modal loop 返回后可能没有 Release。
                // 下一次真实 Mouse 活动证明 App 已重新获得输入，恢复 hover 控制权。
                PointerAction::Move { .. }
                | PointerAction::Press(_)
                | PointerAction::Scroll { .. }
                    if !lease.fresh && matches!(lease.mode, LeaseMode::Native) =>
                {
                    ended = true
                }
                _ => {}
            }
        }
        lease.fresh = false;
        if invalid || ended {
            commands.entity(owner).remove::<CursorLease>();
            continue;
        }
        let replace =
            active
                .get(&lease.window)
                .is_none_or(|(previous, mode)| match (lease.mode, mode) {
                    (LeaseMode::Native, LeaseMode::Dragging(_)) => true,
                    (LeaseMode::Dragging(_), LeaseMode::Native) => false,
                    _ => owner.to_bits() < previous.to_bits(),
                });
        if replace {
            active.insert(lease.window, (owner, lease.mode));
        }
    }
    let hovered = hover.as_ref().and_then(|map| map.get(&PointerId::Mouse));
    let captured = capture
        .as_ref()
        .and_then(|map| map.get(&PointerId::Mouse))
        .map(|(entity, _)| entity);
    let mut nearest: Option<(Entity, f32, f32)> = None;
    for hit in hits.read().filter(|hit| hit.pointer == PointerId::Mouse) {
        for (entity, data) in &hit.picks {
            if !hovered.is_some_and(|hovered| hovered.contains_key(entity))
                || !nodes.contains(*entity)
            {
                continue;
            }
            if nearest.is_none_or(|(_, order, depth)| {
                FloatOrd(hit.order) > FloatOrd(order)
                    || FloatOrd(hit.order) == FloatOrd(order)
                        && FloatOrd(data.depth) < FloatOrd(depth)
            }) {
                nearest = Some((*entity, hit.order, data.depth));
            }
        }
    }
    let entity = captured.or(nearest.map(|(entity, _, _)| entity));
    let hover_cursor = entity
        .and_then(|entity| resolve(entity, &nodes))
        .unwrap_or_default();
    for (entity, window, current) in &mut windows {
        let next = match active.get(&entity).map(|(_, mode)| mode) {
            Some(LeaseMode::Native) => continue,
            Some(LeaseMode::Dragging(icon)) => CursorIcon::System(*icon),
            None if target == Some(entity) && window.focused && !inactive.contains(&entity) => {
                hover_cursor.clone()
            }
            None => CursorIcon::System(SystemCursorIcon::Default),
        };
        if let Some(mut current) = current {
            current.set_if_neq(next);
        } else {
            commands.entity(entity).insert(next);
        }
    }
}

fn disabled_owner(
    mut entity: Entity,
    nodes: &Query<
        (
            Option<&ChildOf>,
            Option<&EntityCursor>,
            Has<InteractionDisabled>,
        ),
        Without<Window>,
    >,
) -> bool {
    loop {
        let Ok((parent, _, disabled)) = nodes.get(entity) else {
            return true;
        };
        if disabled {
            return true;
        }
        let Some(parent) = parent else {
            return false;
        };
        entity = parent.parent();
    }
}

fn resolve(
    mut entity: Entity,
    nodes: &Query<
        (
            Option<&ChildOf>,
            Option<&EntityCursor>,
            Has<InteractionDisabled>,
        ),
        Without<Window>,
    >,
) -> Option<CursorIcon> {
    let mut disabled = false;
    let mut declaration = None;
    loop {
        let (parent, cursor, current_disabled) = nodes.get(entity).ok()?;
        disabled |= current_disabled;
        if declaration.is_none() {
            declaration = cursor.map(EntityCursor::to_cursor_icon);
        }
        let Some(parent) = parent else {
            return declaration.map(|cursor| {
                if disabled {
                    CursorIcon::System(SystemCursorIcon::NotAllowed)
                } else {
                    cursor
                }
            });
        };
        entity = parent.parent();
    }
}

fn mouse_button(button: PointerButton) -> MouseButton {
    match button {
        PointerButton::Primary => MouseButton::Left,
        PointerButton::Secondary => MouseButton::Right,
        PointerButton::Middle => MouseButton::Middle,
    }
}
