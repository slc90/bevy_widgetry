#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input_focus::InputFocus,
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{pointer_ids, queue_pointer};
mod common;
use common::*;

fn assert_position(app: &App, shape: Entity, expected: Vec2) {
    let actual = app
        .world()
        .get::<Transform>(shape)
        .unwrap()
        .translation
        .truncate();
    assert!(
        (actual - expected).length() < 0.001,
        "actual {actual:?}, expected {expected:?}"
    );
}

#[test]
fn first_press_only_selects_and_next_press_moves_with_grab_offset_at_current_zoom() {
    for id in pointer_ids() {
        for (scale, ui_scale, zoom) in [(1.0, 1.0, 0.5), (1.5, 1.0, 2.0), (2.0, 1.5, 1.0)] {
            let (mut app, a, b, shape, _, base) = fixture(id, scale, ui_scale);
            let document = CanvasView::document(app.world(), a).unwrap().unwrap();
            let original = Transform::default()
                .with_rotation(Quat::from_rotation_z(0.3))
                .with_scale(Vec3::new(-1.2, 0.8, 0.0));
            app.world_mut().entity_mut(shape).insert(original);
            CanvasView::set_zoom_in_world(app.world_mut(), a, zoom).unwrap();
            CanvasView::set_center_in_world(app.world_mut(), b, Vec2::new(10.0, 5.0)).unwrap();
            CanvasView::set_zoom_in_world(app.world_mut(), b, 0.75).unwrap();
            let other = CanvasDocument::create_rectangle(
                app.world_mut(),
                document,
                CanvasRectangle {
                    size: Vec2::splat(5.0),
                },
                Transform::from_xyz(-80.0, -30.0, 0.0),
                CanvasPaint::default(),
            )
            .unwrap();
            CanvasView::set_selection_in_world(app.world_mut(), b, Some(other)).unwrap();
            update(&mut app);
            let meshes = app
                .world_mut()
                .query::<(&Name, &Mesh2d)>()
                .iter(app.world())
                .filter(|(name, _)| ["CanvasFill", "CanvasStroke"].contains(&name.as_str()))
                .map(|(_, mesh)| mesh.0.id())
                .collect::<Vec<_>>();
            let mut at = base.clone();
            at.position += Vec2::new(3.0, 2.0) * zoom * ui_scale;
            moved(&mut app, id, &mut at, Vec2::ZERO);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert_eq!(CanvasView::selection(app.world(), a).unwrap(), Some(shape));
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            moved(&mut app, id, &mut at, Vec2::new(5.0, 2.0) * ui_scale);
            assert_position(&app, shape, Vec2::ZERO);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            at = base.clone();
            at.position += Vec2::new(3.0, 2.0) * zoom * ui_scale;
            moved(&mut app, id, &mut at, Vec2::ZERO);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            assert_position(&app, shape, Vec2::ZERO);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Space);
            moved(&mut app, id, &mut at, Vec2::new(40.0, 20.0) * ui_scale);
            assert_position(&app, shape, Vec2::new(40.0, 20.0) / zoom);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::Space);
            moved(&mut app, id, &mut at, Vec2::new(160.0, 60.0) * ui_scale);
            assert_position(&app, shape, Vec2::new(200.0, 80.0) / zoom);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            moved(&mut app, id, &mut at, Vec2::ONE);
            assert_position(&app, shape, Vec2::new(200.0, 80.0) / zoom);
            assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
            assert_eq!(
                CanvasView::center(app.world(), b).unwrap(),
                Vec2::new(10.0, 5.0)
            );
            assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 0.75);
            assert_eq!(CanvasView::selection(app.world(), b).unwrap(), Some(other));
            let transform = app.world().get::<Transform>(shape).unwrap();
            assert_eq!(transform.rotation, original.rotation);
            assert_eq!(transform.scale, original.scale);
            assert_eq!(
                app.world().get::<CanvasRectangle>(shape).unwrap().size,
                Vec2::splat(30.0)
            );
            assert_eq!(
                *app.world().get::<CanvasPaint>(shape).unwrap(),
                CanvasPaint::default()
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
                .filter(|(name, _)| name.as_str() == "CanvasShapeProxy")
                .filter(|(_, t)| {
                    t.translation
                        .truncate()
                        .distance(Vec2::new(200.0, 80.0) / zoom)
                        < 0.001
                })
                .count();
            assert_eq!(proxies, 2);
        }
    }
}

