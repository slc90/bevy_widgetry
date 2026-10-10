use bevy::{
    app::App,
    camera::NormalizedRenderTarget,
    ecs::entity::Entity,
    math::Vec2,
    picking::{
        backend::HitData,
        events::{
            Pointer, PointerCancel, PointerClick, PointerDragEnd, PointerPress, PointerRelease,
        },
        pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    },
};
use std::time::Duration;

pub fn press(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
}

pub fn primary_press(entity: Entity) -> PointerPress {
    PointerPress {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        count: 1,
    }
}

pub fn release(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().flush();
}

pub fn primary_release(entity: Entity) -> PointerRelease {
    PointerRelease {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
    }
}

pub fn cancel(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_cancel(entity));
    app.world_mut().flush();
}

pub fn primary_cancel(entity: Entity) -> PointerCancel {
    PointerCancel {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
        ),
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
    }
}

pub fn drag_end(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_drag_end(entity));
    app.world_mut().flush();
}

pub fn primary_drag_end(entity: Entity) -> PointerDragEnd {
    PointerDragEnd {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        distance: Vec2::ZERO,
    }
}

pub fn primary_click(entity: Entity) -> PointerClick {
    PointerClick {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        duration: Duration::ZERO,
        count: 1,
    }
}

pub fn pointer_ids() -> [PointerId; 2] {
    [PointerId::Mouse, PointerId::Custom(Default::default())]
}

pub fn pointer_event<E: bevy::picking::events::PointerEvent>(
    pointer: PointerId,
    location: Location,
    target: Entity,
    event: impl FnOnce(Entity, Pointer) -> E,
) -> E {
    event(target, Pointer::new(pointer, location))
}

pub fn queue_pointer(app: &mut App, pointer: PointerId, location: Location, action: PointerAction) {
    app.world_mut()
        .write_message(PointerInput::new(pointer, location, action));
}
