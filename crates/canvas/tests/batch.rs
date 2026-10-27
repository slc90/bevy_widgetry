#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::LogCapture;

fn fixture() -> (World, Entity, Vec<Entity>) {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let a = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(20.0),
            corner_radius: 0.0,
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let b = CanvasDocument::create_ellipse(
        &mut world,
        document,
        CanvasEllipse {
            radii: Vec2::new(15.0, 10.0),
        },
        Transform::from_xyz(40.0, 0.0, 4.0)
            .with_rotation(Quat::from_rotation_z(0.3))
            .with_scale(Vec3::new(-2.0, 0.8, 1.0)),
        CanvasPaint::default(),
    )
    .unwrap();
    let c = CanvasDocument::create_path(
        &mut world,
        document,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::new(-10.0, 0.0)),
                CanvasPathCommand::LineTo(Vec2::new(10.0, 20.0)),
            ],
        },
        Transform::from_xyz(-40.0, 0.0, 0.0),
        CanvasPaint::default(),
    )
    .unwrap();
    (world, document, vec![a, b, c])
}
#[test]
fn batch_transform_updates_validate_every_member_before_writing() {
    let (mut world, document, shapes) = fixture();
    let updates = shapes
        .iter()
        .map(|s| {
            let mut t = *world.get::<Transform>(*s).unwrap();
            t.translation += Vec3::new(10.0, 5.0, 0.0);
            (*s, t)
        })
        .collect::<Vec<_>>();
    CanvasDocument::update_transforms(&mut world, document, &updates).unwrap();
    for (shape, t) in &updates {
        assert_eq!(world.get::<Transform>(*shape), Some(t));
    }
    let before = updates.clone();
    let mut invalid = updates.clone();
    invalid[0].1.translation.x += 10.0;
    invalid[2].1.scale.x = 0.0;
    let logs = LogCapture::default();
    let error = logs
        .run(|| CanvasDocument::update_transforms(&mut world, document, &invalid))
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(logs.records().len(), 1);
    for (shape, t) in &before {
        assert_eq!(world.get::<Transform>(*shape), Some(t));
    }
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        shapes
    );
}

#[test]
fn batch_deletion_rejects_foreign_members_then_removes_the_complete_valid_set_in_order() {
    let (mut world, document, shapes) = fixture();
    let logs = LogCapture::default();
    let error = logs
        .run(|| {
            CanvasDocument::remove_shapes(&mut world, document, &[shapes[0], Entity::PLACEHOLDER])
        })
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        shapes
    );
    assert_eq!(logs.records().len(), 1);
    CanvasDocument::remove_shapes(&mut world, document, &[shapes[2], shapes[0], shapes[2]])
        .unwrap();
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [shapes[1]]
    );
    assert!(world.get_entity(shapes[0]).is_err());
    assert!(world.get_entity(shapes[2]).is_err());
}

