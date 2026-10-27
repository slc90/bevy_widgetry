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
use coordinates::at_document;

fn fixture_shape(kind: usize, id: PointerId) -> (App, Entity, Entity, Entity, Location) {
    let (mut app, a, b, old, _, base) = fixture(id, 1.5, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    CanvasDocument::remove_shape(app.world_mut(), document, old).unwrap();
    let transform = Transform::from_xyz(-5.0, 4.0, 3.0)
        .with_rotation(Quat::from_rotation_z(0.3))
        .with_scale(Vec3::new(-1.2, 0.8, 0.0));
    let paint = CanvasPaint {
        stroke: Some(CanvasStroke {
            width: 2.0,
            cap: CanvasLineCap::Round,
            ..default()
        }),
        ..default()
    };
    let shape = match kind {
        0 => CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            CanvasRectangle {
                size: Vec2::new(40.0, 30.0),
            },
            transform,
            paint,
        ),
        1 => CanvasDocument::create_ellipse(
            app.world_mut(),
            document,
            CanvasEllipse {
                radii: Vec2::new(20.0, 15.0),
            },
            transform,
            paint,
        ),
        _ => CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    CanvasPathCommand::MoveTo(Vec2::new(-20.0, -10.0)),
                    CanvasPathCommand::LineTo(Vec2::new(20.0, -10.0)),
                    CanvasPathCommand::LineTo(Vec2::new(0.0, 15.0)),
                    CanvasPathCommand::Close,
                    CanvasPathCommand::MoveTo(Vec2::new(30.0, 20.0)),
                    CanvasPathCommand::LineTo(Vec2::new(45.0, 20.0)),
                ],
            },
            transform,
            paint,
        ),
    }
    .unwrap();
    CanvasView::set_zoom_in_world(app.world_mut(), b, 0.75).unwrap();
    CanvasView::set_center_in_world(app.world_mut(), b, Vec2::new(7.0, 3.0)).unwrap();
    update(&mut app);
    (app, a, b, shape, base)
}

fn near(a: Vec2, b: Vec2) {
    assert!(a.distance(b) < 0.003, "actual={a:?}, expected={b:?}");
}

fn outline(app: &mut App) -> Vec<[f32; 3]> {
    let handle = app
        .world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .find(|(n, _)| n.as_str() == "CanvasSelectionOutline")
        .unwrap()
        .1
        .0
        .clone();
    app.world()
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap()
        .to_vec()
}

fn handle(points: &[[f32; 3]], index: usize) -> Vec2 {
    let points = &points[8 + index * 4..12 + index * 4];
    points.iter().map(|p| Vec2::new(p[0], p[1])).sum::<Vec2>() * 0.25
}

fn paint_meshes(app: &mut App) -> Vec<AssetId<Mesh>> {
    app.world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .filter(|(name, _)| ["CanvasFill", "CanvasStroke"].contains(&name.as_str()))
        .map(|(_, mesh)| mesh.0.id())
        .collect()
}

fn unchanged_meshes(
    app: &App,
    reader: &mut bevy::ecs::message::MessageCursor<AssetEvent<Mesh>>,
    ids: &[AssetId<Mesh>],
) {
    assert!(
        !reader
            .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
            .any(|event| matches!(event,AssetEvent::Modified {id} if ids.contains(id)))
    );
}

#[test]
fn transform_api_validates_all_shape_kinds_without_replacing_geometry_or_order() {
    for kind in 0..3 {
        let (mut app, view, _, shape, _) = fixture_shape(kind, pointer_ids()[1]);
        let document = CanvasView::document(app.world(), view).unwrap().unwrap();
        let geometry = app.world().get::<CanvasGeometry>(shape).unwrap().clone();
        let paint = *app.world().get::<CanvasPaint>(shape).unwrap();
        let transform = Transform::from_xyz(20.0, -15.0, 8.0)
            .with_rotation(Quat::from_rotation_z(0.5))
            .with_scale(Vec3::new(1.5, -0.8, 0.0));
        CanvasDocument::update_transform(app.world_mut(), document, shape, transform).unwrap();
        assert_eq!(*app.world().get::<Transform>(shape).unwrap(), transform);
        assert_eq!(app.world().get::<CanvasGeometry>(shape), Some(&geometry));
        assert_eq!(*app.world().get::<CanvasPaint>(shape).unwrap(), paint);
        assert_eq!(
            CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
            [shape]
        );
        let other = app.world_mut().spawn(CanvasDocument).id();
        for (doc, transform) in [
            (document, Transform::from_scale(Vec3::new(0.0, 1.0, 1.0))),
            (document, Transform::from_xyz(f32::INFINITY, 0.0, 0.0)),
            (other, Transform::default()),
        ] {
            let previous = *app.world().get::<Transform>(shape).unwrap();
            let logs = LogCapture::default();
            let error = logs
                .run(|| CanvasDocument::update_transform(app.world_mut(), doc, shape, transform))
                .unwrap_err();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            assert!(
                logs.records()
                    .iter()
                    .any(|r| r.level == bevy::log::Level::ERROR)
            );
            assert_eq!(*app.world().get::<Transform>(shape).unwrap(), previous);
            assert_eq!(app.world().get::<CanvasGeometry>(shape), Some(&geometry));
        }
    }
}

