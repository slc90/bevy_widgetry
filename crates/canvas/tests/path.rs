#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::LogCapture;

fn path() -> CanvasPath {
    use CanvasPathCommand::*;
    CanvasPath {
        commands: vec![
            MoveTo(Vec2::ZERO),
            LineTo(Vec2::new(20.0, 0.0)),
            QuadTo {
                control: Vec2::new(30.0, 10.0),
                to: Vec2::new(20.0, 20.0),
            },
            CubicTo {
                control1: Vec2::new(10.0, 30.0),
                control2: Vec2::new(0.0, 30.0),
                to: Vec2::new(0.0, 20.0),
            },
            Close,
            MoveTo(Vec2::new(40.0, 0.0)),
            ArcTo {
                radii: Vec2::splat(10.0),
                rotation: 0.3,
                large_arc: false,
                sweep: true,
                to: Vec2::new(60.0, 0.0),
            },
        ],
    }
}

#[test]
fn path_crud_retains_commands_identity_order_and_rejects_invalid_updates() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let original = path();
    let shape = CanvasDocument::create_path(
        &mut world,
        document,
        original.clone(),
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    assert_eq!(
        world.get::<CanvasGeometry>(shape),
        Some(&CanvasGeometry::Path(original.clone()))
    );
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [shape]
    );
    let transform = Transform::from_xyz(-20.0, 30.0, 7.0)
        .with_rotation(Quat::from_rotation_z(0.3))
        .with_scale(Vec3::new(-2.0, 0.5, 0.0));
    CanvasDocument::update_path(
        &mut world,
        document,
        shape,
        original.clone(),
        transform,
        CanvasPaint::default(),
    )
    .unwrap();
    let logs = LogCapture::default();
    let invalid = [
        vec![CanvasPathCommand::LineTo(Vec2::ONE)],
        vec![CanvasPathCommand::Close],
        vec![CanvasPathCommand::MoveTo(Vec2::splat(f32::NAN))],
        vec![
            CanvasPathCommand::MoveTo(Vec2::ZERO),
            CanvasPathCommand::ArcTo {
                radii: Vec2::new(-1.0, 1.0),
                rotation: 0.0,
                large_arc: false,
                sweep: true,
                to: Vec2::ONE,
            },
        ],
        vec![
            CanvasPathCommand::MoveTo(Vec2::ZERO),
            CanvasPathCommand::QuadTo {
                control: Vec2::splat(f32::INFINITY),
                to: Vec2::ONE,
            },
        ],
    ];
    for commands in invalid {
        let error = logs
            .run(|| {
                CanvasDocument::update_path(
                    &mut world,
                    document,
                    shape,
                    CanvasPath { commands },
                    Transform::default(),
                    CanvasPaint::default(),
                )
            })
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(
            world.get::<CanvasGeometry>(shape),
            Some(&CanvasGeometry::Path(original.clone()))
        );
        assert_eq!(*world.get::<Transform>(shape).unwrap(), transform);
    }
    assert!(
        logs.run(|| CanvasDocument::update_path(
            &mut world,
            other,
            shape,
            original,
            transform,
            CanvasPaint::default()
        ))
        .is_err()
    );
    CanvasDocument::remove_shape(&mut world, document, shape).unwrap();
    assert!(
        CanvasDocument::shape_order(&mut world, document)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn svg_arcs_preserve_flags_degeneracies_and_close_context() {
    use CanvasPathCommand::*;
    for large_arc in [false, true] {
        for sweep in [false, true] {
            let p = CanvasPath {
                commands: vec![
                    MoveTo(Vec2::new(10.0, 0.0)),
                    ArcTo {
                        radii: Vec2::new(10.0, 20.0),
                        rotation: 0.0,
                        large_arc,
                        sweep,
                        to: Vec2::new(0.0, 20.0),
                    },
                ],
            };
            let bounds = p
                .bounds(&Transform::default(), &CanvasPaint::default())
                .unwrap();
            assert!(
                bounds.min.x <= 0.0
                    && bounds.max.x >= 10.0
                    && bounds.min.y <= 0.0
                    && bounds.max.y >= 20.0
            );
            if large_arc {
                assert!(bounds.width() > 15.0 || bounds.height() > 30.0);
            }
            if !large_arc && sweep {
                assert!(bounds.min.x >= -0.001 && bounds.min.y >= -0.001);
            }
        }
    }
    let zero = CanvasPath {
        commands: vec![
            MoveTo(Vec2::ZERO),
            ArcTo {
                radii: Vec2::ZERO,
                rotation: 0.0,
                large_arc: true,
                sweep: false,
                to: Vec2::new(10.0, 20.0),
            },
            Close,
            LineTo(Vec2::new(-10.0, -20.0)),
            Close,
        ],
    };
    let bounds = zero
        .bounds(&Transform::default(), &CanvasPaint::default())
        .unwrap();
    assert_eq!(bounds.min, Vec2::new(-10.0, -20.0));
    assert_eq!(bounds.max, Vec2::new(10.0, 20.0));
    for radius in [1e-30, 1e30] {
        let p = CanvasPath {
            commands: vec![
                MoveTo(Vec2::ZERO),
                ArcTo {
                    radii: Vec2::splat(radius),
                    rotation: 0.7,
                    large_arc: false,
                    sweep: true,
                    to: Vec2::new(20.0, 10.0),
                },
            ],
        };
        assert!(
            p.validate(&Transform::default(), &CanvasPaint::default())
                .is_ok()
        );
        let bounds = p
            .bounds(&Transform::default(), &CanvasPaint::default())
            .unwrap();
        assert!(
            bounds.width() < 40.0 && bounds.height() < 40.0,
            "short corrected arc has spurious huge bounds: {bounds:?}"
        );
    }
}

#[test]
fn path_export_rejection_and_kind_guards_preserve_existing_data() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_path(
        &mut world,
        document,
        path(),
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::update_ellipse(
            &mut world,
            document,
            shape,
            CanvasEllipse::default(),
            Transform::default(),
            CanvasPaint::default()
        ))
        .is_err()
    );
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"preserve").unwrap();
    assert!(matches!(
        logs.run(|| CanvasDocument::export_png(
            &mut world,
            document,
            file.path(),
            CanvasPngOptions::default()
        )),
        Err(CanvasExportError::UnsupportedGeometry)
    ));
    assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
    let p = CanvasPath {
        commands: vec![
            CanvasPathCommand::MoveTo(Vec2::splat(-1e38)),
            CanvasPathCommand::LineTo(Vec2::splat(1e38)),
        ],
    };
    assert!(
        p.validate(
            &Transform::from_scale(Vec3::splat(4.0)),
            &CanvasPaint::default()
        )
        .is_err()
    );
}