#[test]
fn moving_cancels_on_unavailable_view_or_target_and_does_not_resume() {
    for cause in 0..12 {
        let id = pointer_ids()[1];
        let (mut app, a, _, shape, window, mut at) = fixture(id, 1.0, 1.0);
        CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
        moved(&mut app, id, &mut at, Vec2::ZERO);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
        moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0));
        assert_position(&app, shape, Vec2::new(10.0, 5.0));
        match cause {
            0 => input(&mut app, id, &at, PointerAction::Cancel),
            1 => {
                app.world_mut().entity_mut(a).insert(Visibility::Hidden);
            }
            2 => {
                app.world_mut()
                    .entity_mut(a)
                    .insert(bevy::ui::InteractionDisabled);
            }
            3 => app.world_mut().get_mut::<Window>(window).unwrap().focused = false,
            4 => {
                app.world_mut().despawn(a);
            }
            5 => app.world_mut().resource_mut::<InputFocus>().clear(),
            6 => {
                let doc = CanvasView::document(app.world(), a).unwrap().unwrap();
                app.world_mut().despawn(doc);
            }
            7 => app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None,
            8 => app.world_mut().get_mut::<Node>(a).unwrap().width = px(0),
            9 => {
                app.world_mut().despawn(shape);
            }
            10 => {
                CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
            }
            _ => {
                CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
            }
        }
        update(&mut app);
        assert!(
            !app.world().resource::<PointerCaptureMap>().is_captured(&id),
            "cause {cause}"
        );
        moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0));
        if app.world().get::<Transform>(shape).is_some() {
            assert_position(&app, shape, Vec2::new(10.0, 5.0));
        }
    }
}

#[test]
fn moving_preserves_final_delta_before_terminal_input_and_ignores_secondary_release() {
    for id in pointer_ids() {
        for terminal in [
            PointerAction::Release(PointerButton::Primary),
            PointerAction::Cancel,
        ] {
            let (mut app, a, _, shape, _, mut at) = fixture(id, 1.0, 1.0);
            CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
            moved(&mut app, id, &mut at, Vec2::ZERO);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            at.position += Vec2::new(20.0, 10.0);
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Move {
                    delta: Vec2::new(20.0, 10.0),
                },
            );
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Release(PointerButton::Secondary),
            );
            update(&mut app);
            assert_position(&app, shape, Vec2::new(20.0, 10.0));
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            at.position += Vec2::new(10.0, 5.0);
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Move {
                    delta: Vec2::new(10.0, 5.0),
                },
            );
            queue_pointer(&mut app, id, at.clone(), terminal);
            update(&mut app);
            assert_position(&app, shape, Vec2::new(30.0, 15.0));
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn wheel_during_move_preserves_grab_and_uses_the_new_zoom_for_later_motion() {
    let id = pointer_ids()[1];
    let (mut app, a, _, shape, _, mut at) = fixture(id, 1.0, 1.0);
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    moved(&mut app, id, &mut at, Vec2::new(3.0, 2.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, id, &mut at, Vec2::new(20.0, 10.0));
    assert_position(&app, shape, Vec2::new(20.0, 10.0));
    scroll(&mut app, id, &at, 1.0);
    assert_position(&app, shape, Vec2::new(20.0, 10.0));
    let zoom = CanvasView::zoom(app.world(), a).unwrap();
    moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0));
    assert_position(
        &app,
        shape,
        Vec2::new(20.0, 10.0) + Vec2::new(10.0, 5.0) / zoom,
    );
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
}