#[test]
fn batch_guards_reject_duplicate_updates_foreign_shapes_and_invalid_empty_targets_atomically() {
    let (mut world, document, shapes) = fixture();
    let before = shapes
        .iter()
        .map(|shape| (*shape, *world.get::<Transform>(*shape).unwrap()))
        .collect::<Vec<_>>();
    let foreign_document = world.spawn(CanvasDocument).id();
    let changed = Transform::from_xyz(25.0, 10.0, 0.0);
    for target in [document, foreign_document] {
        let updates = [(shapes[0], changed), (shapes[0], Transform::default())];
        let logs = LogCapture::default();
        let error = logs
            .run(|| CanvasDocument::update_transforms(&mut world, target, &updates))
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(logs.records().len(), 1);
        for (shape, transform) in &before {
            assert_eq!(world.get::<Transform>(*shape), Some(transform));
        }
    }
    let logs = LogCapture::default();
    let error = logs
        .run(|| CanvasDocument::remove_shapes(&mut world, foreign_document, &shapes))
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(logs.records().len(), 1);
    CanvasDocument::update_transforms(&mut world, document, &[]).unwrap();
    CanvasDocument::remove_shapes(&mut world, document, &[]).unwrap();
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        shapes
    );
    for (shape, transform) in &before {
        assert_eq!(world.get::<Transform>(*shape), Some(transform));
    }
    let logs = LogCapture::default();
    let update_error = logs
        .run(|| CanvasDocument::update_transforms(&mut world, Entity::PLACEHOLDER, &[]))
        .unwrap_err();
    let remove_error = logs
        .run(|| CanvasDocument::remove_shapes(&mut world, Entity::PLACEHOLDER, &[]))
        .unwrap_err();
    assert_eq!(update_error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(remove_error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(logs.records().len(), 2);
}

#[allow(dead_code)]
mod common;
#[path = "common/coordinates.rs"]
mod coordinates;

fn selection(app: &App, view: Entity) -> Vec<Entity> {
    CanvasView::selection(app.world(), view).unwrap()
}

#[test]
fn batch_delete_repairs_only_removed_members_in_each_view_and_preserves_navigation() {
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, pointer_ids, press_key};
    let id = pointer_ids()[1];
    let (mut app, a, b, first, window, at) = common::fixture(id, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let mut additional = Vec::new();
    for x in [40.0, 80.0] {
        additional.push(
            CanvasDocument::create_ellipse(
                app.world_mut(),
                document,
                CanvasEllipse {
                    radii: Vec2::splat(8.0),
                },
                Transform::from_xyz(x, 0.0, 0.0),
                CanvasPaint::default(),
            )
            .unwrap(),
        );
    }
    let [second, survivor] = [additional[0], additional[1]];
    CanvasView::set_selection_in_world(app.world_mut(), a, [first, second]).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), b, [second, survivor]).unwrap();
    CanvasView::set_zoom_in_world(app.world_mut(), b, 0.5).unwrap();
    CanvasView::set_center_in_world(app.world_mut(), b, Vec2::new(13.0, -7.0)).unwrap();
    common::update(&mut app);
    common::input(
        &mut app,
        id,
        &at,
        bevy::picking::pointer::PointerAction::Move { delta: Vec2::ONE },
    );
    common::input(
        &mut app,
        id,
        &at,
        bevy::picking::pointer::PointerAction::Press(
            bevy::picking::pointer::PointerButton::Primary,
        ),
    );
    press_key(&mut app, window, KeyCode::Delete);
    common::update(&mut app);
    assert!(selection(&app, a).is_empty());
    assert_eq!(selection(&app, b), [survivor]);
    assert_eq!(
        CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
        [survivor]
    );
    assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 0.5);
    assert_eq!(
        CanvasView::center(app.world(), b).unwrap(),
        Vec2::new(13.0, -7.0)
    );
    assert!(
        !app.world()
            .resource::<bevy::picking::hover::PointerCaptureMap>()
            .is_captured(&id)
    );
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 3);
}

