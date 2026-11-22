#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    camera::visibility::RenderLayers,
    input::touch::{TouchInput, TouchPhase},
    picking::{PickingSettings, input::PointerInputSettings, pointer::*},
    prelude::*,
    window::WindowEvent,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{add_keyboard_dispatch, pointer_ids, press_key, queue_pointer};

#[allow(dead_code)]
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;

fn previews(app: &mut App, name: &str) -> usize {
    app.world_mut()
        .query::<&Name>()
        .iter(app.world())
        .filter(|entry| entry.as_str() == name)
        .count()
}

fn click(app: &mut App, id: PointerId, at: &Location) {
    input(app, id, at, PointerAction::Press(PointerButton::Primary));
    input(app, id, at, PointerAction::Release(PointerButton::Primary));
}

#[allow(clippy::panic)]
fn commands(app: &App, view: Entity) -> &[CanvasPathCommand] {
    let item = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    let Some(CanvasGeometry::Path(path)) = app.world().get(item) else {
        panic!("the committed item must be a Path");
    };
    &path.commands
}

fn paint() -> CanvasPaint {
    CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            width: 3.0,
            ..default()
        }),
        ..default()
    }
}

#[test]
fn line_release_creates_one_open_path_and_one_history_entry() {
    let id = pointer_ids()[1];
    let (mut app, a, b, original, _, mut at) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    CanvasDocument::clear_history(app.world_mut(), document).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Line(paint())).unwrap();
    moved(&mut app, id, &mut at, Vec2::new(40.0, 30.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    let delta = Vec2::new(-20.0, 0.0);
    moved(&mut app, id, &mut at, delta);
    assert_eq!(previews(&mut app, "CanvasLinePreview"), 1);
    assert_eq!(
        CanvasDocument::draw_order(app.world(), document).unwrap(),
        [original]
    );
    assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
    let directory = tempfile::tempdir().unwrap();
    let svg = directory.path().join("line.svg");
    CanvasDocument::export_svg(app.world_mut(), document, &svg, None).unwrap();
    assert!(!std::fs::read_to_string(&svg).unwrap().contains("<path"));

    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        commands(&app, a),
        [
            CanvasPathCommand::MoveTo(-delta * 0.5),
            CanvasPathCommand::LineTo(delta * 0.5)
        ]
    );
    assert_eq!(previews(&mut app, "CanvasLinePreview"), 0);
    assert_eq!(
        CanvasView::mode(app.world(), a).unwrap(),
        CanvasMode::Select
    );
    assert!(CanvasView::selection(app.world(), b).unwrap().is_empty());
    assert!(CanvasDocument::undo(app.world_mut(), document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(app.world(), document).unwrap(),
        [original]
    );
    assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());

    assert!(CanvasDocument::redo(app.world_mut(), document).unwrap());
    let shapes = CanvasDocument::draw_order(app.world(), document).unwrap();
    let line = *shapes.last().unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(line)).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::PathNodeEdit).unwrap();
    update(&mut app);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    let displacement = Vec2::new(8.0, -12.0);
    moved(&mut app, id, &mut at, displacement);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        commands(&app, a)[1],
        CanvasPathCommand::LineTo(delta * 0.5 + displacement)
    );
    CanvasDocument::export_svg(app.world_mut(), document, &svg, None).unwrap();
    assert!(std::fs::read_to_string(svg).unwrap().contains("<path"));
}

