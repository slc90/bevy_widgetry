#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::{picking::pointer::*, prelude::*};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::pointer_ids;

#[allow(dead_code)]
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;

const PRESETS: [CanvasFlowchartPreset; 6] = {
    use CanvasFlowchartPreset::*;
    [Terminator, Decision, InputOutput, Database, Document, Arrow]
};

#[test]
fn presets_produce_editable_contours_inside_the_requested_box() {
    use CanvasFlowchartPreset::*;
    let size = Vec2::new(80.0, 40.0);
    for preset in PRESETS {
        assert!(preset.geometry(Vec2::ZERO).is_none());
        assert!(preset.geometry(Vec2::new(f32::NAN, 12.0)).is_none());
        assert!(preset.geometry(Vec2::new(0.5, 20.0)).is_none());
        let geometry = preset.geometry(size).unwrap();
        let paint = CanvasPaint::default();
        geometry.validate(&Transform::IDENTITY, &paint).unwrap();
        let bounds = geometry.bounds(&Transform::IDENTITY, &paint).unwrap();
        assert!(
            bounds.min.abs_diff_eq(-size * 0.5, 0.01),
            "{preset:?} {bounds:?}"
        );
        assert!(
            bounds.max.abs_diff_eq(size * 0.5, 0.01),
            "{preset:?} {bounds:?}"
        );
        match geometry {
            CanvasGeometry::Rectangle(rect) => {
                assert_eq!(preset, Terminator);
                assert_eq!(rect.corner_radius, 20.0);
            }
            CanvasGeometry::Path(path) => {
                use CanvasPathCommand::*;
                assert_ne!(preset, Terminator);
                match preset {
                    InputOutput => assert_eq!(
                        path.commands,
                        [
                            MoveTo(Vec2::new(-20.0, -20.0)),
                            LineTo(Vec2::new(40.0, -20.0)),
                            LineTo(Vec2::new(20.0, 20.0)),
                            LineTo(Vec2::new(-40.0, 20.0)),
                            Close
                        ]
                    ),
                    Arrow => assert_eq!(
                        path.commands,
                        [
                            MoveTo(Vec2::new(-40.0, -9.0)),
                            LineTo(Vec2::new(8.0, -9.0)),
                            LineTo(Vec2::new(8.0, -20.0)),
                            LineTo(Vec2::new(40.0, 0.0)),
                            LineTo(Vec2::new(8.0, 20.0)),
                            LineTo(Vec2::new(8.0, 9.0)),
                            LineTo(Vec2::new(-40.0, 9.0)),
                            Close
                        ]
                    ),
                    Database => assert_eq!(
                        path.commands
                            .iter()
                            .filter(|cmd| matches!(cmd, ArcTo { .. }))
                            .count(),
                        2
                    ),
                    _ => {}
                }
                assert_eq!(path.commands.last(), Some(&CanvasPathCommand::Close));
                assert_eq!(
                    path.commands
                        .iter()
                        .filter(|c| matches!(c, CanvasPathCommand::ArcTo { .. }))
                        .count()
                        > 0,
                    preset == Database
                );
                assert_eq!(
                    path.commands
                        .iter()
                        .filter(|c| matches!(c, CanvasPathCommand::CubicTo { .. }))
                        .count()
                        > 0,
                    preset == Document
                );
            }
            _ => panic!("unexpected preset geometry"),
        }
    }
}

