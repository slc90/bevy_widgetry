#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::LogCapture;

fn fixture() -> (World, Entity, Entity) {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_path(
        &mut world,
        document,
        CanvasPath {
            commands: vec![CanvasPathCommand::MoveTo(Vec2::new(-100.0, -100.0))],
        },
        Transform::from_xyz(12.0, -6.0, 5.0)
            .with_rotation(Quat::from_rotation_z(0.5))
            .with_scale(Vec3::new(-2.0, 0.5, 0.0)),
        CanvasPaint {
            fill_rule: CanvasFillRule::EvenOdd,
            stroke: Some(CanvasStroke::default()),
            ..default()
        },
    )
    .unwrap();
    (world, document, shape)
}

fn commands(world: &World, shape: Entity) -> Vec<CanvasPathCommand> {
    let CanvasGeometry::Path(path) = world.get::<CanvasGeometry>(shape).unwrap() else {
        panic!("expected Path");
    };
    path.commands.clone()
}

#[test]
fn append_contours_keep_one_shape_and_start_independent_open_or_closed_subpaths() {
    use CanvasPathCommand::*;
    let (mut world, document, shape) = fixture();
    let count = world.entities().count_spawned();
    let transform = *world.get::<Transform>(shape).unwrap();
    let paint = *world.get::<CanvasPaint>(shape).unwrap();
    let mut expected = commands(&world, shape);
    let rect = Rect::new(-20.0, -10.0, 20.0, 10.0);
    CanvasDocument::append_rectangle(&mut world, document, shape, rect).unwrap();
    expected.extend([
        MoveTo(rect.min),
        LineTo(Vec2::new(rect.max.x, rect.min.y)),
        LineTo(rect.max),
        LineTo(Vec2::new(rect.min.x, rect.max.y)),
        Close,
    ]);
    assert_eq!(commands(&world, shape), expected);
    let from = Vec2::new(80.0, 30.0);
    let to = Vec2::new(90.0, 40.0);
    CanvasDocument::append_line(&mut world, document, shape, from, to).unwrap();
    expected.extend([MoveTo(from), LineTo(to)]);
    assert_eq!(commands(&world, shape), expected);
    let points = [
        Vec2::new(-30.0, 50.0),
        Vec2::new(-10.0, 60.0),
        Vec2::new(10.0, 50.0),
    ];
    CanvasDocument::append_polyline(&mut world, document, shape, &points).unwrap();
    expected.extend([MoveTo(points[0]), LineTo(points[1]), LineTo(points[2])]);
    assert_eq!(commands(&world, shape), expected);
    CanvasDocument::append_polygon(&mut world, document, shape, &points).unwrap();
    expected.extend([
        MoveTo(points[0]),
        LineTo(points[1]),
        LineTo(points[2]),
        Close,
    ]);
    assert_eq!(commands(&world, shape), expected);
    for (radii, circle) in [(Vec2::new(15.0, 8.0), false), (Vec2::splat(10.0), true)] {
        let center = Vec2::new(40.0, -20.0);
        if circle {
            CanvasDocument::append_circle(&mut world, document, shape, center, radii.x).unwrap();
        } else {
            CanvasDocument::append_ellipse(&mut world, document, shape, center, radii).unwrap();
        }
        let start = center + Vec2::X * radii.x;
        expected.extend([
            MoveTo(start),
            ArcTo {
                radii,
                rotation: 0.0,
                large_arc: false,
                sweep: true,
                to: center - Vec2::X * radii.x,
            },
            ArcTo {
                radii,
                rotation: 0.0,
                large_arc: false,
                sweep: true,
                to: start,
            },
            Close,
        ]);
        assert_eq!(commands(&world, shape), expected);
    }
    assert_eq!(world.entities().count_spawned(), count);
    assert!(world.get::<CanvasShape>(shape).is_some());
    assert_eq!(world.get::<ChildOf>(shape).unwrap().parent(), document);
    assert_eq!(*world.get::<Transform>(shape).unwrap(), transform);
    assert_eq!(*world.get::<CanvasPaint>(shape).unwrap(), paint);
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [shape]
    );
}

fn rejected(
    world: &mut World,
    document: Entity,
    shape: Entity,
    operation: impl FnOnce(&mut World) -> Result,
) {
    let original = commands(world, shape);
    let transform = *world.get::<Transform>(shape).unwrap();
    let paint = *world.get::<CanvasPaint>(shape).unwrap();
    let count = world.entities().count_spawned();
    let order = CanvasDocument::shape_order(world, document).unwrap();
    let logs = LogCapture::default();
    let error = logs.run(|| operation(world)).unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert!(
        logs.records()
            .iter()
            .any(|record| record.level == bevy::log::Level::ERROR
                && record.target == "bevy_widgetry")
    );
    assert_eq!(commands(world, shape), original);
    assert_eq!(*world.get::<Transform>(shape).unwrap(), transform);
    assert_eq!(*world.get::<CanvasPaint>(shape).unwrap(), paint);
    assert_eq!(world.entities().count_spawned(), count);
    assert_eq!(CanvasDocument::shape_order(world, document).unwrap(), order);
}

