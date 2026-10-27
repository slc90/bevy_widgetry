#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::pointer_ids;
#[allow(dead_code)]
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;
use coordinates::at_document;

fn path() -> CanvasPath {
    use CanvasPathCommand::*;
    CanvasPath {
        commands: vec![
            MoveTo(Vec2::new(-40.0, -30.0)),
            LineTo(Vec2::new(-15.0, -30.0)),
            QuadTo {
                control: Vec2::new(0.0, -50.0),
                to: Vec2::new(20.0, -30.0),
            },
            CubicTo {
                control1: Vec2::new(45.0, -20.0),
                control2: Vec2::new(45.0, 10.0),
                to: Vec2::new(20.0, 20.0),
            },
            Close,
            MoveTo(Vec2::new(-40.0, 25.0)),
            ArcTo {
                radii: Vec2::splat(15.0),
                rotation: 0.0,
                large_arc: false,
                sweep: true,
                to: Vec2::new(-10.0, 25.0),
            },
        ],
    }
}

#[allow(clippy::panic)]
fn coordinate(command: &mut CanvasPathCommand, role: usize) -> &mut Vec2 {
    use CanvasPathCommand::*;
    match command {
        MoveTo(p) | LineTo(p) => p,
        QuadTo { control, to } => {
            if role == 0 {
                to
            } else {
                control
            }
        }
        CubicTo {
            control1,
            control2,
            to,
        } => match role {
            1 => control1,
            2 => control2,
            _ => to,
        },
        ArcTo { to, .. } => to,
        Close => panic!("Close has no independent anchor"),
    }
}

#[test]
fn node_drag_edits_only_the_target_local_coordinate_and_preserves_shared_shape_identity() {
    for id in pointer_ids() {
        for mirrored in [false, true] {
            for (command, role) in [
                (0, 0),
                (1, 0),
                (2, 1),
                (2, 0),
                (3, 1),
                (3, 2),
                (3, 0),
                (5, 0),
                (6, 0),
            ] {
                let (mut app, a, b, old, _, base) = fixture(
                    id,
                    if mirrored { 2.0 } else { 1.0 },
                    if mirrored { 1.25 } else { 1.0 },
                );
                let document = CanvasView::document(app.world(), a).unwrap().unwrap();
                CanvasDocument::remove_shape(app.world_mut(), document, old).unwrap();
                let transform = Transform::from_rotation(Quat::from_rotation_z(0.2))
                    .with_scale(Vec3::new(if mirrored { -0.8 } else { 0.8 }, 1.1, 1.0));
                let mut expected = path();
                let local = *coordinate(&mut expected.commands[command], role);
                let shape = CanvasDocument::create_path(
                    app.world_mut(),
                    document,
                    expected.clone(),
                    transform,
                    CanvasPaint::default(),
                )
                .unwrap();
                CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
                CanvasView::set_selection_in_world(app.world_mut(), b, Some(shape)).unwrap();
                CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::PathNodeEdit)
                    .unwrap();
                CanvasView::set_center_in_world(app.world_mut(), a, Vec2::new(4.0, -3.0)).unwrap();
                CanvasView::set_zoom_in_world(app.world_mut(), a, 0.8).unwrap();
                update(&mut app);
                let entities = app.world().entities().count_spawned();
                let handles = app
                    .world()
                    .resource::<Assets<Mesh>>()
                    .ids()
                    .collect::<Vec<_>>();
                let p = transform.transform_point(local.extend(0.0)).truncate();
                let mut at = at_document(&app, a, &base, p);
                input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
                input(
                    &mut app,
                    id,
                    &at,
                    PointerAction::Press(PointerButton::Primary),
                );
                assert!(
                    app.world().resource::<PointerCaptureMap>().is_captured(&id),
                    "node must capture"
                );
                let delta = Vec2::new(3.0, 4.0);
                let end = at_document(
                    &app,
                    a,
                    &base,
                    transform
                        .transform_point((local + delta).extend(0.0))
                        .truncate(),
                );
                let screen = end.position - at.position;
                moved(&mut app, id, &mut at, screen);
                *coordinate(&mut expected.commands[command], role) += delta;
                let CanvasGeometry::Path(actual) =
                    app.world().get::<CanvasGeometry>(shape).unwrap()
                else {
                    panic!("Path expected")
                };
                let mut actual = actual.clone();
                let wanted = *coordinate(&mut expected.commands[command], role);
                let edited = coordinate(&mut actual.commands[command], role);
                assert!(
                    edited.distance(wanted) < 0.001,
                    "target coordinate must move"
                );
                *edited = wanted;
                assert_eq!(
                    actual, expected,
                    "unrelated commands and coordinates must stay intact"
                );
                assert_eq!(app.world().get::<Transform>(shape), Some(&transform));
                assert_eq!(
                    app.world().get::<CanvasPaint>(shape),
                    Some(&CanvasPaint::default())
                );
                assert_eq!(
                    CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
                    [shape]
                );
                assert_eq!(
                    CanvasView::single_selection(app.world(), b).unwrap(),
                    Some(shape)
                );
                input(
                    &mut app,
                    id,
                    &at,
                    PointerAction::Release(PointerButton::Primary),
                );
                assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
                assert_eq!(app.world().entities().count_spawned(), entities);
                assert_eq!(
                    app.world()
                        .resource::<Assets<Mesh>>()
                        .ids()
                        .collect::<Vec<_>>(),
                    handles
                );
            }
        }
    }
}