#[test]
fn decision_hit_uses_its_outline_and_the_edited_copy_exports_in_a_group() {
    let id = pointer_ids()[1];
    let (mut app, view, _, _, _, base) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    let paint = CanvasPaint::default();
    let Some(CanvasGeometry::Path(path)) =
        CanvasFlowchartPreset::Decision.geometry(Vec2::new(80.0, 40.0))
    else {
        panic!("expected decision path");
    };
    let item = CanvasDocument::create_path(
        app.world_mut(),
        document,
        path.clone(),
        Transform::IDENTITY,
        paint,
    )
    .unwrap();
    update(&mut app);
    for (point, expected) in [
        (Vec2::new(36.0, 16.0), None),
        (Vec2::new(22.0, 0.0), Some(item)),
    ] {
        let at = coordinates::at_document(&app, view, &base, point);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(
            CanvasView::single_selection(app.world(), view).unwrap(),
            expected
        );
    }
    let copy = CanvasDocument::create_path(
        app.world_mut(),
        document,
        path,
        Transform::from_xyz(90.0, 0.0, 0.0),
        paint,
    )
    .unwrap();
    CanvasDocument::insert_path_anchor(app.world_mut(), document, copy, 1, Vec2::new(25.0, -10.0))
        .unwrap();
    let group =
        CanvasDocument::group_items(app.world_mut(), document, &[item, copy], "Decisions").unwrap();
    CanvasDocument::update_group_transform(
        app.world_mut(),
        document,
        group,
        Transform::from_scale(Vec3::new(1.25, 1.25, 1.0)).with_rotation(Quat::from_rotation_z(0.4)),
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let svg = directory.path().join("group.svg");
    CanvasDocument::export_svg(app.world_mut(), document, &svg, None).unwrap();
    assert_eq!(
        std::fs::read_to_string(svg)
            .unwrap()
            .matches("<path")
            .count(),
        2
    );
}

#[test]
fn database_cap_is_filled_under_both_winding_rules() {
    let id = pointer_ids()[1];
    for fill_rule in [CanvasFillRule::NonZero, CanvasFillRule::EvenOdd] {
        let (mut app, view, _, _, _, base) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), view).unwrap().unwrap();
        let Some(CanvasGeometry::Path(path)) =
            CanvasFlowchartPreset::Database.geometry(Vec2::new(80.0, 40.0))
        else {
            panic!("expected cylinder path");
        };
        let item = CanvasDocument::create_path(
            app.world_mut(),
            document,
            path,
            Transform::IDENTITY,
            CanvasPaint {
                fill_rule,
                ..default()
            },
        )
        .unwrap();
        update(&mut app);
        let at = coordinates::at_document(&app, view, &base, Vec2::new(0.0, -11.0));
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(
            CanvasView::single_selection(app.world(), view).unwrap(),
            Some(item),
            "{fill_rule:?}"
        );
    }
}

#[test]
fn drag_cancel_and_commit_each_preset_as_one_document_edit() {
    let id = pointer_ids()[1];
    for preset in PRESETS {
        let (mut app, main, other, original, _, mut at) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), main).unwrap().unwrap();
        CanvasDocument::clear_history(app.world_mut(), document).unwrap();
        let delta = Vec2::new(-54.0, -30.0);
        for cancel in [true, false] {
            CanvasView::set_mode_in_world(
                app.world_mut(),
                main,
                CanvasMode::Flowchart(preset, CanvasPaint::default()),
            )
            .unwrap();
            moved(&mut app, id, &mut at, Vec2::ONE);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            moved(&mut app, id, &mut at, delta);
            assert_eq!(
                app.world_mut()
                    .query::<&Name>()
                    .iter(app.world())
                    .filter(|name| name.as_str() == "CanvasFlowchartPreview")
                    .count(),
                1
            );
            assert_eq!(
                CanvasDocument::draw_order(app.world(), document).unwrap(),
                [original]
            );
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
            if cancel {
                assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
                moved(&mut app, id, &mut at, -delta);
            }
        }
        let item = CanvasView::single_selection(app.world(), main)
            .unwrap()
            .unwrap();
        assert_eq!(
            app.world().get::<CanvasGeometry>(item),
            preset.geometry(delta.abs()).as_ref()
        );
        assert!(
            CanvasView::selection(app.world(), other)
                .unwrap()
                .is_empty()
        );
        assert!(CanvasDocument::undo(app.world_mut(), document).unwrap());
        assert_eq!(
            CanvasDocument::draw_order(app.world(), document).unwrap(),
            [original]
        );
        assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
        assert!(CanvasDocument::redo(app.world_mut(), document).unwrap());
        assert_eq!(
            CanvasDocument::draw_order(app.world(), document).unwrap(),
            [original, item]
        );
    }
}
