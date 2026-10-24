#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, pointer_ids};
mod common;
use common::*;

fn shapes(app: &mut App, document: Entity) -> Vec<Entity> {
    app.world_mut()
        .query::<(Entity, &ChildOf, &CanvasRectangle)>()
        .iter(app.world())
        .filter(|(_, parent, _)| parent.parent() == document)
        .map(|(entity, _, _)| entity)
        .collect()
}

fn previews(app: &mut App) -> usize {
    app.world_mut()
        .query::<&Name>()
        .iter(app.world())
        .filter(|name| name.as_str() == "CanvasRectanglePreview")
        .count()
}

#[test]
fn rectangle_tool_previews_locally_and_commits_on_release_in_all_four_directions() {
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
                    color: Color::BLACK,
                    width: 2.0,
                }),
            };
            CanvasView::set_selection_in_world(app.world_mut(), b, Some(original)).unwrap();
            CanvasView::set_zoom_in_world(app.world_mut(), a, 2.0).unwrap();
            CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Rectangle(paint))
                .unwrap();
            moved(&mut app, id, &mut at, Vec2::new(60.0, 30.0));
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            moved(&mut app, id, &mut at, delta * 2.0);
            assert_eq!(shapes(&mut app, document), [original]);
            assert_eq!(previews(&mut app), 1);
            scroll(&mut app, id, &at, 1.0);
            assert_eq!(shapes(&mut app, document), [original]);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            let selected = CanvasView::selection(app.world(), a).unwrap().unwrap();
            assert_ne!(selected, original);
            assert_eq!(shapes(&mut app, document).len(), 2);
            assert!(
                (app.world().get::<CanvasRectangle>(selected).unwrap().size - delta.abs()).length()
                    < 0.001
            );
            let center = app
                .world()
                .get::<Transform>(selected)
                .unwrap()
                .translation
                .truncate();
            assert!((center - (Vec2::new(30.0, 15.0) + delta * 0.5)).length() < 0.001);
            assert_eq!(*app.world().get::<CanvasPaint>(selected).unwrap(), paint);
            assert_eq!(
                CanvasView::mode(app.world(), a).unwrap(),
                CanvasMode::Select
            );
            assert_eq!(
                CanvasView::mode(app.world(), b).unwrap(),
                CanvasMode::Select
            );
            assert_eq!(
                CanvasView::selection(app.world(), b).unwrap(),
                Some(original)
            );
            assert_eq!(previews(&mut app), 0);
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn invalid_click_escape_cancel_mode_changes_and_view_lifecycle_never_commit_a_preview() {
    for cause in 0..8 {
        let id = pointer_ids()[1];
        let (mut app, a, _, original, window, mut at) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let initial_meshes = app.world().resource::<Assets<Mesh>>().len();
        let initial_materials = app.world().resource::<Assets<ColorMaterial>>().len();
        CanvasView::set_mode_in_world(
            app.world_mut(),
            a,
            CanvasMode::Rectangle(CanvasPaint::default()),
        )
        .unwrap();
        moved(&mut app, id, &mut at, Vec2::new(40.0, 20.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        moved(
            &mut app,
            id,
            &mut at,
            if cause == 0 {
                Vec2::splat(0.5)
            } else {
                Vec2::new(20.0, 10.0)
            },
        );
        match cause {
            0 => input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            ),
            1 => {
                bevy_widgetry_test_utils::press_key(&mut app, window, KeyCode::Escape);
            }
            2 => input(&mut app, id, &at, PointerAction::Cancel),
            3 => {
                app.world_mut().entity_mut(a).insert(Visibility::Hidden);
                update(&mut app);
            }
            4 => {
                app.world_mut().despawn(a);
                update(&mut app);
            }
            5 => {
                app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
                update(&mut app);
            }
            6 => {
                CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Select).unwrap();
                update(&mut app);
            }
            _ => {
                CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
                update(&mut app);
            }
        }
        assert!(
            !app.world().resource::<PointerCaptureMap>().is_captured(&id),
            "cause {cause}"
        );
        assert_eq!(previews(&mut app), 0);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(shapes(&mut app, document), [original]);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), initial_meshes);
        assert_eq!(
            app.world().resource::<Assets<ColorMaterial>>().len(),
            initial_materials - if cause == 4 || cause == 7 { 2 } else { 0 }
        );
        if cause == 0 || cause == 1 {
            assert_eq!(
                CanvasView::mode(app.world(), a).unwrap(),
                CanvasMode::Select
            );
        }
    }
}