#[test]
fn parametric_drag_validates_parameters_and_commits_one_editable_path() {
    use CanvasPathTool::*;
    let id = pointer_ids()[1];
    let (mut app, view, _, original, _, mut at) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    let arc = |radii, rotation| Arc {
        radii,
        rotation,
        large_arc: true,
        sweep: false,
    };
    for tool in [
        RegularPolygon { sides: 2 },
        Star {
            points: 2,
            inner_ratio: 0.5,
        },
        Star {
            points: 3,
            inner_ratio: 0.0,
        },
        Star {
            points: 3,
            inner_ratio: 1.0,
        },
        Star {
            points: 3,
            inner_ratio: f32::NAN,
        },
        arc(Vec2::new(-1.0, 2.0), 0.0),
        arc(Vec2::ZERO, 0.0),
        arc(Vec2::ONE, f32::INFINITY),
    ] {
        assert!(
            CanvasView::set_mode_in_world(
                app.world_mut(),
                view,
                CanvasMode::Parametric(tool, paint())
            )
            .is_err()
        );
    }
    let radii = Vec2::new(35.0, 22.0);
    for tool in [
        RegularPolygon { sides: 3 },
        Star {
            points: 5,
            inner_ratio: 0.4,
        },
        arc(radii, 0.7),
    ] {
        let mode = CanvasMode::Parametric(tool, paint());
        CanvasDocument::clear_history(app.world_mut(), document).unwrap();
        let delta = Vec2::new(12.0, 16.0);
        for cancel in [true, false] {
            CanvasView::set_mode_in_world(app.world_mut(), view, mode).unwrap();
            moved(&mut app, id, &mut at, Vec2::new(1.0, 1.0));
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            moved(&mut app, id, &mut at, delta);
            assert_eq!(previews(&mut app, "CanvasParametricPreview"), 1);
            assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
            input(
                &mut app,
                id,
                &at,
                if cancel {
                    PointerAction::Cancel
                } else {
                    PointerAction::Release(PointerButton::Primary)
                },
            );
            assert_eq!(previews(&mut app, "CanvasParametricPreview"), 0);
            if cancel {
                assert_eq!(
                    CanvasDocument::draw_order(app.world(), document).unwrap(),
                    [original]
                );
                moved(&mut app, id, &mut at, -delta);
            }
        }
        let path = commands(&app, view);
        if matches!(tool, Arc { .. }) {
            assert_eq!(
                path,
                [
                    CanvasPathCommand::MoveTo(-delta * 0.5),
                    CanvasPathCommand::ArcTo {
                        radii,
                        rotation: 0.7,
                        large_arc: true,
                        sweep: false,
                        to: delta * 0.5
                    }
                ]
            );
        } else {
            let star = matches!(tool, Star { .. });
            let count = if star { 10 } else { 3 };
            assert_eq!(path.len(), count + 1);
            assert_eq!(path.last(), Some(&CanvasPathCommand::Close));
            for (i, command) in path[..count].iter().enumerate() {
                let direction = Vec2::from_angle(std::f32::consts::TAU * i as f32 / count as f32);
                let expected = direction.rotate(delta) * if star && i % 2 == 1 { 0.4 } else { 1.0 };
                let point = match command {
                    CanvasPathCommand::MoveTo(p) if i == 0 => p,
                    CanvasPathCommand::LineTo(p) if i > 0 => p,
                    _ => panic!("unexpected vertex"),
                };
                assert!(point.abs_diff_eq(expected, 0.001));
            }
        }
        assert!(CanvasDocument::undo(app.world_mut(), document).unwrap());
        assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
        assert!(CanvasDocument::redo(app.world_mut(), document).unwrap());
        let item = *CanvasDocument::draw_order(app.world(), document)
            .unwrap()
            .last()
            .unwrap();
        let Some(CanvasGeometry::Path(path)) = app.world().get::<CanvasGeometry>(item).cloned()
        else {
            panic!("redo must restore the path");
        };
        let transform =
            Transform::from_xyz(18.0, -24.0, 0.0).with_rotation(Quat::from_rotation_z(0.3));
        CanvasDocument::update_transform(app.world_mut(), document, item, transform).unwrap();
        let copy = CanvasDocument::create_path(app.world_mut(), document, path, transform, paint())
            .unwrap();
        assert!(
            CanvasDocument::set_path_segment(
                app.world_mut(),
                document,
                copy,
                1,
                CanvasPathSegment::Quadratic { control: Vec2::Y }
            )
            .unwrap()
        );
        let directory = tempfile::tempdir().unwrap();
        let svg = directory.path().join("paths.svg");
        CanvasDocument::export_svg(app.world_mut(), document, &svg, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(svg)
                .unwrap()
                .matches("<path")
                .count(),
            2
        );
        while CanvasDocument::undo(app.world_mut(), document).unwrap() {}
        moved(&mut app, id, &mut at, -delta);
    }
}