#[test]
fn first_frame_multi_selection_click_completes_on_release_but_not_cancel() {
    use bevy::picking::{hover::PointerCaptureMap, pointer::*};
    use bevy_widgetry_test_utils::{pointer_ids, queue_pointer};
    for id in pointer_ids() {
        for (cancel, repress) in [(false, false), (true, false), (false, true)] {
            let (mut app, a, b, first, _, at) = common::fixture(id, 1.0, 1.0);
            let document = CanvasView::document(app.world(), a).unwrap().unwrap();
            let second = CanvasDocument::create_ellipse(
                app.world_mut(),
                document,
                CanvasEllipse {
                    radii: Vec2::splat(8.0),
                },
                Transform::from_xyz(50.0, 0.0, 0.0),
                CanvasPaint::default(),
            )
            .unwrap();
            for view in [a, b] {
                CanvasView::set_selection_in_world(app.world_mut(), view, [first, second]).unwrap();
            }
            let initial_selection = selection(&app, a);
            let outside = Location {
                position: Vec2::new(1000.0, 700.0),
                ..at.clone()
            };
            common::input(
                &mut app,
                id,
                &outside,
                PointerAction::Move { delta: Vec2::ONE },
            );
            for action in [
                PointerAction::Move {
                    delta: at.position - outside.position,
                },
                PointerAction::Press(PointerButton::Primary),
                if cancel {
                    PointerAction::Cancel
                } else {
                    PointerAction::Release(PointerButton::Primary)
                },
            ] {
                queue_pointer(&mut app, id, at.clone(), action);
            }
            if repress {
                queue_pointer(
                    &mut app,
                    id,
                    at.clone(),
                    PointerAction::Press(PointerButton::Primary),
                );
            }
            common::update(&mut app);
            let expected = if cancel {
                initial_selection.clone()
            } else {
                vec![first]
            };
            assert_eq!(
                selection(&app, a),
                expected,
                "cancel={cancel}, repress={repress}"
            );
            assert_eq!(selection(&app, b), initial_selection);
            assert_eq!(
                app.world().resource::<PointerCaptureMap>().is_captured(&id),
                repress
            );
            let mut next = at.clone();
            common::moved(&mut app, id, &mut next, Vec2::new(10.0, 5.0));
            if repress {
                assert_eq!(
                    app.world().get::<Transform>(first).unwrap().translation,
                    Vec3::new(10.0, 5.0, 0.0)
                );
                common::input(
                    &mut app,
                    id,
                    &next,
                    PointerAction::Release(PointerButton::Primary),
                );
            } else {
                assert_eq!(
                    app.world().get::<Transform>(first).unwrap().translation,
                    Vec3::ZERO
                );
            }
            assert_eq!(
                app.world().get::<Transform>(second).unwrap().translation,
                Vec3::new(50.0, 0.0, 0.0)
            );
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn dragging_any_selected_shape_moves_the_set_with_one_document_delta_and_keeps_identity() {
    use bevy::{
        input_focus::InputFocus,
        picking::{hover::PointerCaptureMap, pointer::*},
    };
    use bevy_widgetry_test_utils::pointer_ids;
    for id in pointer_ids() {
        let (mut app, a, b, first, _, base) = common::fixture(id, 2.0, 1.25);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let second = CanvasDocument::create_ellipse(
            app.world_mut(),
            document,
            CanvasEllipse {
                radii: Vec2::new(10.0, 8.0),
            },
            Transform::from_xyz(45.0, 0.0, 3.0)
                .with_rotation(Quat::from_rotation_z(0.3))
                .with_scale(Vec3::new(-1.2, 0.8, 2.0)),
            CanvasPaint::default(),
        )
        .unwrap();
        let third = CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    CanvasPathCommand::MoveTo(Vec2::splat(-5.0)),
                    CanvasPathCommand::LineTo(Vec2::new(5.0, -5.0)),
                    CanvasPathCommand::LineTo(Vec2::new(0.0, 5.0)),
                    CanvasPathCommand::Close,
                ],
            },
            Transform::from_xyz(-45.0, 0.0, 0.0),
            CanvasPaint::default(),
        )
        .unwrap();
        let shapes = [first, second, third];
        CanvasView::set_selection_in_world(app.world_mut(), a, shapes).unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), b, [second, third]).unwrap();
        CanvasView::set_zoom_in_world(app.world_mut(), a, 0.8).unwrap();
        CanvasView::set_center_in_world(app.world_mut(), a, Vec2::new(5.0, -3.0)).unwrap();
        common::update(&mut app);
        let original = shapes.map(|s| {
            (
                *app.world().get::<Transform>(s).unwrap(),
                app.world().get::<CanvasGeometry>(s).unwrap().clone(),
                *app.world().get::<CanvasPaint>(s).unwrap(),
            )
        });
        let ordered = CanvasDocument::shape_order(app.world_mut(), document).unwrap();
        let selected = selection(&app, a);
        let other = selection(&app, b);
        let assets = app
            .world()
            .resource::<Assets<Mesh>>()
            .ids()
            .collect::<Vec<_>>();
        for target in shapes {
            let start = app
                .world()
                .get::<Transform>(target)
                .unwrap()
                .translation
                .truncate();
            let mut at = coordinates::at_document(&app, a, &base, start);
            common::input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            common::input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            assert_eq!(
                selection(&app, a),
                selected,
                "press must preserve the group until a click or drag completes"
            );
            assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
            let end = coordinates::at_document(&app, a, &base, start + Vec2::new(12.0, 7.0));
            let delta = end.position - at.position;
            common::moved(&mut app, id, &mut at, delta);
            for (shape, (transform, geometry, paint)) in shapes.iter().zip(&original) {
                let actual = app.world().get::<Transform>(*shape).unwrap();
                assert!(
                    actual
                        .translation
                        .truncate()
                        .distance(transform.translation.truncate() + Vec2::new(12.0, 7.0))
                        < 0.001
                );
                assert_eq!(actual.rotation, transform.rotation);
                assert_eq!(actual.scale, transform.scale);
                assert_eq!(actual.translation.z, transform.translation.z);
                assert_eq!(app.world().get::<CanvasGeometry>(*shape), Some(geometry));
                assert_eq!(app.world().get::<CanvasPaint>(*shape), Some(paint));
            }
            let end = coordinates::at_document(&app, a, &base, start);
            let delta = end.position - at.position;
            common::moved(&mut app, id, &mut at, delta);
            common::input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert_eq!(
                selection(&app, a),
                selected,
                "returning to the origin is still a drag"
            );
            assert_eq!(selection(&app, b), other);
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(a));
        }
        assert_eq!(
            CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
            ordered
        );
        assert_eq!(
            app.world()
                .resource::<Assets<Mesh>>()
                .ids()
                .collect::<Vec<_>>(),
            assets
        );
        let at = coordinates::at_document(&app, a, &base, Vec2::ZERO);
        common::input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        common::input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        common::input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(selection(&app, a), [first]);
    }
}

