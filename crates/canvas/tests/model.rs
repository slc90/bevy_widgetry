#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::LogCapture;

#[test]
fn headless_crud_preserves_multiple_shapes_and_document_ownership() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        size: Vec2::new(120.0, 80.0),
    };
    let paint = CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            color: Color::BLACK,
            width: 10.0,
        }),
    };
    let a = CanvasDocument::create_rectangle(
        &mut world,
        document,
        rectangle,
        Transform::from_xyz(-50.0, -30.0, 0.0),
        paint,
    )
    .unwrap();
    let b = CanvasDocument::create_rectangle(
        &mut world,
        document,
        rectangle,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    assert_ne!(a, b);
    assert_eq!(world.get::<ChildOf>(a).unwrap().parent(), document);
    assert_eq!(*world.get::<CanvasPaint>(a).unwrap(), paint);
    let transform =
        Transform::from_xyz(-100.0, 15.0, 0.0).with_rotation(Quat::from_rotation_z(0.5));
    CanvasDocument::update_rectangle(
        &mut world,
        document,
        a,
        CanvasRectangle {
            size: Vec2::new(10.0, 20.0),
        },
        transform,
        CanvasPaint {
            fill: Some(Color::NONE),
            stroke: None,
        },
    )
    .unwrap();
    assert_eq!(*world.get::<Transform>(a).unwrap(), transform);
    assert_eq!(world.get::<CanvasPaint>(a).unwrap().fill, Some(Color::NONE));
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::remove_shape(&mut world, other, a))
            .is_err()
    );
    assert!(world.entities().contains(a));
    CanvasDocument::remove_shape(&mut world, document, b).unwrap();
    assert!(world.entities().contains(document));
    world.despawn(document);
    assert!(!world.entities().contains(a));
    assert!(world.entities().contains(other));
}

#[test]
fn invalid_requests_preserve_existing_geometry_and_report_errors() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        size: Vec2::new(10.0, 20.0),
    };
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        rectangle,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let count = world.entities().count_spawned();
    let logs = LogCapture::default();
    for size in [
        Vec2::new(0.99, 1.0),
        Vec2::new(1.0, -1.0),
        Vec2::splat(f32::NAN),
        Vec2::splat(f32::INFINITY),
    ] {
        let invalid = CanvasRectangle { size };
        for update in [false, true] {
            let error = logs
                .run(|| {
                    if update {
                        CanvasDocument::update_rectangle(
                            &mut world,
                            document,
                            shape,
                            invalid,
                            Transform::default(),
                            CanvasPaint::default(),
                        )
                    } else {
                        CanvasDocument::create_rectangle(
                            &mut world,
                            document,
                            invalid,
                            Transform::default(),
                            CanvasPaint::default(),
                        )
                        .map(|_| ())
                    }
                })
                .unwrap_err();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            assert_eq!(
                *world
                    .get::<CanvasGeometry>(shape)
                    .and_then(CanvasGeometry::rectangle)
                    .unwrap(),
                rectangle
            );
            assert_eq!(world.entities().count_spawned(), count);
        }
    }
    let invalid_paint = CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            color: Color::WHITE,
            width: f32::NAN,
        }),
    };
    assert!(
        logs.run(|| CanvasDocument::update_rectangle(
            &mut world,
            document,
            shape,
            rectangle,
            Transform::default(),
            invalid_paint
        ))
        .is_err()
    );
    let invalid_transform = Transform::from_xyz(f32::NAN, 0.0, 0.0);
    assert!(
        logs.run(|| CanvasDocument::update_rectangle(
            &mut world,
            document,
            shape,
            rectangle,
            invalid_transform,
            CanvasPaint::default()
        ))
        .is_err()
    );
    assert_eq!(
        *world.get::<Transform>(shape).unwrap(),
        Transform::default()
    );
    assert_eq!(
        *world.get::<CanvasPaint>(shape).unwrap(),
        CanvasPaint::default()
    );
    let invalid_document = world.spawn_empty().id();
    assert!(
        logs.run(|| CanvasDocument::create_rectangle(
            &mut world,
            invalid_document,
            rectangle,
            Transform::default(),
            CanvasPaint::default()
        ))
        .is_err()
    );
    world.despawn(shape);
    assert!(
        logs.run(|| CanvasDocument::remove_shape(&mut world, document, shape))
            .is_err()
    );
    assert!(
        logs.records()
            .iter()
            .any(|record| record.target == "bevy_widgetry"
                && record.level == bevy::log::Level::ERROR)
    );
}

#[test]
fn bounds_include_centered_stroke_rotation_and_nonuniform_scale() {
    let rectangle = CanvasRectangle {
        size: Vec2::new(120.0, 80.0),
    };
    let paint = CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            color: Color::BLACK,
            width: 10.0,
        }),
    };
    let transform = Transform::from_xyz(-200.0, -100.0, 0.0).with_scale(Vec3::new(2.0, 3.0, 1.0));
    let bounds = rectangle.bounds(&transform, &paint).unwrap();
    assert_eq!(bounds.min, Vec2::new(-330.0, -235.0));
    assert_eq!(bounds.max, Vec2::new(-70.0, 35.0));
    let transform = transform.with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4));
    let bounds = rectangle.bounds(&transform, &paint).unwrap();
    let half = 265.0 / 2.0_f32.sqrt();
    assert!((bounds.min - (Vec2::new(-200.0, -100.0) - Vec2::splat(half))).length() < 0.001);
    assert!((bounds.max - (Vec2::new(-200.0, -100.0) + Vec2::splat(half))).length() < 0.001);
    let corners = rectangle.corners(&Transform::from_xyz(-10.0, -20.0, 0.0));
    assert_eq!(
        corners,
        [
            Vec2::new(-70.0, -60.0),
            Vec2::new(50.0, -60.0),
            Vec2::new(50.0, 20.0),
            Vec2::new(-70.0, 20.0)
        ]
    );
    let boundary = CanvasRectangle { size: Vec2::ONE };
    assert!(
        boundary
            .bounds(&Transform::default(), &CanvasPaint::default())
            .is_ok()
    );
}

#[test]
fn geometry_validation_rejects_degenerate_transforms_and_overflow() {
    let rectangle = CanvasRectangle {
        size: Vec2::new(120.0, 80.0),
    };
    let paint = CanvasPaint::default();
    for transform in [
        Transform::from_scale(Vec3::new(0.0, 1.0, 1.0)),
        Transform::from_scale(Vec3::new(1.0, f32::INFINITY, 1.0)),
        Transform::from_rotation(Quat::from_xyzw(0.0, 0.0, 0.0, 0.0)),
        Transform::from_rotation(Quat::from_rotation_x(0.2)),
        Transform::from_rotation(Quat::from_xyzw(0.0, 0.0, f32::NAN, 1.0)),
        Transform::from_scale(Vec3::splat(f32::MAX)),
    ] {
        let error = rectangle.bounds(&transform, &paint).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    }
    for paint in [
        CanvasPaint {
            fill: Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0)),
            stroke: None,
        },
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                color: Color::WHITE,
                width: 0.0,
            }),
        },
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                color: Color::linear_rgba(0.0, 0.0, 0.0, f32::NAN),
                width: 1.0,
            }),
        },
    ] {
        assert!(rectangle.bounds(&Transform::default(), &paint).is_err());
    }
    let mirrored = Transform::from_scale(Vec3::new(-2.0, 3.0, 1.0));
    let bounds = rectangle.bounds(&mirrored, &paint).unwrap();
    assert_eq!(bounds.size(), Vec2::new(240.0, 240.0));
    assert_eq!(bounds.center(), Vec2::ZERO);
}