#[test]
fn first_frame_press_move_release_commits_after_previous_hover_was_outside_canvas() {
    for id in pointer_ids() {
        let (mut app, a, _, _, _, mut at) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let mut outside = at.clone();
        outside.position = Vec2::new(1000.0, 700.0);
        input(
            &mut app,
            id,
            &outside,
            PointerAction::Move { delta: Vec2::ONE },
        );
        CanvasView::set_mode_in_world(
            app.world_mut(),
            a,
            CanvasMode::Rectangle(CanvasPaint::default()),
        )
        .unwrap();
        for action in [
            PointerAction::Move { delta: Vec2::ONE },
            PointerAction::Press(PointerButton::Primary),
        ] {
            bevy_widgetry_test_utils::queue_pointer(&mut app, id, at.clone(), action);
        }
        at.position += Vec2::new(20.0, 10.0);
        for action in [
            PointerAction::Move {
                delta: Vec2::new(20.0, 10.0),
            },
            PointerAction::Release(PointerButton::Primary),
        ] {
            bevy_widgetry_test_utils::queue_pointer(&mut app, id, at.clone(), action);
        }
        update(&mut app);
        assert_eq!(shapes(&mut app, document).len(), 2);
        assert_eq!(
            CanvasView::mode(app.world(), a).unwrap(),
            CanvasMode::Select
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        assert_eq!(previews(&mut app), 0);
    }
}

#[test]
fn completion_mode_observers_can_remove_or_rebind_the_view_without_creating_stale_shapes() {
    for remove in [false, true] {
        let id = pointer_ids()[1];
        let (mut app, a, _, original, _, mut at) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        CanvasView::set_mode_in_world(
            app.world_mut(),
            a,
            CanvasMode::Rectangle(CanvasPaint::default()),
        )
        .unwrap();
        app.add_observer(
            move |event: On<CanvasModeChanged>, mut commands: Commands| {
                if event.entity == a && event.mode == CanvasMode::Select {
                    if remove {
                        commands.entity(a).despawn();
                    } else {
                        CanvasView::set_document(&mut commands, a, None);
                    }
                }
            },
        );
        moved(&mut app, id, &mut at, Vec2::new(40.0, 20.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        moved(&mut app, id, &mut at, Vec2::new(20.0, 10.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(shapes(&mut app, document), [original]);
        assert_eq!(previews(&mut app), 0);
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    }
}

#[test]
fn first_frame_click_without_drag_returns_to_select_even_if_release_targets_old_hover() {
    for id in pointer_ids() {
        for repress in [false, true] {
            let (mut app, a, _, original, _, at) = fixture(id, 1.0, 1.0);
            let document = CanvasView::document(app.world(), a).unwrap().unwrap();
            let mut outside = at.clone();
            outside.position = Vec2::new(1000.0, 700.0);
            input(
                &mut app,
                id,
                &outside,
                PointerAction::Move { delta: Vec2::ONE },
            );
            CanvasView::set_mode_in_world(
                app.world_mut(),
                a,
                CanvasMode::Rectangle(CanvasPaint::default()),
            )
            .unwrap();
            for action in [
                PointerAction::Move { delta: Vec2::ONE },
                PointerAction::Press(PointerButton::Primary),
                PointerAction::Release(PointerButton::Primary),
            ] {
                bevy_widgetry_test_utils::queue_pointer(&mut app, id, at.clone(), action);
            }
            if repress {
                for action in [
                    PointerAction::Press(PointerButton::Primary),
                    PointerAction::Move {
                        delta: Vec2::new(20.0, 10.0),
                    },
                    PointerAction::Release(PointerButton::Primary),
                ] {
                    let mut next = at.clone();
                    if matches!(action, PointerAction::Move { .. }) {
                        next.position += Vec2::new(20.0, 10.0);
                    }
                    bevy_widgetry_test_utils::queue_pointer(&mut app, id, next, action);
                }
            }
            update(&mut app);
            assert_eq!(
                CanvasView::mode(app.world(), a).unwrap(),
                CanvasMode::Select
            );
            assert_eq!(shapes(&mut app, document), [original]);
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn new_same_frame_drawing_reuses_preview_with_its_own_colors_and_stroke_width() {
    for fill in [
        Some(Color::srgb(0.0, 1.0, 0.0)),
        Some(Color::srgba(0.0, 1.0, 0.0, 0.5)),
        None,
    ] {
        let id = pointer_ids()[1];
        let (mut app, a, _, _, _, mut at) = fixture(id, 1.0, 1.0);
        let paint = CanvasPaint {
            fill: Some(Color::srgb(1.0, 0.0, 0.0)),
            stroke: Some(CanvasStroke {
                color: Color::BLACK,
                width: 2.0,
            }),
        };
        let next = CanvasPaint {
            fill,
            stroke: Some(CanvasStroke {
                color: Color::srgb(0.0, 0.0, 1.0),
                width: 8.0,
            }),
        };
        CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Rectangle(paint)).unwrap();
        moved(&mut app, id, &mut at, Vec2::new(40.0, 20.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        moved(&mut app, id, &mut at, Vec2::new(20.0, 10.0));
        app.add_observer(move |e: On<CanvasModeChanged>, mut commands: Commands| {
            if e.entity == a && e.mode == CanvasMode::Select {
                CanvasView::set_mode(&mut commands, a, CanvasMode::Rectangle(next));
            }
        });
        for action in [
            PointerAction::Release(PointerButton::Primary),
            PointerAction::Press(PointerButton::Primary),
        ] {
            bevy_widgetry_test_utils::queue_pointer(&mut app, id, at.clone(), action);
        }
        moved(&mut app, id, &mut at, Vec2::new(20.0, 10.0));
        let preview = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "CanvasRectanglePreview")
            .unwrap()
            .0;
        let children = app.world().get::<Children>(preview).unwrap();
        let (fill, stroke) = (children[0], children[1]);
        for (entity, color) in [
            (fill, next.fill.unwrap_or(Color::NONE)),
            (stroke, next.stroke.unwrap().color),
        ] {
            let material = &app
                .world()
                .get::<MeshMaterial2d<ColorMaterial>>(entity)
                .unwrap()
                .0;
            assert_eq!(
                app.world()
                    .resource::<Assets<ColorMaterial>>()
                    .get(material)
                    .unwrap()
                    .color,
                color
            );
            assert_eq!(
                app.world()
                    .resource::<Assets<ColorMaterial>>()
                    .get(material)
                    .unwrap()
                    .alpha_mode,
                ColorMaterial::from(color).alpha_mode
            );
        }
        let mesh = &app.world().get::<Mesh2d>(stroke).unwrap().0;
        let positions = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(mesh)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        assert_eq!(positions[0], [-14.0, -9.0, 0.0]);
        input(&mut app, id, &at, PointerAction::Cancel);
    }
}

#[test]
fn space_pan_has_priority_over_rectangle_tool_and_mode_api_commits_before_notice() {
    let id = pointer_ids()[1];
    let (mut app, a, b, original, _, mut at) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    #[derive(Resource, Default)]
    struct Seen(Vec<CanvasMode>);
    app.init_resource::<Seen>().add_observer(
        |event: On<CanvasModeChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(CanvasView::mode(&world, event.entity).unwrap(), event.mode);
            world.resource_mut::<Seen>().0.push(event.mode);
        },
    );
    let mode = CanvasMode::Rectangle(CanvasPaint::default());
    assert!(CanvasView::set_mode_in_world(app.world_mut(), a, mode).unwrap());
    assert!(!CanvasView::set_mode_in_world(app.world_mut(), a, mode).unwrap());
    assert_eq!(
        CanvasView::mode(app.world(), b).unwrap(),
        CanvasMode::Select
    );
    let invalid = CanvasMode::Rectangle(CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            color: Color::BLACK,
            width: f32::NAN,
        }),
    });
    let logs = LogCapture::default();
    logs.run(|| {
        assert_eq!(
            CanvasView::set_mode_in_world(app.world_mut(), a, invalid)
                .unwrap_err()
                .severity(),
            bevy::ecs::error::Severity::Error
        );
    });
    let errors = ErrorCapture::default();
    CanvasView::set_mode(&mut app.world_mut().commands(), Entity::PLACEHOLDER, mode);
    logs.run(|| errors.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert_eq!(logs.records().len(), 2);
    moved(&mut app, id, &mut at, Vec2::ZERO);
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
    assert_eq!(
        CanvasView::center(app.world(), a).unwrap(),
        Vec2::new(-20.0, -10.0)
    );
    assert_eq!(shapes(&mut app, document), [original]);
    assert_eq!(previews(&mut app), 0);
    assert_eq!(CanvasView::mode(app.world(), a).unwrap(), mode);
    CanvasView::set_mode(&mut app.world_mut().commands(), a, CanvasMode::Select);
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Seen>().0, [mode, CanvasMode::Select]);
}