#[test]
fn impossible_arc_drags_leave_no_item_and_shift_only_snaps_radial_direction() {
    let id = pointer_ids()[1];
    let (mut app, view, _, original, _, mut at) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasDocument::clear_history(app.world_mut(), document).unwrap();
    let arc = CanvasPathTool::Arc {
        radii: Vec2::new(40.0, 4.0),
        rotation: std::f32::consts::FRAC_PI_2,
        large_arc: false,
        sweep: true,
    };
    for delta in [Vec2::ZERO, Vec2::X * 10.0] {
        CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Parametric(arc, paint()))
            .unwrap();
        moved(&mut app, id, &mut at, Vec2::ONE);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        moved(&mut app, id, &mut at, delta);
        assert_eq!(previews(&mut app, "CanvasParametricPreview"), 0);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(
            CanvasDocument::draw_order(app.world(), document).unwrap(),
            [original]
        );
        assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
    }
    let polygon = CanvasPathTool::RegularPolygon { sides: 4 };
    CanvasView::set_mode_in_world(
        app.world_mut(),
        view,
        CanvasMode::Parametric(polygon, paint()),
    )
    .unwrap();
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    let delta = Vec2::new(-15.0, -20.0);
    moved(&mut app, id, &mut at, delta);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftRight);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let CanvasPathCommand::MoveTo(first) = commands(&app, view)[0] else {
        panic!("expected vertex")
    };
    assert!(first.abs_diff_eq(Vec2::from_angle((-120.0_f32).to_radians()) * 25.0, 0.001));
}

#[test]
fn quarter_turned_arc_accepts_a_drag_across_its_full_major_diameter() {
    let id = pointer_ids()[1];
    let (mut app, view, _, _, _, mut at) = fixture(id, 1.0, 1.0);
    let radii = Vec2::new(40.0, 4.0);
    let rotation = std::f32::consts::FRAC_PI_2;
    let tool = CanvasPathTool::Arc {
        radii,
        rotation,
        large_arc: false,
        sweep: true,
    };
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Parametric(tool, paint()))
        .unwrap();
    let half_chord = Vec2::Y * radii.x;
    moved(&mut app, id, &mut at, -half_chord);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, id, &mut at, half_chord * 2.0);
    assert_eq!(previews(&mut app, "CanvasParametricPreview"), 1);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        commands(&app, view),
        [
            CanvasPathCommand::MoveTo(-half_chord),
            CanvasPathCommand::ArcTo {
                radii,
                rotation,
                large_arc: false,
                sweep: true,
                to: half_chord
            }
        ]
    );
}

#[test]
fn contours_preview_in_one_view_and_finish_with_enter_or_double_click() {
    for (mode, closed) in [
        (CanvasMode::Polyline(paint()), false),
        (CanvasMode::Polygon(paint()), true),
    ] {
        let id = pointer_ids()[1];
        let (mut app, a, b, original, window, mut at) = fixture(id, 1.0, 1.0);
        add_keyboard_dispatch(&mut app);
        app.world_mut()
            .resource_mut::<PickingSettings>()
            .multi_click_interval = std::time::Duration::MAX;
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        CanvasDocument::clear_history(app.world_mut(), document).unwrap();
        CanvasView::set_mode_in_world(app.world_mut(), a, mode).unwrap();

        let deltas = [
            Vec2::new(25.0, 20.0),
            Vec2::new(25.0, 0.0),
            Vec2::new(0.0, 30.0),
        ];
        for delta in deltas {
            moved(&mut app, id, &mut at, delta);
            click(&mut app, id, &at);
            assert_eq!(
                CanvasDocument::draw_order(app.world(), document).unwrap(),
                [original]
            );
            assert_eq!(previews(&mut app, "CanvasContourPreview"), 1);
        }
        if closed {
            click(&mut app, id, &at);
        } else {
            press_key(&mut app, window, KeyCode::Enter);
        }

        let path = commands(&app, a);
        let CanvasPathCommand::MoveTo(first) = path[0] else {
            panic!("first command must move");
        };
        assert_eq!(
            &path[1..3],
            [
                CanvasPathCommand::LineTo(first + deltas[1]),
                CanvasPathCommand::LineTo(first + deltas[1] + deltas[2])
            ]
        );
        assert_eq!(path.get(3), closed.then_some(&CanvasPathCommand::Close));
        assert_eq!(previews(&mut app, "CanvasContourPreview"), 0);
        assert!(CanvasView::selection(app.world(), b).unwrap().is_empty());
        assert!(CanvasDocument::undo(app.world_mut(), document).unwrap());
        assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
    }
}

