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

pub fn press(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
}

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

pub fn release(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().flush();
}

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

pub fn cancel(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_cancel(entity));
    app.world_mut().flush();
}

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

pub fn drag_end(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_drag_end(entity));
    app.world_mut().flush();
}

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