#[test]
fn node_gestures_ignore_other_pointers_and_stop_on_lifecycle_changes_without_rollback() {
    for cause in 0..13 {
        let id = pointer_ids()[1];
        let other = pointer_ids()[0];
        let (mut app, a, b, old, window, base) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let shape = CanvasDocument::create_path(
            app.world_mut(),
            document,
            path(),
            Transform::default(),
            CanvasPaint::default(),
        )
        .unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
        CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::PathNodeEdit).unwrap();
        update(&mut app);
        let mut at = at_document(&app, a, &base, Vec2::new(0.0, -50.0));
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
        input(&mut app, other, &at, PointerAction::Cancel);
        assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
        moved(&mut app, id, &mut at, Vec2::new(7.0, 6.0));
        let saved = app.world().get::<CanvasGeometry>(shape).unwrap().clone();
        match cause {
            0 => input(&mut app, id, &at, PointerAction::Cancel),
            1 => app.world_mut().get_mut::<Window>(window).unwrap().focused = false,
            2 => {
                app.world_mut()
                    .resource_mut::<bevy::input_focus::InputFocus>()
                    .set(b, bevy::input_focus::FocusCause::Navigated);
            }
            3 => {
                app.world_mut().entity_mut(a).insert(Visibility::Hidden);
            }
            4 => {
                app.world_mut()
                    .entity_mut(a)
                    .insert(bevy::ui::InteractionDisabled);
            }
            5 => {
                CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
            }
            6 => {
                CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
            }
            7 => {
                CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Select).unwrap();
            }
            8 => {
                app.world_mut().despawn(a);
            }
            9 => app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None,
            10 => {
                CanvasDocument::remove_shape(app.world_mut(), document, shape).unwrap();
            }
            11 => {
                CanvasDocument::update_transform(
                    app.world_mut(),
                    document,
                    shape,
                    Transform::from_xyz(3.0, 2.0, 0.0),
                )
                .unwrap();
            }
            _ => {
                let mut new_path = path();
                new_path
                    .commands
                    .push(CanvasPathCommand::LineTo(Vec2::new(30.0, 40.0)));
                CanvasDocument::update_path(
                    app.world_mut(),
                    document,
                    shape,
                    new_path,
                    Transform::default(),
                    CanvasPaint::default(),
                )
                .unwrap();
            }
        }
        update(&mut app);
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        let after = app.world().get::<CanvasGeometry>(shape).cloned();
        if cause < 10 {
            assert_eq!(after.as_ref(), Some(&saved));
        }
        moved(&mut app, id, &mut at, Vec2::new(7.0, 6.0));
        assert_eq!(
            app.world().get::<CanvasGeometry>(shape),
            after.as_ref(),
            "cause {cause}"
        );
        assert_eq!(
            CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
            if cause == 10 {
                vec![old]
            } else {
                vec![old, shape]
            }
        );
    }
}

#[test]
fn node_mode_does_not_move_or_convert_parameter_shapes_or_edit_unselected_path_nodes() {
    let id = pointer_ids()[1];
    let (mut app, a, _, shape, _, base) = fixture(id, 1.0, 1.0);
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::PathNodeEdit).unwrap();
    let original = app.world().get::<CanvasGeometry>(shape).unwrap().clone();
    input(
        &mut app,
        id,
        &base,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        id,
        &base,
        PointerAction::Press(PointerButton::Primary),
    );
    let mut at = base.clone();
    moved(&mut app, id, &mut at, Vec2::new(20.0, 15.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(app.world().get::<CanvasGeometry>(shape), Some(&original));
    assert_eq!(
        app.world().get::<Transform>(shape),
        Some(&Transform::default())
    );
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|n| n.as_str() == "CanvasPathNodes")
            .count(),
        0
    );
}
