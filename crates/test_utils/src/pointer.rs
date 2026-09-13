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

/// 发送主指针按下事件并执行 observer 排队命令，不推进时间。
pub fn press(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
}

/// 构造无真实渲染目标的鼠标主键按下事件，供无窗口测试触发 observer。
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

/// 发送主指针释放事件并执行 observer 排队命令，不推进时间。
pub fn release(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().flush();
}

/// 构造与主键按下配对的释放事件，供测试结束按压序列。
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

/// 发送指针取消事件并执行 observer 排队命令，不推进时间。
pub fn cancel(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_cancel(entity));
    app.world_mut().flush();
}

/// 构造鼠标指针取消事件，供测试中断未完成的交互。
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

/// 发送主指针拖动结束事件并执行 observer 排队命令，不推进时间。
pub fn drag_end(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_drag_end(entity));
    app.world_mut().flush();
}

/// 构造零位移的主键拖动结束事件，供测试取消持续按压。
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

/// 构造零持续时间的鼠标主键点击，供测试验证控件层级内外的点击。
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
