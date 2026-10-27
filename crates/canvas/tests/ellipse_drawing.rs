#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{pointer_ids, queue_pointer};
mod common;
use common::*;

fn preview(app: &mut App) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "CanvasEllipsePreview")
        .map(|(entity, _)| entity)
}

#[test]
fn ellipse_preview_is_local_and_release_creates_one_typed_shape_on_top_in_four_directions() {
    for id in pointer_ids() {
        for delta in [
            Vec2::new(20.0, 15.0),
            Vec2::new(-20.0, 15.0),
            Vec2::new(20.0, -15.0),
            Vec2::new(-20.0, -15.0),
        ] {
            let (mut app, a, b, original, _, mut at) = fixture(id, 1.5, 1.0);
            let document = CanvasView::document(app.world(), a).unwrap().unwrap();
            let paint = CanvasPaint {
                fill: Some(Color::srgba(0.2, 0.5, 0.8, 0.5)),
                stroke: Some(CanvasStroke {
                    width: 2.0,
                    ..default()
                }),
                ..default()
            };
            CanvasView::set_selection_in_world(app.world_mut(), b, Some(original)).unwrap();
            CanvasView::set_zoom_in_world(app.world_mut(), a, 2.0).unwrap();
            CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Ellipse(paint)).unwrap();
            moved(&mut app, id, &mut at, Vec2::new(60.0, 30.0));
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            moved(&mut app, id, &mut at, delta * 2.0);
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            assert_eq!(
                CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
                [original]
            );
            let p = preview(&mut app).unwrap();
            scroll(&mut app, id, &at, 1.0);
            let preview_layer = app
                .world()
                .get::<bevy::camera::visibility::RenderLayers>(
                    app.world().get::<Children>(p).unwrap()[0],
                )
                .unwrap()
                .clone();
            let mut cameras = app
                .world_mut()
                .query::<(&Name, &bevy::camera::visibility::RenderLayers)>();
            assert_eq!(
                cameras
                    .iter(app.world())
                    .filter(|(n, l)| n.as_str() == "CanvasCamera" && **l == preview_layer)
                    .count(),
                1
            );
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            let selected = CanvasView::selection(app.world(), a).unwrap().unwrap();
            assert_eq!(
                CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
                [original, selected]
            );
            let CanvasGeometry::Ellipse(ellipse) =
                app.world().get::<CanvasGeometry>(selected).unwrap()
            else {
                panic!("must retain ellipse geometry")
            };
            assert!(ellipse.radii.distance(delta.abs() * 0.5) < 0.001);
            assert!(
                app.world()
                    .get::<Transform>(selected)
                    .unwrap()
                    .translation
                    .truncate()
                    .distance(Vec2::new(30.0, 15.0) + delta * 0.5)
                    < 0.001
            );
            assert_eq!(*app.world().get::<CanvasPaint>(selected).unwrap(), paint);
            assert_eq!(
                CanvasView::selection(app.world(), b).unwrap(),
                Some(original)
            );
            assert_eq!(
                CanvasView::mode(app.world(), a).unwrap(),
                CanvasMode::Select
            );
            assert!(preview(&mut app).is_none());
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn small_input_cancel_escape_tool_changes_and_view_invalidation_discard_the_ellipse_draft() {
    for cause in 0..12 {
        let id = pointer_ids()[1];
        let (mut app, a, b, original, window, mut at) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let initial_meshes = app.world().resource::<Assets<Mesh>>().len();
        CanvasView::set_mode_in_world(
            app.world_mut(),
            a,
            CanvasMode::Ellipse(CanvasPaint::default()),
        )
        .unwrap();
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        if cause != 0 {
            moved(
                &mut app,
                id,
                &mut at,
                if cause == 1 {
                    Vec2::splat(0.5)
                } else {
                    Vec2::new(25.0, 15.0)
                },
            );
        }
        match cause {
            0 | 1 => input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            ),
            2 => input(&mut app, id, &at, PointerAction::Cancel),
            3 => {
                bevy_widgetry_test_utils::press_key(&mut app, window, KeyCode::Escape);
            }
            4 => {
                CanvasView::set_mode_in_world(
                    app.world_mut(),
                    a,
                    CanvasMode::Rectangle(CanvasPaint::default()),
                )
                .unwrap();
            }
            5 => {
                app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
            }
            6 => {
                CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
            }
            7 => {
                app.world_mut().entity_mut(a).insert(Visibility::Hidden);
            }
            8 => {
                app.world_mut()
                    .entity_mut(a)
                    .insert(bevy::ui::InteractionDisabled);
            }
            9 => {
                app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None;
            }
            10 => {
                app.world_mut()
                    .resource_mut::<bevy::input_focus::InputFocus>()
                    .set(b, bevy::input_focus::FocusCause::Navigated);
            }
            _ => {
                app.world_mut().despawn(a);
            }
        }
        update(&mut app);
        assert!(
            !app.world().resource::<PointerCaptureMap>().is_captured(&id),
            "cause={cause}"
        );
        assert!(preview(&mut app).is_none(), "cause={cause}");
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(
            CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
            [original]
        );
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), initial_meshes);
    }
}

#[test]
fn ellipse_drawing_consumes_same_frame_motion_before_release_and_can_finish_outside_the_view() {
    for id in pointer_ids() {
        let (mut app, a, _, original, _, mut at) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        CanvasView::set_mode_in_world(
            app.world_mut(),
            a,
            CanvasMode::Ellipse(CanvasPaint::default()),
        )
        .unwrap();
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        at.position += Vec2::new(150.0, 100.0);
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Move {
                delta: Vec2::new(150.0, 100.0),
            },
        );
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Release(PointerButton::Primary),
        );
        update(&mut app);
        let shape = CanvasView::selection(app.world(), a).unwrap().unwrap();
        assert_eq!(
            CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
            [original, shape]
        );
        assert_eq!(
            app.world().get::<CanvasGeometry>(shape),
            Some(&CanvasGeometry::Ellipse(CanvasEllipse {
                radii: Vec2::new(75.0, 50.0)
            }))
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    }
}
