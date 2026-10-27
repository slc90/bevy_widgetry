#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput, NativeKey},
    },
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{pointer_ids, queue_key};
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;
use coordinates::at_document;

fn key(app: &mut App, window: Entity, code: KeyCode, state: ButtonState) {
    queue_key(
        app,
        KeyboardInput {
            key_code: code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        },
    );
    update(app);
}

fn rotation_point(rectangle: CanvasRectangle, t: Transform, paint: CanvasPaint, zoom: f32) -> Vec2 {
    let half = (rectangle.size.y + paint.stroke.map_or(0.0, |stroke| stroke.width)) * 0.5;
    let top = t.transform_point(Vec3::new(0.0, -half, 0.0)).truncate();
    top + (t.rotation * Vec3::new(0.0, -t.scale.y.signum(), 0.0)).truncate() * (24.0 / zoom)
}

fn angle(app: &App, shape: Entity, expected: f32) {
    let actual = app.world().get::<Transform>(shape).unwrap().rotation;
    assert!(
        actual.dot(Quat::from_rotation_z(expected)).abs() > 0.999999,
        "actual {actual:?} expected {expected}"
    );
}

#[test]
fn rotation_handle_changes_only_rotation_and_shares_the_document_without_rebuilding_geometry() {
    for id in pointer_ids() {
        for (scale, ui_scale, zoom, shape_scale) in [
            (1.0, 1.0, 0.5, Vec3::ONE),
            (1.5, 1.0, 2.0, Vec3::new(-1.2, 0.8, 0.0)),
            (2.0, 1.5, 1.0, Vec3::new(0.9, -1.1, 2.0)),
        ] {
            let (mut app, a, b, shape, _, base) = fixture(id, scale, ui_scale);
            let doc = CanvasView::document(app.world(), a).unwrap().unwrap();
            let rectangle = CanvasRectangle {
                corner_radius: 0.0,
                size: Vec2::new(60.0, 30.0),
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
            let transform = Transform::from_xyz(-5.0, 4.0, 3.0)
                .with_rotation(Quat::from_rotation_z(0.3))
                .with_scale(shape_scale);
            CanvasDocument::update_rectangle(
                app.world_mut(),
                doc,
                shape,
                rectangle,
                transform,
                paint,
            )
            .unwrap();
            CanvasView::set_zoom_in_world(app.world_mut(), a, zoom).unwrap();
            CanvasView::set_zoom_in_world(app.world_mut(), b, 0.75).unwrap();
            for view in [a, b] {
                CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
            }
            update(&mut app);
            let meshes = app
                .world_mut()
                .query::<(&Name, &Mesh2d)>()
                .iter(app.world())
                .filter(|(name, _)| ["CanvasFill", "CanvasStroke"].contains(&name.as_str()))
                .map(|(_, mesh)| mesh.0.id())
                .collect::<Vec<_>>();
            let point = rotation_point(rectangle, transform, paint, zoom);
            let mut at = at_document(&app, a, &base, point);
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            let vector = point - transform.translation.truncate();
            let target = transform.translation.truncate()
                + (Quat::from_rotation_z(0.4) * vector.extend(0.0)).truncate();
            let next = at_document(&app, a, &base, target);
            let delta = next.position - at.position;
            at = next;
            input(&mut app, id, &at, PointerAction::Move { delta });
            angle(&app, shape, 0.7);
            let edited = *app.world().get::<Transform>(shape).unwrap();
            assert_eq!(edited.translation, transform.translation);
            assert_eq!(edited.scale, transform.scale);
            assert_eq!(
                *app.world()
                    .get::<CanvasGeometry>(shape)
                    .and_then(CanvasGeometry::rectangle)
                    .unwrap(),
                rectangle
            );
            assert_eq!(*app.world().get::<CanvasPaint>(shape).unwrap(), paint);
            assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 0.75);
            assert_eq!(CanvasView::center(app.world(), b).unwrap(), Vec2::ZERO);
            assert_eq!(
                CanvasView::single_selection(app.world(), b).unwrap(),
                Some(shape)
            );
            let after = app
                .world_mut()
                .query::<(&Name, &Mesh2d)>()
                .iter(app.world())
                .filter(|(name, _)| ["CanvasFill", "CanvasStroke"].contains(&name.as_str()))
                .map(|(_, mesh)| mesh.0.id())
                .collect::<Vec<_>>();
            assert_eq!(meshes, after);
            let proxies = app
                .world_mut()
                .query::<(&Name, &Transform)>()
                .iter(app.world())
                .filter(|(name, t)| {
                    name.as_str() == "CanvasShapeProxy"
                        && t.rotation.dot(edited.rotation).abs() > 0.999999
                })
                .count();
            assert_eq!(proxies, 2);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn shift_press_and_release_snap_the_current_direction_even_without_pointer_motion() {
    let id = pointer_ids()[1];
    let (mut app, a, _, shape, window, base) = fixture(id, 1.0, 1.0);
    let original = Transform::default().with_rotation(Quat::from_rotation_z(0.1));
    app.world_mut().entity_mut(shape).insert(original);
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    update(&mut app);
    let point = rotation_point(
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(30.0),
        },
        original,
        CanvasPaint::default(),
        1.0,
    );
    let mut at = at_document(&app, a, &base, point);
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    let target = (Quat::from_rotation_z(0.21) * point.extend(0.0)).truncate();
    let next = at_document(&app, a, &base, target);
    let delta = next.position - at.position;
    at = next;
    input(&mut app, id, &at, PointerAction::Move { delta });
    angle(&app, shape, 0.31);
    for code in [KeyCode::ShiftLeft, KeyCode::ShiftRight] {
        key(&mut app, window, code, ButtonState::Pressed);
        angle(&app, shape, std::f32::consts::PI / 12.0);
        key(&mut app, window, code, ButtonState::Released);
        angle(&app, shape, 0.31);
    }
    scroll(&mut app, id, &at, 1.0);
    angle(&app, shape, 0.31);
    input(&mut app, id, &at, PointerAction::Cancel);
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
}

#[test]
fn free_rotation_crosses_half_turn_and_cancelled_targets_stop_updating() {
    let id = pointer_ids()[1];
    let (mut app, a, _, shape, _, base) = fixture(id, 1.0, 1.0);
    let original =
        Transform::default().with_rotation(Quat::from_rotation_z(170.0_f32.to_radians()));
    app.world_mut().entity_mut(shape).insert(original);
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    update(&mut app);
    let point = rotation_point(
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(30.0),
        },
        original,
        CanvasPaint::default(),
        1.0,
    );
    let mut at = at_document(&app, a, &base, point);
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    for degrees in [175.0_f32, 185.0, 195.0, 350.0, 355.0, 365.0] {
        let target =
            (Quat::from_rotation_z((degrees - 170.0).to_radians()) * point.extend(0.0)).truncate();
        let next = at_document(&app, a, &base, target);
        let delta = next.position - at.position;
        at = next;
        input(&mut app, id, &at, PointerAction::Move { delta });
        angle(&app, shape, degrees.to_radians());
    }
    CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
    update(&mut app);
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    moved(&mut app, id, &mut at, Vec2::new(20.0, 10.0));
    angle(&app, shape, 365.0_f32.to_radians());
}
