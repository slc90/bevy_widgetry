#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

#[allow(dead_code)]
mod common;

use bevy::{camera::primitives::MeshAabb, picking::pointer::*, prelude::*};
use bevy_widgetry_canvas::*;
use bevy_widgetry_job::JobFailure;
use bevy_widgetry_test_utils::{press_key, queue_pointer};

fn kind(error: JobFailure) -> CanvasErrorKind {
    error
        .diagnostics()
        .next()
        .unwrap()
        .source()
        .unwrap()
        .downcast_ref::<CanvasError>()
        .unwrap()
        .kind
}

fn stop(offset: f32, color: Color) -> CanvasColorStop {
    CanvasColorStop { offset, color }
}

fn stops() -> CanvasGradientStops {
    CanvasGradientStops::new(&[
        stop(0.0, Color::BLACK),
        stop(0.5, Color::srgb(1.0, 0.0, 0.0)),
        stop(0.5, Color::srgb(0.0, 0.0, 1.0)),
        stop(1.0, Color::WHITE),
    ])
    .unwrap()
}

#[test]
fn gradients_sort_offsets_stably_and_reject_invalid_stop_lists() {
    let red = stop(0.5, Color::srgb(1.0, 0.0, 0.0));
    let blue = stop(0.5, Color::srgb(0.0, 0.0, 1.0));
    let black = stop(0.0, Color::BLACK);
    let white = stop(1.0, Color::WHITE);
    let stops = CanvasGradientStops::new(&[white, red, black, blue]).unwrap();
    assert_eq!(stops.as_slice(), &[black, red, blue, white]);
    for invalid in [
        vec![black],
        vec![black; 9],
        vec![black, stop(-0.1, Color::WHITE)],
        vec![black, stop(f32::NAN, Color::WHITE)],
        vec![
            black,
            stop(0.8, Color::linear_rgba(f32::INFINITY, 0.0, 0.0, 1.0)),
        ],
    ] {
        assert_eq!(
            kind(CanvasGradientStops::new(&invalid).unwrap_err()),
            CanvasErrorKind::Invalid(CanvasIssue::GradientStops)
        );
    }
}

#[test]
fn paint_changes_preserve_local_coordinates_and_undo_as_one_edit() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        size: Vec2::splat(40.0),
        corner_radius: 0.0,
    };
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        rectangle,
        Transform::IDENTITY,
        CanvasPaint::default(),
    )
    .unwrap();
    let invalid_linear = CanvasFill::LinearGradient {
        start: Vec2::ONE,
        end: Vec2::ONE,
        stops: stops(),
    };
    assert_eq!(
        kind(
            CanvasDocument::create_rectangle(
                &mut world,
                document,
                rectangle,
                Transform::IDENTITY,
                CanvasPaint {
                    fill: Some(invalid_linear),
                    ..default()
                }
            )
            .unwrap_err()
        ),
        CanvasErrorKind::Invalid(CanvasIssue::GradientGeometry)
    );
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let valid = CanvasPaint {
        fill: Some(CanvasFill::RadialGradient {
            center: Vec2::new(2.0, 3.0),
            radius: 16.0,
            stops: stops(),
        }),
        ..default()
    };
    CanvasDocument::edit(&mut world, document, |world, transaction| {
        CanvasDocument::update_paint(world, transaction, shape, valid)?;
        CanvasDocument::update_transform(
            world,
            transaction,
            shape,
            Transform::from_xyz(-20.0, 30.0, 0.0)
                .with_rotation(Quat::from_rotation_z(0.5))
                .with_scale(Vec3::new(2.0, -1.0, 1.0)),
        )
    })
    .unwrap();
    let invalid = CanvasPaint {
        fill: Some(CanvasFill::RadialGradient {
            center: Vec2::ZERO,
            radius: 0.0,
            stops: stops(),
        }),
        ..default()
    };
    assert_eq!(
        kind(CanvasDocument::update_paint(&mut world, document, shape, invalid).unwrap_err()),
        CanvasErrorKind::Invalid(CanvasIssue::GradientGeometry)
    );
    assert_eq!(world.get::<CanvasPaint>(shape), Some(&valid));
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        world.get::<CanvasPaint>(shape),
        Some(&CanvasPaint::default())
    );
    assert_eq!(world.get::<Transform>(shape), Some(&Transform::IDENTITY));
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(world.get::<CanvasPaint>(shape), Some(&valid));
}