#[test]
fn insufficient_vertices_and_cancelled_drawings_leave_no_history() {
    let id = pointer_ids()[1];
    let (mut app, view, _, original, window, mut at) = fixture(id, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasDocument::clear_history(app.world_mut(), document).unwrap();
    for (mode, preview) in [
        (CanvasMode::Line(paint()), "CanvasLinePreview"),
        (CanvasMode::Polyline(paint()), "CanvasContourPreview"),
        (CanvasMode::Polygon(paint()), "CanvasContourPreview"),
    ] {
        moved(&mut app, id, &mut at, Vec2::new(2.0, 1.0));
        CanvasView::set_mode_in_world(app.world_mut(), view, mode).unwrap();
        click(&mut app, id, &at);
        if matches!(mode, CanvasMode::Polygon(_)) {
            moved(&mut app, id, &mut at, Vec2::new(15.0, 0.0));
            click(&mut app, id, &at);
        }
        press_key(&mut app, window, KeyCode::Enter);
        assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
        CanvasView::set_mode_in_world(app.world_mut(), view, mode).unwrap();
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        moved(&mut app, id, &mut at, Vec2::new(15.0, 5.0));
        assert_eq!(previews(&mut app, preview), 1, "{mode:?}");
        if matches!(mode, CanvasMode::Line(_)) {
            input(&mut app, id, &at, PointerAction::Cancel);
        } else {
            press_key(&mut app, window, KeyCode::Escape);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
        }
        assert_eq!(previews(&mut app, preview), 0);
        assert_eq!(
            CanvasDocument::draw_order(app.world(), document).unwrap(),
            [original]
        );
        assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
    }
}

#[test]
fn contour_drags_do_not_add_vertices_or_seed_double_clicks() {
    for return_to_origin in [false, true] {
        let id = pointer_ids()[1];
        let (mut app, view, _, original, window, mut at) = fixture(id, 1.0, 1.0);
        add_keyboard_dispatch(&mut app);
        app.world_mut()
            .resource_mut::<PickingSettings>()
            .multi_click_interval = std::time::Duration::MAX;
        let document = CanvasView::document(app.world(), view).unwrap().unwrap();
        CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Polyline(paint()))
            .unwrap();
        let delta = Vec2::new(20.0, 15.0);
        for _ in 0..2 {
            moved(&mut app, id, &mut at, delta);
            click(&mut app, id, &at);
        }
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        moved(&mut app, id, &mut at, delta);
        if return_to_origin {
            moved(&mut app, id, &mut at, -delta);
        }
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        if !return_to_origin {
            moved(&mut app, id, &mut at, -delta);
            click(&mut app, id, &at);
        }
        assert_eq!(
            CanvasDocument::draw_order(app.world(), document).unwrap(),
            [original]
        );
        press_key(&mut app, window, KeyCode::Enter);
        assert_eq!(commands(&app, view).len(), 2);
    }
}

