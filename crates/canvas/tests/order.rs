#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::LogCapture;

fn create(world: &mut World, document: Entity, z: f32) -> Entity {
    CanvasDocument::create_rectangle(
        world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::from_xyz(0.0, 0.0, z),
        CanvasPaint::default(),
    )
    .unwrap()
}

#[test]
fn document_has_one_total_order_with_atomic_reordering_and_removal() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let a = create(&mut world, document, 9000.0);
    let b = create(&mut world, document, -9000.0);
    let c = create(&mut world, document, 0.0);
    let foreign = create(&mut world, other, 0.0);
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [a, b, c]
    );
    for (shape, operation, expected, changed) in [
        (c, CanvasOrder::Raise, [a, b, c], false),
        (a, CanvasOrder::Lower, [a, b, c], false),
        (a, CanvasOrder::Raise, [b, a, c], true),
        (c, CanvasOrder::Lower, [b, c, a], true),
        (b, CanvasOrder::Front, [c, a, b], true),
        (b, CanvasOrder::Front, [c, a, b], false),
        (a, CanvasOrder::Back, [a, c, b], true),
        (a, CanvasOrder::Back, [a, c, b], false),
    ] {
        assert_eq!(
            CanvasDocument::reorder_shape(&mut world, document, shape, operation).unwrap(),
            changed
        );
        assert_eq!(
            CanvasDocument::shape_order(&mut world, document).unwrap(),
            expected
        );
    }
    let logs = LogCapture::default();
    for invalid in [foreign, document, Entity::PLACEHOLDER] {
        let error = logs
            .run(|| {
                CanvasDocument::reorder_shape(&mut world, document, invalid, CanvasOrder::Front)
            })
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(
            CanvasDocument::shape_order(&mut world, document).unwrap(),
            [a, c, b]
        );
    }
    assert_eq!(world.get::<Transform>(a).unwrap().translation.z, 9000.0);
    CanvasDocument::remove_shape(&mut world, document, c).unwrap();
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [a, b]
    );
    world.despawn(a);
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [b]
    );
    world.entity_mut(b).insert(ChildOf(other));
    assert!(
        CanvasDocument::shape_order(&mut world, document)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        CanvasDocument::shape_order(&mut world, other).unwrap(),
        [foreign]
    );
}