#[test]
fn first_press_selects_then_whole_shape_movement_preserves_geometry_and_shared_meshes() {
    for id in pointer_ids() {
        for kind in 0..3 {
            let (mut app, a, b, shape, base) = fixture_shape(kind, id);
            let geometry = app.world().get::<CanvasGeometry>(shape).unwrap().clone();
            let original = *app.world().get::<Transform>(shape).unwrap();
            let paint = *app.world().get::<CanvasPaint>(shape).unwrap();
            let mut at = at_document(&app, a, &base, original.translation.truncate());
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            moved(&mut app, id, &mut at, Vec2::new(3.0, 2.0));
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert_eq!(CanvasView::selection(app.world(), a).unwrap(), Some(shape));
            assert_eq!(*app.world().get::<Transform>(shape).unwrap(), original);
            at = at_document(&app, a, &base, original.translation.truncate());
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            let ids = paint_meshes(&mut app);
            let mut reader = app
                .world()
                .resource::<Messages<AssetEvent<Mesh>>>()
                .get_cursor_current();
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            let target = original.translation.truncate() + Vec2::new(15.0, -8.0);
            let next = at_document(&app, a, &base, target);
            let delta = next.position - at.position;
            at = next;
            input(&mut app, id, &at, PointerAction::Move { delta });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            let actual = *app.world().get::<Transform>(shape).unwrap();
            near(actual.translation.truncate(), target);
            assert_eq!(actual.rotation, original.rotation);
            assert_eq!(actual.scale, original.scale);
            assert_eq!(actual.translation.z, original.translation.z);
            assert_eq!(app.world().get::<CanvasGeometry>(shape), Some(&geometry));
            assert_eq!(*app.world().get::<CanvasPaint>(shape).unwrap(), paint);
            assert_eq!(CanvasView::selection(app.world(), b).unwrap(), None);
            assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 0.75);
            near(
                CanvasView::center(app.world(), b).unwrap(),
                Vec2::new(7.0, 3.0),
            );
            unchanged_meshes(&app, &mut reader, &ids);
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn all_shapes_scale_nonuniformly_through_zero_to_a_mirror_around_the_fixed_opposite_corner() {
    for id in pointer_ids() {
        for kind in 0..3 {
            let (mut app, a, b, shape, base) = fixture_shape(kind, id);
            CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
            update(&mut app);
            let geometry = app.world().get::<CanvasGeometry>(shape).unwrap().clone();
            let original = *app.world().get::<Transform>(shape).unwrap();
            let points = outline(&mut app);
            let anchor = handle(&points, 0);
            let start = handle(&points, 4);
            let inverse = |p: Vec2| {
                (original.rotation.conjugate() * (p - original.translation.truncate()).extend(0.0))
                    .truncate()
                    / original.scale.truncate()
            };
            let low = inverse(anchor);
            let high = inverse(start);
            let ids = paint_meshes(&mut app);
            let mut reader = app
                .world()
                .resource::<Messages<AssetEvent<Mesh>>>()
                .get_cursor_current();
            let mut at = at_document(&app, a, &base, start);
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            for factors in [
                Vec2::new(1.5, 0.5),
                Vec2::new(0.0, 1.0),
                Vec2::new(-0.75, 1.25),
            ] {
                let local = low + (high - low) * factors;
                let target = original.transform_point(local.extend(0.0)).truncate();
                let next = at_document(&app, a, &base, target);
                let delta = next.position - at.position;
                at = next;
                input(&mut app, id, &at, PointerAction::Move { delta });
                let actual = *app.world().get::<Transform>(shape).unwrap();
                if factors.x != 0.0 {
                    near(actual.scale.truncate(), original.scale.truncate() * factors);
                }
                assert!(actual.scale.x != 0.0 && actual.scale.y != 0.0);
                near(actual.transform_point(low.extend(0.0)).truncate(), anchor);
                assert_eq!(actual.rotation, original.rotation);
                assert_eq!(actual.translation.z, original.translation.z);
                assert_eq!(app.world().get::<CanvasGeometry>(shape), Some(&geometry));
                assert_eq!(CanvasView::selection(app.world(), b).unwrap(), None);
                unchanged_meshes(&app, &mut reader, &ids);
            }
            let submitted = *app.world().get::<Transform>(shape).unwrap();
            input(&mut app, id, &at, PointerAction::Cancel);
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            moved(&mut app, id, &mut at, Vec2::ONE);
            assert_eq!(*app.world().get::<Transform>(shape).unwrap(), submitted);
        }
    }
}

#[test]
fn whole_shape_rotation_keeps_the_frame_center_geometry_and_shared_meshes() {
    for id in pointer_ids() {
        for kind in 0..3 {
            let (mut app, a, _, shape, base) = fixture_shape(kind, id);
            CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
            update(&mut app);
            let geometry = app.world().get::<CanvasGeometry>(shape).unwrap().clone();
            let original = *app.world().get::<Transform>(shape).unwrap();
            let points = outline(&mut app);
            let center = (handle(&points, 0) + handle(&points, 4)) * 0.5;
            let local_center = (original.rotation.conjugate()
                * (center - original.translation.truncate()).extend(0.0))
            .truncate()
                / original.scale.truncate();
            let knob = Vec2::new(points[44][0], points[44][1]);
            let direction = knob - center;
            let ids = paint_meshes(&mut app);
            let mut reader = app
                .world()
                .resource::<Messages<AssetEvent<Mesh>>>()
                .get_cursor_current();
            let mut at = at_document(&app, a, &base, knob);
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            let angle = 0.4_f32;
            let target = center + Rot2::radians(angle) * direction;
            let next = at_document(&app, a, &base, target);
            let delta = next.position - at.position;
            at = next;
            input(&mut app, id, &at, PointerAction::Move { delta });
            let actual = *app.world().get::<Transform>(shape).unwrap();
            near(
                (actual.rotation * Vec3::X).truncate(),
                (Quat::from_rotation_z(0.3 + angle) * Vec3::X).truncate(),
            );
            near(
                actual.transform_point(local_center.extend(0.0)).truncate(),
                center,
            );
            assert_eq!(actual.scale, original.scale);
            assert_eq!(actual.translation.z, original.translation.z);
            assert_eq!(app.world().get::<CanvasGeometry>(shape), Some(&geometry));
            unchanged_meshes(&app, &mut reader, &ids);
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
fn every_shape_gesture_releases_capture_on_invalidation_and_preserves_submitted_transforms() {
    let id = pointer_ids()[1];
    for kind in 1..3 {
        for action in 0..3 {
            for cause in 0..10 {
                let (mut app, a, b, shape, base) = fixture_shape(kind, id);
                let document = CanvasView::document(app.world(), a).unwrap().unwrap();
                CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
                update(&mut app);
                let points = outline(&mut app);
                let start = match action {
                    0 => app
                        .world()
                        .get::<Transform>(shape)
                        .unwrap()
                        .translation
                        .truncate(),
                    1 => handle(&points, 4),
                    _ => Vec2::new(points[44][0], points[44][1]),
                };
                let mut at = at_document(&app, a, &base, start);
                input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
                input(
                    &mut app,
                    id,
                    &at,
                    PointerAction::Press(PointerButton::Primary),
                );
                assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
                moved(&mut app, id, &mut at, Vec2::new(8.0, 4.0));
                let submitted = *app.world().get::<Transform>(shape).unwrap();
                match cause {
                    0 => input(&mut app, id, &at, PointerAction::Cancel),
                    1 => app
                        .world_mut()
                        .resource_mut::<bevy::input_focus::InputFocus>()
                        .set(b, bevy::input_focus::FocusCause::Navigated),
                    2 => {
                        let window = app
                            .world_mut()
                            .query::<(Entity, &Window)>()
                            .iter(app.world())
                            .next()
                            .unwrap()
                            .0;
                        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
                    }
                    3 => app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None,
                    4 => {
                        CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
                    }
                    5 => {
                        CanvasDocument::remove_shape(app.world_mut(), document, shape).unwrap();
                    }
                    6 => {
                        CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
                    }
                    7 => {
                        app.world_mut().entity_mut(a).insert(Visibility::Hidden);
                    }
                    8 => {
                        app.world_mut()
                            .entity_mut(a)
                            .insert(bevy::ui::InteractionDisabled);
                    }
                    _ => {
                        app.world_mut().despawn(a);
                    }
                }
                update(&mut app);
                assert!(
                    !app.world().resource::<PointerCaptureMap>().is_captured(&id),
                    "kind={kind}, action={action}, cause={cause}"
                );
                moved(&mut app, id, &mut at, Vec2::new(5.0, 3.0));
                if cause != 5 {
                    assert_eq!(*app.world().get::<Transform>(shape).unwrap(), submitted);
                } else {
                    assert!(!app.world().entities().contains(shape));
                }
            }
        }
    }
}

#[test]
fn a_zero_height_path_frame_can_scale_its_nonzero_axis_without_dividing_by_zero() {
    let id = pointer_ids()[1];
    let (mut app, view, _, old, _, base) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasDocument::remove_shape(app.world_mut(), document, old).unwrap();
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::new(-20.0, 0.0)),
                CanvasPathCommand::LineTo(Vec2::new(20.0, 0.0)),
            ],
        },
        Transform::default(),
        CanvasPaint {
            fill: None,
            stroke: None,
            ..default()
        },
    )
    .unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
    update(&mut app);
    let mut at = at_document(&app, view, &base, Vec2::new(20.0, 0.0));
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, id, &mut at, Vec2::new(20.0, 0.0));
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let transform = app.world().get::<Transform>(shape).unwrap();
    near(transform.scale.truncate(), Vec2::new(1.5, 1.0));
    near(
        transform
            .transform_point(Vec3::new(-20.0, 0.0, 0.0))
            .truncate(),
        Vec2::new(-20.0, 0.0),
    );
}