#[test]
fn successive_native_taps_keep_a_contour_after_each_touch_pointer_disappears() {
    for batch_touch_handoff in [false, true] {
        let (mut app, view, _, original, window, at) = fixture(PointerId::Mouse, 1.0, 1.0);
        app.world_mut()
            .resource_mut::<PointerInputSettings>()
            .is_touch_enabled = true;
        app.world_mut()
            .resource_mut::<PickingSettings>()
            .multi_click_interval = std::time::Duration::MAX;
        let document = CanvasView::document(app.world(), view).unwrap().unwrap();
        CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Polygon(paint())).unwrap();
        let offsets = [
            Vec2::new(25.0, 20.0),
            Vec2::new(50.0, 20.0),
            Vec2::new(50.0, 50.0),
            Vec2::new(50.0, 50.0),
        ];
        for (i, offset) in offsets.into_iter().enumerate() {
            for phase in [TouchPhase::Started, TouchPhase::Ended] {
                let touch = TouchInput {
                    phase,
                    position: at.position + offset,
                    window,
                    force: None,
                    id: i as u64,
                };
                app.world_mut().write_message(touch);
                app.world_mut()
                    .write_message(WindowEvent::TouchInput(touch));
                if phase == TouchPhase::Started || !batch_touch_handoff {
                    update(&mut app);
                }
            }
            if i < 3 && !batch_touch_handoff {
                update(&mut app);
                assert_eq!(previews(&mut app, "CanvasContourPreview"), 1);
                assert_eq!(
                    CanvasDocument::draw_order(app.world(), document).unwrap(),
                    [original]
                );
            }
        }
        update(&mut app);
        assert_eq!(commands(&app, view).len(), 4);
        assert_eq!(commands(&app, view).last(), Some(&CanvasPathCommand::Close));
    }
}

#[test]
fn space_pan_breaks_a_contour_double_click_at_the_same_screen_position() {
    let id = pointer_ids()[1];
    let (mut app, view, _, original, window, mut at) = fixture(id, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    app.world_mut()
        .resource_mut::<PickingSettings>()
        .multi_click_interval = std::time::Duration::MAX;
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Polyline(paint())).unwrap();
    let delta = Vec2::new(23.0, 15.0);
    for _ in 0..2 {
        moved(&mut app, id, &mut at, delta);
        click(&mut app, id, &at);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, id, &mut at, delta);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(CanvasView::center(app.world(), view).unwrap(), -delta);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    moved(&mut app, id, &mut at, -delta);
    click(&mut app, id, &at);
    assert_eq!(
        CanvasDocument::draw_order(app.world(), document).unwrap(),
        [original]
    );
    press_key(&mut app, window, KeyCode::Enter);
    assert_eq!(commands(&app, view).len(), 3);
}

#[test]
fn contour_keeps_a_first_click_batched_with_the_move_from_outside() {
    for id in pointer_ids() {
        for both_clicks_batched in [false, true] {
            let (mut app, view, _, original, window, mut at) = fixture(id, 1.0, 1.0);
            add_keyboard_dispatch(&mut app);
            let document = CanvasView::document(app.world(), view).unwrap().unwrap();
            let mut outside = at.clone();
            outside.position = Vec2::new(1000.0, 700.0);
            input(
                &mut app,
                id,
                &outside,
                PointerAction::Move { delta: Vec2::ONE },
            );
            CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Polyline(paint()))
                .unwrap();
            for action in [
                PointerAction::Move { delta: Vec2::ONE },
                PointerAction::Press(PointerButton::Primary),
                PointerAction::Release(PointerButton::Primary),
            ] {
                queue_pointer(&mut app, id, at.clone(), action);
            }
            let delta = Vec2::new(30.0, 20.0);
            if both_clicks_batched {
                at.position += delta;
                for action in [
                    PointerAction::Move { delta },
                    PointerAction::Press(PointerButton::Primary),
                    PointerAction::Release(PointerButton::Primary),
                ] {
                    queue_pointer(&mut app, id, at.clone(), action);
                }
            }
            update(&mut app);
            assert_eq!(previews(&mut app, "CanvasContourPreview"), 1);
            assert_eq!(
                CanvasDocument::draw_order(app.world(), document).unwrap(),
                [original]
            );
            if !both_clicks_batched {
                moved(&mut app, id, &mut at, delta);
                click(&mut app, id, &at);
            }
            press_key(&mut app, window, KeyCode::Enter);
            assert_eq!(commands(&app, view).len(), 2);
        }
    }
}

