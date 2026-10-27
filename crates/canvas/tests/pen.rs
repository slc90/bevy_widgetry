#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_keyboard_dispatch, pointer_ids, press_key, queue_pointer,
};
#[allow(dead_code)]
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;
use coordinates::at_document;

fn previews(app: &mut App) -> usize {
    app.world_mut()
        .query::<&Name>()
        .iter(app.world())
        .filter(|n| n.as_str() == "CanvasPenPreview")
        .count()
}

fn click(app: &mut App, id: PointerId, view: Entity, base: &Location, point: Vec2) {
    let at = at_document(app, view, base, point);
    input(app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(app, id, &at, PointerAction::Press(PointerButton::Primary));
    input(app, id, &at, PointerAction::Release(PointerButton::Primary));
}

#[test]
fn pen_builds_typed_curves_and_multiple_subpaths_then_commits_once_and_stays_in_pen() {
    for id in pointer_ids() {
        let (mut app, a, b, original, _, base) = fixture(id, 1.0, 1.0);
        add_keyboard_dispatch(&mut app);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let paint = CanvasPaint {
            stroke: Some(CanvasStroke::default()),
            ..default()
        };
        CanvasView::set_selection_in_world(app.world_mut(), b, Some(original)).unwrap();
        CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Pen(paint)).unwrap();
        click(&mut app, id, a, &base, Vec2::new(-40.0, -20.0));
        click(&mut app, id, a, &base, Vec2::new(0.0, -20.0));
        assert!(CanvasView::pen_draft(app.world(), a).unwrap().is_some());
        CanvasView::set_pen_segment_in_world(app.world_mut(), a, CanvasPenSegment::Quadratic)
            .unwrap();
        click(&mut app, id, a, &base, Vec2::new(20.0, -30.0));
        click(&mut app, id, a, &base, Vec2::new(40.0, 0.0));
        assert!(CanvasView::close_pen_subpath_in_world(app.world_mut(), a).unwrap());
        assert_eq!(
            CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
            [original]
        );
        CanvasView::new_pen_subpath_in_world(app.world_mut(), a).unwrap();
        click(&mut app, id, a, &base, Vec2::new(-40.0, 20.0));
        CanvasView::set_pen_segment_in_world(app.world_mut(), a, CanvasPenSegment::Cubic).unwrap();
        for p in [
            Vec2::new(-20.0, 30.0),
            Vec2::new(20.0, 40.0),
            Vec2::new(40.0, 20.0),
        ] {
            click(&mut app, id, a, &base, p);
        }
        CanvasView::set_pen_segment_in_world(app.world_mut(), a, CanvasPenSegment::Arc).unwrap();
        click(&mut app, id, a, &base, Vec2::new(30.0, 40.0));
        let draft = CanvasView::pen_draft(app.world(), a).unwrap().unwrap();
        assert_eq!(previews(&mut app), 1);
        assert_eq!(draft.path.commands.len(), 7);
        assert!(
            matches!(draft.path.commands[2],CanvasPathCommand::QuadTo {control,to} if control==Vec2::new(20.0,-30.0)&&to==Vec2::new(40.0,0.0))
        );
        assert!(matches!(
            draft.path.commands[5],
            CanvasPathCommand::CubicTo { .. }
        ));
        assert!(matches!(
            draft.path.commands[6],
            CanvasPathCommand::ArcTo { .. }
        ));
        let shape = CanvasView::finish_pen_in_world(app.world_mut(), a)
            .unwrap()
            .unwrap();
        update(&mut app);
        assert_eq!(
            app.world().get::<CanvasGeometry>(shape),
            Some(&CanvasGeometry::Path(draft.path))
        );
        assert_eq!(
            CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
            [original, shape]
        );
        assert_eq!(
            CanvasView::mode(app.world(), a).unwrap(),
            CanvasMode::Pen(paint)
        );
        assert!(CanvasView::pen_draft(app.world(), a).unwrap().is_none());
        assert_eq!(previews(&mut app), 0);
        assert_eq!(
            CanvasView::selection(app.world(), b).unwrap(),
            Some(original)
        );
        CanvasView::set_pen_segment_in_world(app.world_mut(), a, CanvasPenSegment::Line).unwrap();
        click(&mut app, id, a, &base, Vec2::ZERO);
        click(&mut app, id, a, &base, Vec2::new(10.0, 10.0));
        let second = CanvasView::finish_pen_in_world(app.world_mut(), a)
            .unwrap()
            .unwrap();
        assert_ne!(shape, second);
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    }
}