#[test]
fn invalid_contours_and_targets_are_rejected_without_partial_append() {
    let (mut world, document, shape) = fixture();
    let other = world.spawn(CanvasDocument).id();
    let rectangle = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(10.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let stale = world.spawn_empty().id();
    world.despawn(stale);
    for (doc, target) in [
        (other, shape),
        (stale, shape),
        (document, stale),
        (document, rectangle),
    ] {
        rejected(&mut world, document, shape, |w| {
            CanvasDocument::append_line(w, doc, target, Vec2::ZERO, Vec2::ONE)
        });
    }
    for rect in [
        Rect {
            min: Vec2::ONE,
            max: Vec2::ZERO,
        },
        Rect::default(),
        Rect {
            min: Vec2::ZERO,
            max: Vec2::splat(f32::INFINITY),
        },
    ] {
        rejected(&mut world, document, shape, |w| {
            CanvasDocument::append_rectangle(w, document, shape, rect)
        });
    }
    for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        rejected(&mut world, document, shape, |w| {
            CanvasDocument::append_circle(w, document, shape, Vec2::ZERO, radius)
        });
    }
    for (center, radii) in [
        (Vec2::ZERO, Vec2::new(1.0, -1.0)),
        (Vec2::splat(f32::NAN), Vec2::ONE),
        (Vec2::splat(f32::MAX), Vec2::splat(f32::MAX)),
        (Vec2::splat(1e30), Vec2::ONE),
    ] {
        rejected(&mut world, document, shape, |w| {
            CanvasDocument::append_ellipse(w, document, shape, center, radii)
        });
    }
    rejected(&mut world, document, shape, |w| {
        CanvasDocument::append_line(w, document, shape, Vec2::ZERO, Vec2::splat(f32::NAN))
    });
    for points in [
        vec![],
        vec![Vec2::ZERO],
        vec![Vec2::ZERO, Vec2::ONE, Vec2::splat(f32::INFINITY)],
    ] {
        rejected(&mut world, document, shape, |w| {
            CanvasDocument::append_polyline(w, document, shape, &points)
        });
    }
    for points in [
        vec![Vec2::ZERO, Vec2::ONE],
        vec![Vec2::ZERO, Vec2::ONE, Vec2::splat(f32::NAN)],
    ] {
        rejected(&mut world, document, shape, |w| {
            CanvasDocument::append_polygon(w, document, shape, &points)
        });
    }
    rejected(&mut world, document, shape, |w| {
        CanvasDocument::append_line(w, document, shape, Vec2::ZERO, Vec2::new(f32::MAX, 0.0))
    });
    world.entity_mut(shape).remove::<CanvasShape>();
    rejected(&mut world, document, shape, |w| {
        CanvasDocument::append_polygon(w, document, shape, &[Vec2::ZERO, Vec2::X, Vec2::Y])
    });
}

#[test]
fn small_contours_and_zero_length_line_remain_ordinary_editable_path_commands() {
    let (mut world, document, shape) = fixture();
    CanvasDocument::append_rectangle(&mut world, document, shape, Rect::new(0.0, 0.0, 0.1, 0.1))
        .unwrap();
    CanvasDocument::append_ellipse(
        &mut world,
        document,
        shape,
        Vec2::ZERO,
        Vec2::new(0.001, 0.002),
    )
    .unwrap();
    CanvasDocument::append_line(&mut world, document, shape, Vec2::ONE, Vec2::ONE).unwrap();
    let mut edited = commands(&world, shape);
    edited[2] = CanvasPathCommand::LineTo(Vec2::new(2.0, 3.0));
    let transform = *world.get::<Transform>(shape).unwrap();
    let paint = *world.get::<CanvasPaint>(shape).unwrap();
    CanvasDocument::update_path(
        &mut world,
        document,
        shape,
        CanvasPath {
            commands: edited.clone(),
        },
        transform,
        paint,
    )
    .unwrap();
    assert_eq!(commands(&world, shape), edited);
}

#[test]
fn rounded_ellipse_endpoints_cannot_silently_change_the_other_radius() {
    for radius in [0.1, 0.15] {
        let (mut world, document, shape) = fixture();
        rejected(&mut world, document, shape, |w| {
            CanvasDocument::append_ellipse(
                w,
                document,
                shape,
                Vec2::new(1_000_000.0, 0.0),
                Vec2::new(radius, 100.0),
            )
        });
    }
    let (mut world, document, shape) = fixture();
    CanvasDocument::append_ellipse(
        &mut world,
        document,
        shape,
        Vec2::new(1_000_000.0, 0.0),
        Vec2::new(0.125, 100.0),
    )
    .unwrap();
    let appended = commands(&world, shape).split_off(1);
    let bounds = CanvasPath { commands: appended }
        .bounds(&Transform::default(), &CanvasPaint::default())
        .unwrap();
    assert!(
        (bounds.min.y + 100.0).abs() < 0.001 && (bounds.max.y - 100.0).abs() < 0.001,
        "ellipse radii changed: {bounds:?}"
    );
    assert_eq!(bounds.min.x, 1_000_000.0 - 0.125);
    assert_eq!(bounds.max.x, 1_000_000.0 + 0.125);
}