#[test]
fn svg_keeps_gradient_coordinates_relative_to_the_shape_and_duplicate_stops() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(30.0),
            corner_radius: 0.0,
        },
        Transform::from_xyz(20.0, 30.0, 0.0).with_scale(Vec3::new(2.0, 3.0, 1.0)),
        CanvasPaint {
            fill: Some(CanvasFill::LinearGradient {
                start: Vec2::new(-10.0, 0.0),
                end: Vec2::new(10.0, 5.0),
                stops: stops(),
            }),
            ..default()
        },
    )
    .unwrap();
    CanvasDocument::create_ellipse(
        &mut world,
        document,
        CanvasEllipse {
            radii: Vec2::splat(16.0),
        },
        Transform::IDENTITY,
        CanvasPaint {
            fill: Some(CanvasFill::RadialGradient {
                center: Vec2::new(-5.0, 1.0),
                radius: 12.0,
                stops: stops(),
            }),
            ..default()
        },
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fills.svg");
    CanvasDocument::export_svg(&mut world, document, &path, None).unwrap();
    let svg = std::fs::read_to_string(path).unwrap();
    assert!(svg.contains("gradientUnits=\"userSpaceOnUse\""));
    assert!(svg.contains("x1=\"-10\" y1=\"0\" x2=\"10\" y2=\"5\""));
    assert!(svg.contains("cx=\"-5\" cy=\"1\" r=\"12\""));
    assert!(svg.contains("transform=\"matrix(2 0 0 3 20 30)\""));
    assert_eq!(svg.matches("offset=\"0.5\"").count(), 4);
    assert_eq!(svg.matches("fill=\"url(#canvas-gradient-").count(), 2);
}

#[test]
fn dragging_a_handle_previews_in_one_view_then_commits_once_and_reuses_fill_meshes() {
    let pointer = PointerId::Mouse;
    let (mut app, main, auxiliary, shape, _, mut at) = common::fixture(pointer, 1.0, 1.0);
    let document = CanvasView::document(app.world(), main).unwrap().unwrap();
    let fill_meshes = |world: &mut World| {
        let mut result = world
            .query::<(&Name, &Mesh2d)>()
            .iter(world)
            .filter(|(name, _)| name.as_str() == "CanvasFill")
            .map(|(_, mesh)| mesh.0.id())
            .collect::<Vec<_>>();
        result.sort_unstable();
        result
    };
    let meshes = fill_meshes(app.world_mut());
    assert_eq!(meshes.len(), 2);
    let original = CanvasPaint {
        fill: Some(CanvasFill::LinearGradient {
            start: Vec2::new(-10.0, 0.0),
            end: Vec2::new(10.0, 0.0),
            stops: stops(),
        }),
        ..default()
    };
    CanvasDocument::update_paint(app.world_mut(), document, shape, original).unwrap();
    CanvasDocument::clear_history(app.world_mut(), document).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), main, CanvasMode::GradientEdit).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), main, [shape]).unwrap();
    common::update(&mut app);
    let overlay = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .filter(|(_, name)| name.as_str() == "CanvasGradientHandles")
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    assert_eq!(overlay.len(), 1);
    assert_eq!(
        CanvasView::mode(app.world(), auxiliary).unwrap(),
        CanvasMode::Select
    );
    let max_x = |app: &App| {
        let mesh = app.world().get::<Mesh2d>(overlay[0]).unwrap();
        let meshes = app.world().resource::<Assets<Mesh>>();
        meshes.get(&mesh.0).unwrap().get_aabb().unwrap().max().x
    };
    let first = max_x(&app);
    common::moved(&mut app, pointer, &mut at, Vec2::new(10.0, 0.0));
    common::input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    common::moved(&mut app, pointer, &mut at, Vec2::new(18.0, 6.0));
    assert_eq!(app.world().get::<CanvasPaint>(shape), Some(&original));
    assert!(max_x(&app) > first + 12.0);
    assert_eq!(fill_meshes(app.world_mut()), meshes);
    common::input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert!(matches!(
        app.world().get::<CanvasPaint>(shape).unwrap().fill,
        Some(CanvasFill::LinearGradient { end, .. }) if end.abs_diff_eq(Vec2::new(28.0, 6.0), 0.001)
    ));
    assert_eq!(fill_meshes(app.world_mut()), meshes);
    assert!(CanvasDocument::undo(app.world_mut(), document).unwrap());
    assert_eq!(app.world().get::<CanvasPaint>(shape), Some(&original));
    assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
    assert!(CanvasDocument::redo(app.world_mut(), document).unwrap());
    let committed = *app.world().get::<CanvasPaint>(shape).unwrap();
    common::input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    common::moved(&mut app, pointer, &mut at, Vec2::new(-8.0, 12.0));
    common::input(&mut app, pointer, &at, PointerAction::Cancel);
    assert_eq!(app.world().get::<CanvasPaint>(shape), Some(&committed));
    app.world_mut().despawn(main);
    common::update(&mut app);
    assert_eq!(fill_meshes(app.world_mut()), meshes[..1]);
}