#[test]
fn a_single_anchor_or_partial_curve_does_not_create_a_document_shape() {
    let id = pointer_ids()[1];
    let (mut app, a, _, original, _, base) = fixture(id, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Pen(CanvasPaint::default()))
        .unwrap();
    click(&mut app, id, a, &base, Vec2::ZERO);
    CanvasView::set_pen_segment_in_world(app.world_mut(), a, CanvasPenSegment::Cubic).unwrap();
    click(&mut app, id, a, &base, Vec2::new(10.0, 10.0));
    assert!(CanvasView::pen_draft(app.world(), a).unwrap().is_some());
    assert_eq!(
        CanvasView::finish_pen_in_world(app.world_mut(), a).unwrap(),
        None
    );
    assert!(CanvasView::pen_draft(app.world(), a).unwrap().is_none());
    assert_eq!(
        CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
        [original]
    );
}

#[test]
fn pen_cancellation_clears_idle_and_held_drafts_and_reclaims_only_private_preview_assets() {
    for held in [false, true] {
        for cause in 0..12 {
            let id = pointer_ids()[1];
            let (mut app, a, b, original, window, base) = fixture(id, 1.0, 1.0);
            add_keyboard_dispatch(&mut app);
            let document = CanvasView::document(app.world(), a).unwrap().unwrap();
            let mesh_count = app.world().resource::<Assets<Mesh>>().len();
            let material_count = app.world().resource::<Assets<ColorMaterial>>().len();
            CanvasView::set_mode_in_world(
                app.world_mut(),
                a,
                CanvasMode::Pen(CanvasPaint::default()),
            )
            .unwrap();
            click(&mut app, id, a, &base, Vec2::new(-20.0, 0.0));
            click(&mut app, id, a, &base, Vec2::new(20.0, 10.0));
            let at = at_document(&app, a, &base, Vec2::new(30.0, 0.0));
            if held {
                input(
                    &mut app,
                    id,
                    &at,
                    PointerAction::Press(PointerButton::Primary),
                );
            }
            assert_eq!(previews(&mut app), 1);
            match cause {
                0 => input(&mut app, id, &at, PointerAction::Cancel),
                1 => press_key(&mut app, window, KeyCode::Escape),
                2 => {
                    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Select).unwrap();
                }
                3 => {
                    app.world_mut()
                        .resource_mut::<bevy::input_focus::InputFocus>()
                        .set(b, bevy::input_focus::FocusCause::Navigated);
                }
                4 => app.world_mut().get_mut::<Window>(window).unwrap().focused = false,
                5 => {
                    app.world_mut().entity_mut(a).insert(Visibility::Hidden);
                }
                6 => {
                    app.world_mut()
                        .entity_mut(a)
                        .insert(bevy::ui::InteractionDisabled);
                }
                7 => {
                    CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
                }
                8 => {
                    app.world_mut().despawn(document);
                }
                9 => {
                    app.world_mut().despawn(a);
                }
                10 => app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None,
                _ => {
                    let pointer = app
                        .world_mut()
                        .query::<(Entity, &PointerId)>()
                        .iter(app.world())
                        .find(|(_, p)| **p == id)
                        .unwrap()
                        .0;
                    app.world_mut().despawn(pointer);
                }
            }
            update(&mut app);
            assert!(
                !app.world().resource::<PointerCaptureMap>().is_captured(&id),
                "held={held},cause={cause}"
            );
            if cause != 9 {
                assert!(
                    CanvasView::pen_draft(app.world(), a).unwrap().is_none(),
                    "held={held},cause={cause}"
                );
            }
            assert_eq!(previews(&mut app), 0);
            assert_eq!(
                app.world().resource::<Assets<Mesh>>().len(),
                if cause == 8 { 0 } else { mesh_count }
            );
            assert_eq!(
                app.world().resource::<Assets<ColorMaterial>>().len(),
                material_count
                    - if cause == 8 {
                        4
                    } else if cause == 7 || cause == 9 {
                        2
                    } else {
                        0
                    }
            );
            if cause != 8 {
                assert_eq!(
                    CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
                    [original]
                );
            }
        }
    }
}

