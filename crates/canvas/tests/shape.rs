#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::LogCapture;

#[test]
fn rectangle_api_uses_one_authoritative_geometry_and_common_identity() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        corner_radius: 0.0,
        size: Vec2::splat(20.0),
    };
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        rectangle,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    assert!(world.get::<CanvasShape>(shape).is_some());
    assert_eq!(
        world.get::<CanvasGeometry>(shape),
        Some(&CanvasGeometry::Rectangle(rectangle))
    );
    let changed = CanvasRectangle {
        corner_radius: 0.0,
        size: Vec2::new(30.0, 40.0),
    };
    CanvasDocument::update_rectangle(
        &mut world,
        document,
        shape,
        changed,
        Transform::from_xyz(1.0, 2.0, 3.0),
        CanvasPaint::default(),
    )
    .unwrap();
    assert_eq!(
        world.get::<CanvasGeometry>(shape),
        Some(&CanvasGeometry::Rectangle(changed))
    );
    CanvasDocument::remove_shape(&mut world, document, shape).unwrap();
    assert!(!world.entities().contains(shape));
    assert!(world.entities().contains(document));
}

#[test]
fn incomplete_shape_and_foreign_identity_are_rejected_without_partial_writes() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        corner_radius: 0.0,
        size: Vec2::splat(20.0),
    };
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        rectangle,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let logs = LogCapture::default();
    for missing in [false, true] {
        if missing {
            world.entity_mut(shape).remove::<CanvasShape>();
        }
        let owner = if missing { document } else { other };
        let before = world.entities().count_spawned();
        let error = logs
            .run(|| {
                CanvasDocument::update_rectangle(
                    &mut world,
                    owner,
                    shape,
                    CanvasRectangle {
                        corner_radius: 0.0,
                        size: Vec2::splat(99.0),
                    },
                    Transform::from_xyz(9.0, 8.0, 7.0),
                    CanvasPaint {
                        fill: None,
                        stroke: None,
                        ..Default::default()
                    },
                )
            })
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(
            world.get::<CanvasGeometry>(shape),
            Some(&CanvasGeometry::Rectangle(rectangle))
        );
        assert_eq!(
            *world.get::<Transform>(shape).unwrap(),
            Transform::default()
        );
        assert_eq!(
            *world.get::<CanvasPaint>(shape).unwrap(),
            CanvasPaint::default()
        );
        assert!(
            logs.run(|| CanvasDocument::remove_shape(&mut world, owner, shape))
                .is_err()
        );
        assert_eq!(world.entities().count_spawned(), before);
    }
}
