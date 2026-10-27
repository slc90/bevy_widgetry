#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{LogCapture, pointer_ids};
#[allow(dead_code)]
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;

fn sorted(shapes: impl IntoIterator<Item = Entity>) -> Vec<Entity> {
    let mut v = shapes.into_iter().collect::<Vec<_>>();
    v.sort_unstable();
    v.dedup();
    v
}
fn click(app: &mut App, id: PointerId, at: &Location) {
    for action in [
        PointerAction::Move { delta: Vec2::ONE },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        input(app, id, at, action);
    }
}
fn add(app: &mut App, document: Entity, x: f32) -> Entity {
    CanvasDocument::create_ellipse(
        app.world_mut(),
        document,
        CanvasEllipse {
            radii: Vec2::splat(10.0),
        },
        Transform::from_xyz(x, 0.0, 0.0),
        CanvasPaint::default(),
    )
    .unwrap()
}

#[test]
fn selection_api_canonicalizes_full_sets_notifies_after_commit_and_repairs_only_invalid_members() {
    let (mut app, a, b, first, _, _) = fixture(pointer_ids()[1], 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let second = add(&mut app, document, 40.0);
    #[derive(Resource, Default)]
    struct Seen(Vec<Vec<Entity>>);
    app.init_resource::<Seen>().add_observer(
        |e: On<CanvasSelectionChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                CanvasView::selection(&world, e.entity).unwrap(),
                e.selection
            );
            world.resource_mut::<Seen>().0.push(e.selection.clone());
        },
    );
    assert!(
        CanvasView::set_selection_in_world(app.world_mut(), a, [second, first, second]).unwrap()
    );
    assert_eq!(
        CanvasView::selection(app.world(), a).unwrap(),
        sorted([first, second])
    );
    assert_eq!(CanvasView::single_selection(app.world(), a).unwrap(), None);
    assert!(!CanvasView::set_selection_in_world(app.world_mut(), a, [first, second]).unwrap());
    assert_eq!(app.world().resource::<Seen>().0.len(), 1);
    CanvasView::set_selection_in_world(app.world_mut(), b, [first]).unwrap();
    let log = LogCapture::default();
    let error = log
        .run(|| {
            CanvasView::set_selection_in_world(app.world_mut(), a, [first, Entity::PLACEHOLDER])
        })
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(log.records().len(), 1);
    assert_eq!(
        CanvasView::selection(app.world(), a).unwrap(),
        sorted([first, second])
    );
    CanvasDocument::remove_shape(app.world_mut(), document, first).unwrap();
    update(&mut app);
    assert_eq!(CanvasView::selection(app.world(), a).unwrap(), [second]);
    assert!(CanvasView::selection(app.world(), b).unwrap().is_empty());
    assert_eq!(
        CanvasView::single_selection(app.world(), a).unwrap(),
        Some(second)
    );
    CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
    assert!(CanvasView::selection(app.world(), a).unwrap().is_empty());
}

#[test]
fn ctrl_click_toggles_actual_shapes_without_capture_and_plain_click_replaces_or_clears() {
    for id in pointer_ids() {
        for key in [KeyCode::ControlLeft, KeyCode::ControlRight] {
            let (mut app, a, b, first, _, base) = fixture(id, 1.0, 1.0);
            let document = CanvasView::document(app.world(), a).unwrap().unwrap();
            let second = add(&mut app, document, 40.0);
            let second_at = coordinates::at_document(&app, a, &base, Vec2::new(40.0, 0.0));
            let blank = coordinates::at_document(&app, a, &base, Vec2::new(75.0, 55.0));
            click(&mut app, id, &base);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            click(&mut app, id, &second_at);
            assert_eq!(
                CanvasView::selection(app.world(), a).unwrap(),
                sorted([first, second])
            );
            assert!(CanvasView::selection(app.world(), b).unwrap().is_empty());
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            click(&mut app, id, &blank);
            assert_eq!(
                CanvasView::selection(app.world(), a).unwrap(),
                sorted([first, second])
            );
            click(&mut app, id, &base);
            assert_eq!(CanvasView::selection(app.world(), a).unwrap(), [second]);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(key);
            click(&mut app, id, &base);
            assert_eq!(CanvasView::selection(app.world(), a).unwrap(), [first]);
            click(&mut app, id, &blank);
            assert!(CanvasView::selection(app.world(), a).unwrap().is_empty());
        }
    }
}

#[test]
fn multi_outlines_are_view_private_have_no_edit_handles_and_do_not_change_shapes() {
    let id = pointer_ids()[1];
    let (mut app, a, b, first, _, _) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let second = add(&mut app, document, 40.0);
    CanvasView::set_selection_in_world(app.world_mut(), a, [first, second]).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), b, [second]).unwrap();
    update(&mut app);
    let outlines = app
        .world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .filter(|(n, _)| n.as_str() == "CanvasSelectionOutline")
        .map(|(_, m)| m.0.clone())
        .collect::<Vec<_>>();
    assert_eq!(outlines.len(), 2);
    let meshes = app.world().resource::<Assets<Mesh>>();
    assert!(
        outlines
            .iter()
            .any(|h| meshes.get(h).unwrap().count_vertices() == 16),
        "two reference frames have no scale/rotate/parameter handles"
    );
    let saved = app.world().get::<Transform>(first).copied().unwrap();
    assert_eq!(
        CanvasDocument::shape_order(app.world_mut(), document)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        CanvasView::selection(app.world(), a).unwrap(),
        sorted([first, second])
    );
    assert_eq!(CanvasView::selection(app.world(), b).unwrap(), [second]);
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::PathNodeEdit).unwrap();
    update(&mut app);
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|n| n.as_str() == "CanvasPathNodes")
            .count(),
        0
    );
    assert_eq!(
        CanvasView::selection(app.world(), a).unwrap(),
        sorted([first, second])
    );
    assert_eq!(app.world().get::<Transform>(first), Some(&saved));
    let baseline = app.world().resource::<Assets<Mesh>>().len();
    CanvasView::set_selection_in_world(app.world_mut(), a, []).unwrap();
    update(&mut app);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), baseline - 1);
    assert_eq!(CanvasView::selection(app.world(), b).unwrap(), [second]);
}