#[test]
fn pen_space_pan_preserves_the_draft_and_other_pointers_cannot_append_or_cancel_it() {
    let id = pointer_ids()[1];
    let other = pointer_ids()[0];
    let (mut app, a, _, original, _, base) = fixture(id, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Pen(CanvasPaint::default()))
        .unwrap();
    click(&mut app, id, a, &base, Vec2::ZERO);
    click(&mut app, id, a, &base, Vec2::new(20.0, 10.0));
    let before = CanvasView::pen_draft(app.world(), a).unwrap();
    click(&mut app, other, a, &base, Vec2::new(30.0, 10.0));
    let mut at = at_document(&app, a, &base, Vec2::new(30.0, 10.0));
    input(&mut app, other, &at, PointerAction::Cancel);
    assert_eq!(CanvasView::pen_draft(app.world(), a).unwrap(), before);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    moved(&mut app, id, &mut at, Vec2::new(20.0, 10.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(CanvasView::pen_draft(app.world(), a).unwrap(), before);
    assert_eq!(
        CanvasView::center(app.world(), a).unwrap(),
        Vec2::new(-20.0, -10.0)
    );
    assert_eq!(
        CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
        [original]
    );
}

#[test]
fn pen_first_frame_release_uses_its_endpoint_and_keyboard_new_subpath_keeps_open_contours() {
    for id in pointer_ids() {
        let (mut app, a, _, original, window, base) = fixture(id, 1.0, 1.0);
        add_keyboard_dispatch(&mut app);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Pen(CanvasPaint::default()))
            .unwrap();
        let mut outside = base.clone();
        outside.position = Vec2::new(1000.0, 700.0);
        input(
            &mut app,
            id,
            &outside,
            PointerAction::Move { delta: Vec2::ONE },
        );
        for action in [
            PointerAction::Move { delta: Vec2::ONE },
            PointerAction::Press(PointerButton::Primary),
        ] {
            queue_pointer(&mut app, id, base.clone(), action);
        }
        let at = at_document(&app, a, &base, Vec2::new(5.0, 5.0));
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Move {
                delta: Vec2::new(5.0, 5.0),
            },
        );
        queue_pointer(
            &mut app,
            id,
            at,
            PointerAction::Release(PointerButton::Primary),
        );
        update(&mut app);
        let draft = CanvasView::pen_draft(app.world(), a).unwrap().unwrap();
        assert_eq!(
            draft.path.commands,
            [CanvasPathCommand::MoveTo(Vec2::new(5.0, 5.0))]
        );
        click(&mut app, id, a, &base, Vec2::new(20.0, 0.0));
        press_key(&mut app, window, KeyCode::KeyN);
        click(&mut app, id, a, &base, Vec2::new(-20.0, 20.0));
        click(&mut app, id, a, &base, Vec2::new(20.0, 20.0));
        press_key(&mut app, window, KeyCode::Enter);
        let order = CanvasDocument::shape_order(app.world_mut(), document).unwrap();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], original);
        let CanvasGeometry::Path(path) = app.world().get::<CanvasGeometry>(order[1]).unwrap()
        else {
            panic!("Path expected")
        };
        assert_eq!(path.commands.len(), 4);
        assert!(!path.commands.contains(&CanvasPathCommand::Close));
        assert!(matches!(
            CanvasView::mode(app.world(), a).unwrap(),
            CanvasMode::Pen(_)
        ));
    }
}

#[test]
fn pen_business_apis_notify_after_commit_and_reject_invalid_views_or_modes() {
    let id = pointer_ids()[1];
    let (mut app, a, _, _, _, base) = fixture(id, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    #[derive(Resource, Default)]
    struct Seen(usize);
    app.init_resource::<Seen>().add_observer(
        |event: On<CanvasPenChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            CanvasView::pen_segment(&world, event.entity).unwrap();
            CanvasView::pen_draft(&world, event.entity).unwrap();
            world.resource_mut::<Seen>().0 += 1;
        },
    );
    assert!(
        !CanvasView::set_pen_segment_in_world(app.world_mut(), a, CanvasPenSegment::Line).unwrap()
    );
    assert!(
        CanvasView::set_pen_segment_in_world(app.world_mut(), a, CanvasPenSegment::Cubic).unwrap()
    );
    assert_eq!(app.world().resource::<Seen>().0, 1);
    let logs = LogCapture::default();
    let e = logs
        .run(|| CanvasView::finish_pen_in_world(app.world_mut(), a))
        .unwrap_err();
    assert_eq!(e.severity(), bevy::ecs::error::Severity::Error);
    let errors = ErrorCapture::default();
    CanvasView::set_pen_segment(
        &mut app.world_mut().commands(),
        Entity::PLACEHOLDER,
        CanvasPenSegment::Arc,
    );
    logs.run(|| errors.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert_eq!(logs.records().len(), 2);
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Pen(CanvasPaint::default()))
        .unwrap();
    click(&mut app, id, a, &base, Vec2::ZERO);
    let count = app.world().resource::<Seen>().0;
    assert!(CanvasView::cancel_pen_in_world(app.world_mut(), a).unwrap());
    assert_eq!(app.world().resource::<Seen>().0, count + 1);
    assert!(!CanvasView::cancel_pen_in_world(app.world_mut(), a).unwrap());
    assert_eq!(app.world().resource::<Seen>().0, count + 1);
}
