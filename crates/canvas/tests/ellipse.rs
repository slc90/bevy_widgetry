#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::LogCapture;

#[test]
fn typed_ellipse_crud_preserves_identity_and_rejects_partial_or_cross_kind_updates() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let ellipse = CanvasEllipse {
        radii: Vec2::new(0.05, 0.1),
    };
    let shape = CanvasDocument::create_ellipse(
        &mut world,
        document,
        ellipse,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    assert!(world.get::<CanvasShape>(shape).is_some());
    assert_eq!(
        world.get::<CanvasGeometry>(shape),
        Some(&CanvasGeometry::Ellipse(ellipse))
    );
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [shape]
    );
    let before = world.entities().count_spawned();
    let logs = LogCapture::default();
    for radii in [
        Vec2::ZERO,
        Vec2::new(-1.0, 1.0),
        Vec2::splat(f32::NAN),
        Vec2::splat(f32::INFINITY),
        Vec2::splat(f32::MAX),
    ] {
        assert!(
            logs.run(|| CanvasDocument::create_ellipse(
                &mut world,
                document,
                CanvasEllipse { radii },
                Transform::default(),
                CanvasPaint::default()
            ))
            .is_err()
        );
        assert!(
            logs.run(|| CanvasDocument::update_ellipse(
                &mut world,
                document,
                shape,
                CanvasEllipse { radii },
                Transform::from_xyz(10.0, 20.0, 30.0),
                CanvasPaint {
                    fill: None,
                    stroke: None,
                    ..Default::default()
                }
            ))
            .is_err()
        );
        assert_eq!(world.entities().count_spawned(), before);
        assert_eq!(
            world.get::<CanvasGeometry>(shape),
            Some(&CanvasGeometry::Ellipse(ellipse))
        );
        assert_eq!(
            *world.get::<Transform>(shape).unwrap(),
            Transform::default()
        );
        assert_eq!(
            *world.get::<CanvasPaint>(shape).unwrap(),
            CanvasPaint::default()
        );
    }
    let error = logs
        .run(|| {
            CanvasDocument::update_ellipse(
                &mut world,
                other,
                shape,
                ellipse,
                Transform::default(),
                CanvasPaint::default(),
            )
        })
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert!(
        logs.run(|| CanvasDocument::update_rectangle(
            &mut world,
            document,
            shape,
            CanvasRectangle {
                corner_radius: 0.0,
                size: Vec2::splat(20.0)
            },
            Transform::default(),
            CanvasPaint::default()
        ))
        .is_err()
    );
    let rectangle = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    assert!(
        logs.run(|| CanvasDocument::update_ellipse(
            &mut world,
            document,
            rectangle,
            ellipse,
            Transform::default(),
            CanvasPaint::default()
        ))
        .is_err()
    );
    let updated = CanvasEllipse {
        radii: Vec2::new(12.0, 25.0),
    };
    let transform = Transform::from_xyz(-20.0, 30.0, 5.0)
        .with_rotation(Quat::from_rotation_z(0.7))
        .with_scale(Vec3::new(-2.0, 0.5, 0.0));
    CanvasDocument::update_ellipse(
        &mut world,
        document,
        shape,
        updated,
        transform,
        CanvasPaint::default(),
    )
    .unwrap();
    assert_eq!(
        world.get::<CanvasGeometry>(shape),
        Some(&CanvasGeometry::Ellipse(updated))
    );
    assert_eq!(*world.get::<Transform>(shape).unwrap(), transform);
    CanvasDocument::remove_shape(&mut world, document, shape).unwrap();
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [rectangle]
    );
}

#[test]
fn ellipse_bounds_include_rotation_mirroring_stroke_and_small_radii() {
    let ellipse = CanvasEllipse {
        radii: Vec2::new(10.0, 20.0),
    };
    let paint = CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            color: Color::BLACK,
            width: 4.0,
            ..Default::default()
        }),
        ..Default::default()
    };
    let transform = Transform::from_xyz(-30.0, 10.0, 2.0).with_scale(Vec3::new(-2.0, 0.5, 0.0));
    let bounds = ellipse.bounds(&transform, &paint).unwrap();
    assert_eq!(bounds.min, Vec2::new(-54.0, -1.0));
    assert_eq!(bounds.max, Vec2::new(-6.0, 21.0));
    let rotated = transform.with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    let bounds = ellipse.bounds(&rotated, &paint).unwrap();
    assert!((bounds.min - Vec2::new(-41.0, -14.0)).length() < 0.0001);
    assert!((bounds.max - Vec2::new(-19.0, 34.0)).length() < 0.0001);
    for step in 0..128 {
        let direction = Vec2::from_angle(step as f32 * std::f32::consts::TAU / 128.0);
        let local = direction * ellipse.radii + (direction / ellipse.radii).normalize() * 2.0;
        let point = rotated.transform_point(local.extend(0.0)).truncate();
        assert!(point.cmpge(bounds.min - Vec2::splat(0.0001)).all());
        assert!(point.cmple(bounds.max + Vec2::splat(0.0001)).all());
    }
    assert!(
        CanvasEllipse {
            radii: Vec2::splat(0.001)
        }
        .validate(&Transform::default(), &CanvasPaint::default())
        .is_ok()
    );
    assert!(
        ellipse
            .validate(&Transform::from_scale(Vec3::new(0.0, 1.0, 1.0)), &paint)
            .is_err()
    );
    assert!(
        ellipse
            .validate(
                &Transform::from_rotation(Quat::from_rotation_x(0.2)),
                &paint
            )
            .is_err()
    );
}

#[test]
fn ellipse_png_without_renderer_fails_before_modifying_the_destination() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    CanvasDocument::create_ellipse(
        &mut world,
        document,
        CanvasEllipse::default(),
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"keep me").unwrap();
    let logs = LogCapture::default();
    assert!(matches!(
        logs.run(|| CanvasDocument::export_png(
            &mut world,
            document,
            file.path(),
            CanvasPngOptions::default()
        )),
        Err(CanvasExportError::RendererUnavailable)
    ));
    assert_eq!(std::fs::read(file.path()).unwrap(), b"keep me");
}