#[test]
fn cancel_or_invalid_members_stop_the_entire_group_and_delete_cleans_all_selected_shapes() {
    use bevy::{
        input_focus::{FocusCause, InputFocus},
        picking::{hover::PointerCaptureMap, pointer::*},
    };
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, pointer_ids, press_key};
    for cause in 0..13 {
        let id = pointer_ids()[1];
        let (mut app, a, b, first, window, base) = common::fixture(id, 1.0, 1.0);
        add_keyboard_dispatch(&mut app);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let second = CanvasDocument::create_ellipse(
            app.world_mut(),
            document,
            CanvasEllipse {
                radii: Vec2::splat(8.0),
            },
            Transform::from_xyz(40.0, 0.0, 0.0),
            CanvasPaint::default(),
        )
        .unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), a, [first, second]).unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), b, [second]).unwrap();
        let mut at = base.clone();
        common::input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        common::input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        common::moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0));
        assert_eq!(
            app.world()
                .get::<Transform>(first)
                .unwrap()
                .translation
                .truncate(),
            Vec2::new(10.0, 5.0)
        );
        match cause {
            0 => common::input(&mut app, id, &at, PointerAction::Cancel),
            1 => app.world_mut().get_mut::<Window>(window).unwrap().focused = false,
            2 => app
                .world_mut()
                .resource_mut::<InputFocus>()
                .set(b, FocusCause::Navigated),
            3 => {
                app.world_mut().entity_mut(a).insert(Visibility::Hidden);
            }
            4 => {
                app.world_mut()
                    .entity_mut(a)
                    .insert(bevy::ui::InteractionDisabled);
            }
            5 => app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None,
            6 => {
                CanvasView::set_selection_in_world(app.world_mut(), a, [first]).unwrap();
            }
            7 => {
                CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
            }
            8 => {
                CanvasDocument::remove_shape(app.world_mut(), document, second).unwrap();
            }
            9 => {
                let other = app.world_mut().spawn(CanvasDocument).id();
                app.world_mut().entity_mut(second).insert(ChildOf(other));
            }
            10 => {
                app.world_mut().despawn(a);
            }
            11 => {
                CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::PathNodeEdit)
                    .unwrap();
            }
            _ => press_key(&mut app, window, KeyCode::Delete),
        }
        common::update(&mut app);
        assert!(
            !app.world().resource::<PointerCaptureMap>().is_captured(&id),
            "cause {cause}"
        );
        if cause == 12 {
            assert!(
                CanvasDocument::shape_order(app.world_mut(), document)
                    .unwrap()
                    .is_empty()
            );
            assert!(selection(&app, a).is_empty());
            assert!(selection(&app, b).is_empty());
            assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
        } else {
            common::moved(&mut app, id, &mut at, Vec2::new(5.0, 5.0));
            assert_eq!(
                app.world()
                    .get::<Transform>(first)
                    .unwrap()
                    .translation
                    .truncate(),
                Vec2::new(10.0, 5.0),
                "cause {cause}"
            );
        }
    }
}