fn stroke_mesh_covers(mesh: &Mesh, point: Vec2) -> bool {
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let indices = mesh.indices().unwrap().iter().collect::<Vec<_>>();
    let (triangles, _) = indices.as_chunks::<3>();
    triangles.iter().any(|triangle| {
        let a = Vec3::from_array(positions[triangle[0]]).truncate();
        let b = Vec3::from_array(positions[triangle[1]]).truncate();
        let c = Vec3::from_array(positions[triangle[2]]).truncate();
        let sides = [
            (b - a).perp_dot(point - a),
            (c - b).perp_dot(point - b),
            (a - c).perp_dot(point - c),
        ];
        sides.iter().all(|side| *side >= 0.0) || sides.iter().all(|side| *side <= 0.0)
    })
}

#[test]
fn contour_hover_stroke_tracks_the_cursor_and_closes_only_for_polygon_in_the_active_view() {
    for (mode, closed) in [
        (CanvasMode::Polyline(paint()), false),
        (CanvasMode::Polygon(paint()), true),
    ] {
        let id = pointer_ids()[1];
        let (mut app, a, b, _, _, base) = fixture(id, 1.0, 1.0);
        CanvasView::set_mode_in_world(app.world_mut(), a, mode).unwrap();
        let [first, middle, last] = [
            Vec2::new(-60.0, -35.0),
            Vec2::new(-20.0, -35.0),
            Vec2::new(-20.0, 5.0),
        ];
        for point in [first, middle] {
            let at = coordinates::at_document(&app, a, &base, point);
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            click(&mut app, id, &at);
        }
        let preview = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "CanvasContourPreview")
            .unwrap()
            .0;
        let stroke = app.world().get::<Children>(preview).unwrap()[1];
        let layers = app.world().get::<RenderLayers>(stroke).unwrap();
        for (view, visible) in [(a, true), (b, false)] {
            let camera = app
                .world()
                .get::<ViewportNode>(view)
                .unwrap()
                .camera
                .unwrap();
            assert_eq!(
                layers.intersects(app.world().get::<RenderLayers>(camera).unwrap()),
                visible
            );
        }
        let handle = app.world().get::<Mesh2d>(stroke).unwrap().0.clone();
        let hover_segment = middle.lerp(last, 0.5);
        let closing_segment = last.lerp(first, 0.5);
        assert!(!stroke_mesh_covers(
            app.world().resource::<Assets<Mesh>>().get(&handle).unwrap(),
            hover_segment
        ));
        let at = coordinates::at_document(&app, a, &base, last);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&handle).unwrap();
        assert!(stroke_mesh_covers(mesh, hover_segment));
        assert_eq!(stroke_mesh_covers(mesh, closing_segment), closed);
    }
}

#[test]
fn a_contour_cancels_when_window_focus_or_tool_changes() {
    let id = pointer_ids()[1];
    let (mut app, view, _, original, window, mut at) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasDocument::clear_history(app.world_mut(), document).unwrap();
    let contour = CanvasMode::Polyline(paint());
    moved(&mut app, id, &mut at, Vec2::new(20.0, 15.0));
    for lose_focus in [true, false] {
        CanvasView::set_mode_in_world(app.world_mut(), view, contour).unwrap();
        click(&mut app, id, &at);
        moved(&mut app, id, &mut at, Vec2::new(20.0, 15.0));
        assert_eq!(previews(&mut app, "CanvasContourPreview"), 1);
        if lose_focus {
            app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        } else {
            CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Pen(paint())).unwrap();
        }
        update(&mut app);
        assert_eq!(previews(&mut app, "CanvasContourPreview"), 0);
        assert_eq!(
            CanvasDocument::draw_order(app.world(), document).unwrap(),
            [original]
        );
        assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        update(&mut app);
    }
}