fn linear_edit() -> (App, Entity, Entity, Entity, Entity, CanvasPaint, Location) {
    let pointer = PointerId::Mouse;
    let (mut app, view, _, shape, window, mut at) = common::fixture(pointer, 1.0, 1.0);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    let paint = CanvasPaint {
        fill: Some(CanvasFill::LinearGradient {
            start: Vec2::new(-10.0, 0.0),
            end: Vec2::new(10.0, 0.0),
            stops: stops(),
        }),
        ..default()
    };
    CanvasDocument::update_paint(app.world_mut(), document, shape, paint).unwrap();
    CanvasDocument::clear_history(app.world_mut(), document).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::GradientEdit).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, [shape]).unwrap();
    common::update(&mut app);
    at.position.x += 10.0;
    (app, view, document, shape, window, paint, at)
}

#[test]
fn escape_cancels_the_gradient_gesture_before_the_pointer_releases() {
    let pointer = PointerId::Mouse;
    let (mut app, view, document, shape, window, paint, mut at) = linear_edit();
    common::input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    common::input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    common::moved(&mut app, pointer, &mut at, Vec2::new(12.0, 4.0));
    assert!(
        app.world()
            .resource::<bevy::picking::hover::PointerCaptureMap>()
            .is_captured(&pointer)
    );
    press_key(&mut app, window, KeyCode::Escape);
    assert!(
        !app.world()
            .resource::<bevy::picking::hover::PointerCaptureMap>()
            .is_captured(&pointer)
    );
    assert_eq!(
        CanvasView::mode(app.world(), view).unwrap(),
        CanvasMode::GradientEdit
    );
    common::input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(app.world().get::<CanvasPaint>(shape), Some(&paint));
    assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
}

#[test]
fn escape_bubbles_from_an_idle_gradient_editor() {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        input_focus::FocusedInput,
    };

    #[derive(Default, Resource)]
    struct AncestorEscapes(usize);

    let (mut app, view, _, _, window, _, _) = linear_edit();
    app.init_resource::<AncestorEscapes>();
    let ancestor = app
        .world_mut()
        .spawn_empty()
        .observe(
            |event: On<FocusedInput<KeyboardInput>>, mut received: ResMut<AncestorEscapes>| {
                if event.input.key_code == KeyCode::Escape {
                    received.0 += 1;
                }
            },
        )
        .id();
    app.world_mut().entity_mut(view).insert(ChildOf(ancestor));
    app.world_mut()
        .insert_resource(bevy::input_focus::InputFocus::from_entity(view));
    app.world_mut().trigger(FocusedInput::new(
        view,
        KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
        window,
    ));
    assert_eq!(app.world().resource::<AncestorEscapes>().0, 1);
    assert_eq!(
        CanvasView::mode(app.world(), view).unwrap(),
        CanvasMode::GradientEdit
    );
}

#[test]
fn first_frame_gradient_drag_commits_the_release_position_after_entering_the_view() {
    let pointer = PointerId::Mouse;
    let (mut app, _, document, shape, _, _, start) = linear_edit();
    let mut outside = start.clone();
    outside.position = Vec2::new(1000.0, 700.0);
    common::input(
        &mut app,
        pointer,
        &outside,
        PointerAction::Move { delta: Vec2::ONE },
    );
    let delta = Vec2::new(12.0, 4.0);
    let mut end = start.clone();
    end.position += delta;
    for (action, at) in [
        (
            PointerAction::Move {
                delta: start.position - outside.position,
            },
            &start,
        ),
        (PointerAction::Press(PointerButton::Primary), &start),
        (PointerAction::Move { delta }, &end),
        (PointerAction::Release(PointerButton::Primary), &end),
    ] {
        queue_pointer(&mut app, pointer, at.clone(), action);
    }
    common::update(&mut app);
    assert!(matches!(
        app.world().get::<CanvasPaint>(shape).unwrap().fill,
        Some(CanvasFill::LinearGradient { end, .. }) if end.abs_diff_eq(Vec2::new(22.0, 4.0), 0.001)
    ));
    assert!(CanvasDocument::undo(app.world_mut(), document).unwrap());
    assert!(!CanvasDocument::can_undo(app.world(), document).unwrap());
}
