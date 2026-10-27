#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::pointer_ids;
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;
use coordinates::at_document;

const AXES: [Vec2; 8] = [
    Vec2::new(-1.0, -1.0),
    Vec2::new(0.0, -1.0),
    Vec2::new(1.0, -1.0),
    Vec2::new(1.0, 0.0),
    Vec2::ONE,
    Vec2::new(0.0, 1.0),
    Vec2::new(-1.0, 1.0),
    Vec2::new(-1.0, 0.0),
];

fn near(actual: Vec2, expected: Vec2) {
    assert!(
        actual.distance(expected) < 0.002,
        "actual={actual:?}, expected={expected:?}"
    );
}

#[test]
fn all_eight_handles_scale_in_local_axes_and_keep_the_opposite_edge_or_corner_fixed() {
    for id in pointer_ids() {
        for (scale, ui_scale, zoom, shape_scale) in [
            (1.0, 1.0, 0.1, Vec3::ONE),
            (1.0, 1.0, 0.5, Vec3::ONE),
            (1.5, 1.0, 2.0, Vec3::new(-1.2, 0.8, 0.0)),
            (2.0, 1.5, 1.0, Vec3::new(0.9, -1.1, 2.0)),
            (2.0, 1.5, 10.0, Vec3::new(-1.2, 0.8, 0.0)),
        ] {
            for axes in AXES {
                let (mut app, a, b, shape, _, base) = fixture(id, scale, ui_scale);
                let document = CanvasView::document(app.world(), a).unwrap().unwrap();
                let rectangle = CanvasRectangle {
                    corner_radius: 0.0,
                    size: Vec2::new(60.0, 30.0),
                };
                let transform = Transform::from_xyz(-5.0, 4.0, 3.0)
                    .with_rotation(Quat::from_rotation_z(0.4))
                    .with_scale(shape_scale);
                let paint = CanvasPaint {
                    fill: Some(Color::srgba(0.3, 0.5, 0.8, 0.7)),
                    stroke: Some(CanvasStroke {
                        color: Color::BLACK,
                        width: 8.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                CanvasDocument::update_rectangle(
                    app.world_mut(),
                    document,
                    shape,
                    rectangle,
                    transform,
                    paint,
                )
                .unwrap();
                CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
                CanvasView::set_selection_in_world(app.world_mut(), b, Some(shape)).unwrap();
                CanvasView::set_zoom_in_world(app.world_mut(), a, zoom).unwrap();
                CanvasView::set_zoom_in_world(app.world_mut(), b, 0.75).unwrap();
                CanvasView::set_center_in_world(app.world_mut(), b, Vec2::new(7.0, 3.0)).unwrap();
                update(&mut app);
                let point = transform
                    .transform_point((axes * (rectangle.size + Vec2::splat(8.0)) * 0.5).extend(0.0))
                    .truncate();
                if zoom == 10.0 {
                    CanvasView::set_center_in_world(app.world_mut(), a, point).unwrap();
                    update(&mut app);
                }
                let mut at = at_document(&app, a, &base, point);
                input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
                input(
                    &mut app,
                    id,
                    &at,
                    PointerAction::Press(PointerButton::Primary),
                );
                assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
                let local_delta = axes * Vec2::new(10.0, 6.0);
                let delta = (transform.rotation
                    * (local_delta * transform.scale.truncate()).extend(0.0))
                .truncate();
                let next = at_document(&app, a, &base, point + delta);
                let window_delta = next.position - at.position;
                at = next;
                input(
                    &mut app,
                    id,
                    &at,
                    PointerAction::Move {
                        delta: window_delta,
                    },
                );
                let size = app
                    .world()
                    .get::<CanvasGeometry>(shape)
                    .and_then(CanvasGeometry::rectangle)
                    .unwrap()
                    .size;
                let edited = *app.world().get::<Transform>(shape).unwrap();
                let outer = rectangle.size + Vec2::splat(8.0);
                let factors = Vec2::ONE + axes.abs() * Vec2::new(10.0, 6.0) / outer;
                near(size, rectangle.size);
                near(
                    edited.scale.truncate(),
                    transform.scale.truncate() * factors,
                );
                near(
                    edited
                        .transform_point((-axes * outer * 0.5).extend(0.0))
                        .truncate(),
                    transform
                        .transform_point((-axes * outer * 0.5).extend(0.0))
                        .truncate(),
                );
                assert_eq!(edited.rotation, transform.rotation);
                assert_eq!(edited.translation.z, transform.translation.z);
                assert_eq!(*app.world().get::<CanvasPaint>(shape).unwrap(), paint);
                assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 0.75);
                assert_eq!(
                    CanvasView::center(app.world(), b).unwrap(),
                    Vec2::new(7.0, 3.0)
                );
                assert_eq!(
                    CanvasView::single_selection(app.world(), b).unwrap(),
                    Some(shape)
                );
                let bounds = app
                    .world()
                    .get::<CanvasGeometry>(shape)
                    .and_then(CanvasGeometry::rectangle)
                    .unwrap()
                    .bounds(&edited, &paint)
                    .unwrap();
                assert!(bounds.size().is_finite());
                input(
                    &mut app,
                    id,
                    &at,
                    PointerAction::Release(PointerButton::Primary),
                );
                assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
                let proxies = app
                    .world_mut()
                    .query::<(&Name, &Transform)>()
                    .iter(app.world())
                    .filter(|(name, t)| {
                        name.as_str() == "CanvasShapeProxy"
                            && t.translation
                                .truncate()
                                .distance(edited.translation.truncate())
                                < 0.001
                    })
                    .count();
                assert_eq!(proxies, 2);
            }
        }
    }
}

#[test]
fn handle_scaling_can_mirror_without_moving_the_fixed_corner() {
    let id = pointer_ids()[1];
    for axes in AXES {
        let (mut app, a, _, shape, _, base) = fixture(id, 1.0, 1.0);
        CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
        update(&mut app);
        let mut at = at_document(&app, a, &base, axes * 15.0);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        for distance in [40.0, 80.0, 35.0] {
            let next = at_document(&app, a, &base, axes * (15.0 - distance));
            let delta = next.position - at.position;
            at = next;
            input(&mut app, id, &at, PointerAction::Move { delta });
            let size = app
                .world()
                .get::<CanvasGeometry>(shape)
                .and_then(CanvasGeometry::rectangle)
                .unwrap()
                .size;
            near(size, Vec2::splat(30.0));
            let transform = app.world().get::<Transform>(shape).unwrap();
            near(
                transform.scale.truncate(),
                Vec2::ONE - axes.abs() * (distance / 30.0),
            );
            near(
                transform
                    .transform_point((-axes * 15.0).extend(0.0))
                    .truncate(),
                -axes * 15.0,
            );
        }
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    }
}

#[test]
fn wheel_preserves_scale_grab_and_cancellation_releases_the_gesture() {
    let id = pointer_ids()[1];
    let (mut app, a, _, shape, _, base) = fixture(id, 1.0, 1.0);
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    update(&mut app);
    let mut at = at_document(&app, a, &base, Vec2::splat(15.0));
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0));
    near(
        app.world()
            .get::<Transform>(shape)
            .unwrap()
            .scale
            .truncate(),
        Vec2::new(40.0, 35.0) / 30.0,
    );
    scroll(&mut app, id, &at, 1.0);
    near(
        app.world()
            .get::<Transform>(shape)
            .unwrap()
            .scale
            .truncate(),
        Vec2::new(40.0, 35.0) / 30.0,
    );
    moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0));
    let zoom = CanvasView::zoom(app.world(), a).unwrap();
    let expected = (Vec2::new(40.0, 35.0) + Vec2::new(10.0, 5.0) / zoom) / 30.0;
    near(
        app.world()
            .get::<Transform>(shape)
            .unwrap()
            .scale
            .truncate(),
        expected,
    );
    input(&mut app, id, &at, PointerAction::Cancel);
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    moved(&mut app, id, &mut at, Vec2::ONE);
    near(
        app.world()
            .get::<Transform>(shape)
            .unwrap()
            .scale
            .truncate(),
        expected,
    );
}