#[test]
fn queued_sets_validate_at_execution_and_reparenting_repairs_only_the_departing_member() {
    use bevy_widgetry_test_utils::ErrorCapture;
    let (mut app, a, _, first, _, _) = fixture(pointer_ids()[1], 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let second = add(&mut app, document, 40.0);
    CanvasView::set_selection(&mut app.world_mut().commands(), a, [second, first]);
    assert!(CanvasView::selection(app.world(), a).unwrap().is_empty());
    app.world_mut().flush();
    assert_eq!(
        CanvasView::selection(app.world(), a).unwrap(),
        sorted([first, second])
    );
    let other = app.world_mut().spawn(CanvasDocument).id();
    app.world_mut().entity_mut(second).insert(ChildOf(other));
    assert_eq!(CanvasView::selection(app.world(), a).unwrap(), [first]);
    update(&mut app);
    assert_eq!(CanvasView::selection(app.world(), a).unwrap(), [first]);
    CanvasView::set_selection(&mut app.world_mut().commands(), a, [first, second]);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert_eq!(logs.records().len(), 1);
    assert_eq!(CanvasView::selection(app.world(), a).unwrap(), [first]);
}

#[test]
fn space_pan_preserves_a_multi_selection_and_multi_reference_corners_cannot_transform_it() {
    let id = pointer_ids()[1];
    let (mut app, a, _, first, _, base) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let second = add(&mut app, document, 40.0);
    CanvasView::set_selection_in_world(app.world_mut(), a, [first, second]).unwrap();
    let original = *app.world().get::<Transform>(first).unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    let mut at = base.clone();
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        CanvasView::selection(app.world(), a).unwrap(),
        sorted([first, second])
    );
    assert_eq!(
        CanvasView::center(app.world(), a).unwrap(),
        Vec2::new(-10.0, -5.0)
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ControlLeft);
    let mut corner = coordinates::at_document(&app, a, &base, Vec2::splat(-18.0));
    input(
        &mut app,
        id,
        &corner,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        id,
        &corner,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, id, &mut corner, Vec2::splat(10.0));
    input(
        &mut app,
        id,
        &corner,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(app.world().get::<Transform>(first), Some(&original));
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
}

#[test]
fn svg_ignores_different_view_selection_sets_and_reference_frames() {
    let (mut app, a, b, first, _, _) = fixture(pointer_ids()[1], 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let second = CanvasDocument::create_rectangle(
        app.world_mut(),
        document,
        CanvasRectangle {
            size: Vec2::splat(20.0),
            corner_radius: 0.0,
        },
        Transform::from_xyz(40.0, 0.0, 0.0),
        CanvasPaint::default(),
    )
    .unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(app.world_mut(), document, file.path(), None).unwrap();
    let before = std::fs::read(file.path()).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), a, [first, second]).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), b, [second]).unwrap();
    CanvasView::set_zoom_in_world(app.world_mut(), a, 2.0).unwrap();
    update(&mut app);
    CanvasDocument::export_svg(app.world_mut(), document, file.path(), None).unwrap();
    assert_eq!(std::fs::read(file.path()).unwrap(), before);
}

#[test]
fn ctrl_geometry_hits_pass_through_path_holes_and_select_whole_multicontour_paths() {
    use CanvasPathCommand::*;
    let id = pointer_ids()[1];
    let (mut app, a, _, bottom, _, base) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let path = CanvasPath {
        commands: vec![
            MoveTo(Vec2::splat(-40.0)),
            LineTo(Vec2::new(40.0, -40.0)),
            LineTo(Vec2::splat(40.0)),
            LineTo(Vec2::new(-40.0, 40.0)),
            Close,
            MoveTo(Vec2::splat(-20.0)),
            LineTo(Vec2::new(20.0, -20.0)),
            LineTo(Vec2::splat(20.0)),
            LineTo(Vec2::new(-20.0, 20.0)),
            Close,
            MoveTo(Vec2::new(60.0, -10.0)),
            LineTo(Vec2::new(80.0, -10.0)),
            LineTo(Vec2::new(70.0, 10.0)),
            Close,
        ],
    };
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        path,
        Transform::default(),
        CanvasPaint {
            fill_rule: CanvasFillRule::EvenOdd,
            ..default()
        },
    )
    .unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    for (point, expected) in [
        (Vec2::ZERO, vec![bottom]),
        (Vec2::new(30.0, 0.0), sorted([bottom, shape])),
        (Vec2::new(70.0, 0.0), vec![bottom]),
    ] {
        let at = coordinates::at_document(&app, a, &base, point);
        click(&mut app, id, &at);
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), expected);
    }
}
