use bevy::{
    app::App,
    camera::NormalizedRenderTarget,
    ecs::entity::Entity,
    math::Vec2,
    picking::{
        backend::HitData,
        events::{Cancel, Click, DragEnd, Pointer, Press, Release},
        pointer::{Location, PointerButton, PointerId},
    },
};
use std::time::Duration;

/// 发送主 pointer 的 press event 并执行 observer 排队的 command，不推进时间。
pub fn press(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
}

/// 构造无真实 render target 的鼠标主键 press event，供 headless 测试触发 observer。
pub fn primary_press(entity: Entity) -> Pointer<Press> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Press {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            count: 1,
        },
        entity,
    )
}

/// 发送主 pointer 的 release event 并执行 observer 排队的 command，不推进时间。
pub fn release(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().flush();
}

/// 构造与主键 press 配对的 release event，供测试结束 press 序列。
pub fn primary_release(entity: Entity) -> Pointer<Release> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Release {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
        entity,
    )
}

/// 发送 pointer cancel event 并执行 observer 排队的 command，不推进时间。
pub fn cancel(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_cancel(entity));
    app.world_mut().flush();
}

/// 构造鼠标 pointer cancel event，供测试中断未完成的交互。
pub fn primary_cancel(entity: Entity) -> Pointer<Cancel> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Cancel {
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
        entity,
    )
}

/// 发送主 pointer 的 drag end event 并执行 observer 排队的 command，不推进时间。
pub fn drag_end(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_drag_end(entity));
    app.world_mut().flush();
}

/// 构造零位移的主键 drag end event，供测试取消持续 press。
pub fn primary_drag_end(entity: Entity) -> Pointer<DragEnd> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        DragEnd {
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        },
        entity,
    )
}

/// 构造零持续时间的鼠标主键 click，供测试验证 Widget hierarchy 内外的 click。
pub fn primary_click(entity: Entity) -> Pointer<Click> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Click {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            duration: Duration::ZERO,
            count: 1,
        },
        entity,
    )
}
