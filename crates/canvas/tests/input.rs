#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input_focus::InputFocus,
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
    ui::{InteractionDisabled, UiGlobalTransform},
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, picking_app, pointer_ids, queue_pointer, spawn_picking_camera,
};

fn update(app: &mut App) {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
}

fn fixture(
    id: PointerId,
    scale: f32,
    ui_scale: f32,
) -> (App, Entity, Entity, Entity, Entity, Location) {
    let mut app = picking_app();
    app.add_plugins((TransformPlugin, CanvasPlugin));
    app.set_error_handler(ErrorCapture::handler());
    app.insert_resource(UiScale(ui_scale));
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(1200, 800)
                    .with_scale_factor_override(scale),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::new(1200, 800), scale);
    app.world_mut()
        .get_mut::<Camera>(camera)
        .unwrap()
        .viewport
        .as_mut()
        .unwrap()
        .physical_position = UVec2::new(24, 32);
    let document = app.world_mut().spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_rectangle(
        app.world_mut(),
        document,
        CanvasRectangle {
            size: Vec2::splat(30.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let a = app.world_mut().spawn_scene(bsn! {
        @CanvasView { @document: {Some(document)} }
        Node { position_type: PositionType::Absolute, left: px(40), top: px(40), width: px(200), height: px(160) }
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    let b = app.world_mut().spawn_scene(bsn! {
        @CanvasView { @document: {Some(document)} }
        Node { position_type: PositionType::Absolute, left: px(300), top: px(40), width: px(180), height: px(150) }
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    update(&mut app);
    update(&mut app);
    let point = app.world().get::<UiGlobalTransform>(a).unwrap().translation;
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: (point + Vec2::new(24.0, 32.0)) / scale,
    };
    (app, a, b, shape, window, location)
}

fn input(app: &mut App, id: PointerId, at: &Location, action: PointerAction) {
    queue_pointer(app, id, at.clone(), action);
    update(app);
}

fn moved(app: &mut App, id: PointerId, at: &mut Location, delta: Vec2) {
    at.position += delta;
    input(app, id, at, PointerAction::Move { delta });
}

#[test]
fn space_primary_pan_locks_until_release_and_only_moves_its_view() {
    for id in pointer_ids() {
        for (scale, ui_scale) in [(1.0, 1.0), (1.5, 1.0), (2.0, 1.5)] {
            let (mut app, a, b, shape, _, mut at) = fixture(id, scale, ui_scale);
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
            assert_eq!(
                app.world()
                    .resource::<PointerCaptureMap>()
                    .get(&id)
                    .unwrap()
                    .0,
                a
            );
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(a));
            moved(&mut app, id, &mut at, Vec2::new(40.0, 20.0) * ui_scale);
            assert!(
                (CanvasView::center(app.world(), a).unwrap() - Vec2::new(-40.0, -20.0)).length()
                    < 0.001
            );
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::Space);
            moved(&mut app, id, &mut at, Vec2::new(160.0, 60.0) * ui_scale);
            assert!(
                (CanvasView::center(app.world(), a).unwrap() - Vec2::new(-200.0, -80.0)).length()
                    < 0.001
            );
            assert_eq!(CanvasView::center(app.world(), b).unwrap(), Vec2::ZERO);
            assert_eq!(
                *app.world().get::<Transform>(shape).unwrap(),
                Transform::default()
            );
            let camera = app.world().get::<ViewportNode>(a).unwrap().camera.unwrap();
            let position = app
                .world()
                .get::<GlobalTransform>(camera)
                .unwrap()
                .translation();
            assert!((position.truncate() - Vec2::new(-200.0, 80.0)).length() < 0.001);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            moved(&mut app, id, &mut at, Vec2::splat(15.0));
            assert!(
                (CanvasView::center(app.world(), a).unwrap() - Vec2::new(-200.0, -80.0)).length()
                    < 0.001
            );
        }
    }
}

#[test]
fn pan_rejects_unmodified_primary_and_middle_and_cancels_when_unavailable() {
    for cause in 0..9 {
        let id = pointer_ids()[1];
        let (mut app, a, _, shape, window, mut at) = fixture(id, 1.0, 1.0);
        moved(&mut app, id, &mut at, Vec2::ZERO);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Middle),
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Middle),
        );
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
        match cause {
            0 => input(&mut app, id, &at, PointerAction::Cancel),
            1 => {
                app.world_mut().entity_mut(a).insert(Visibility::Hidden);
            }
            2 => {
                app.world_mut().entity_mut(a).insert(InteractionDisabled);
            }
            3 => {
                app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
            }
            4 => {
                app.world_mut().despawn(a);
            }
            5 => {
                app.world_mut().resource_mut::<InputFocus>().clear();
            }
            6 => {
                let document = app.world().get::<ChildOf>(shape).unwrap().parent();
                app.world_mut().despawn(document);
            }
            7 => {
                app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None;
            }
            _ => {
                app.world_mut().get_mut::<Node>(a).unwrap().width = px(0);
            }
        }
        update(&mut app);
        assert!(
            !app.world().resource::<PointerCaptureMap>().is_captured(&id),
            "cause {cause}"
        );
        if cause != 4 {
            assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
        }
    }
}

#[test]
fn pan_uses_transformed_local_axes_and_ignores_other_pointers() {
    let id = pointer_ids()[1];
    let (mut app, a, _, _, _, mut at) = fixture(id, 1.5, 1.0);
    app.world_mut().entity_mut(a).insert(bevy::ui::UiTransform {
        rotation: Rot2::degrees(30.0),
        scale: Vec2::new(1.2, 0.8),
        ..default()
    });
    update(&mut app);
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
    let local = Vec2::new(30.0, -20.0);
    let delta = app
        .world()
        .get::<UiGlobalTransform>(a)
        .unwrap()
        .transform_vector2(local * 1.5)
        / 1.5;
    moved(&mut app, id, &mut at, delta);
    assert!((CanvasView::center(app.world(), a).unwrap() + local).length() < 0.001);
    input(
        &mut app,
        PointerId::Mouse,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        PointerId::Mouse,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        app.world()
            .resource::<PointerCaptureMap>()
            .get(&id)
            .unwrap()
            .0,
        a
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    moved(&mut app, id, &mut at, Vec2::splat(10.0));
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    assert!((CanvasView::center(app.world(), a).unwrap() + local).length() < 0.001);
}

#[test]
fn releasing_one_view_allows_the_same_pointer_to_pan_another_view() {
    let id = pointer_ids()[1];
    let (mut app, a, b, _, _, mut at) = fixture(id, 1.0, 1.0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    for view in [a, b] {
        let point = app
            .world()
            .get::<UiGlobalTransform>(view)
            .unwrap()
            .translation
            + Vec2::new(24.0, 32.0);
        let delta = point - at.position;
        moved(&mut app, id, &mut at, delta);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(view));
        assert_eq!(
            app.world()
                .resource::<PointerCaptureMap>()
                .get(&id)
                .unwrap()
                .0,
            view
        );
        moved(&mut app, id, &mut at, Vec2::new(20.0, -10.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(
            CanvasView::center(app.world(), view).unwrap(),
            Vec2::new(-20.0, 10.0)
        );
    }
    assert_eq!(
        CanvasView::center(app.world(), a).unwrap(),
        Vec2::new(-20.0, 10.0)
    );
}

#[test]
fn non_primary_releases_preserve_pan_and_final_move_precedes_primary_release() {
    for id in pointer_ids() {
        let (mut app, a, _, _, _, mut at) = fixture(id, 1.0, 1.0);
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
        moved(&mut app, id, &mut at, Vec2::new(200.0, 200.0));
        for button in [PointerButton::Secondary, PointerButton::Middle] {
            input(&mut app, id, &at, PointerAction::Press(button));
            input(&mut app, id, &at, PointerAction::Release(button));
            assert_eq!(
                app.world()
                    .resource::<PointerCaptureMap>()
                    .get(&id)
                    .unwrap()
                    .0,
                a
            );
            moved(&mut app, id, &mut at, Vec2::splat(10.0));
        }
        let delta = Vec2::new(30.0, -20.0);
        at.position += delta;
        queue_pointer(&mut app, id, at.clone(), PointerAction::Move { delta });
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Release(PointerButton::Primary),
        );
        update(&mut app);
        assert_eq!(
            CanvasView::center(app.world(), a).unwrap(),
            Vec2::new(-250.0, -200.0)
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    }
}

#[test]
fn final_move_is_applied_before_same_frame_release_or_cancel() {
    for terminal in [
        PointerAction::Release(PointerButton::Primary),
        PointerAction::Cancel,
    ] {
        for id in pointer_ids() {
            let (mut app, a, _, _, _, mut at) = fixture(id, 1.0, 1.0);
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
            let delta = Vec2::new(300.0, 250.0);
            at.position += delta;
            queue_pointer(&mut app, id, at.clone(), PointerAction::Move { delta });
            queue_pointer(&mut app, id, at.clone(), terminal);
            update(&mut app);
            assert_eq!(CanvasView::center(app.world(), a).unwrap(), -delta);
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        }
    }
}

#[test]
fn same_frame_release_and_repress_routes_the_new_gesture_to_its_actual_view() {
    for id in pointer_ids() {
        let (mut app, a, b, _, _, mut at) = fixture(id, 1.0, 1.0);
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
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Release(PointerButton::Primary),
        );
        let position =
            app.world().get::<UiGlobalTransform>(b).unwrap().translation + Vec2::new(24.0, 32.0);
        let delta = position - at.position;
        at.position = position;
        queue_pointer(&mut app, id, at.clone(), PointerAction::Move { delta });
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Press(PointerButton::Primary),
        );
        let delta = Vec2::splat(5.0);
        at.position += delta;
        queue_pointer(&mut app, id, at.clone(), PointerAction::Move { delta });
        update(&mut app);
        assert_eq!(
            app.world()
                .resource::<PointerCaptureMap>()
                .get(&id)
                .unwrap()
                .0,
            b
        );
        moved(&mut app, id, &mut at, Vec2::new(10.0, 20.0));
        assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
        assert_eq!(
            CanvasView::center(app.world(), b).unwrap(),
            Vec2::new(-15.0, -25.0)
        );
    }
}

#[test]
fn secondary_press_before_same_frame_primary_release_preserves_the_final_move() {
    for id in pointer_ids() {
        let (mut app, a, b, _, _, mut at) = fixture(id, 1.0, 1.0);
        #[derive(Resource, Default)]
        struct Presses(Vec<(Entity, PointerButton)>);
        app.init_resource::<Presses>().add_observer(
            |event: On<bevy::picking::events::PointerPress>, mut presses: ResMut<Presses>| {
                if event.entity == event.original_event_target()
                    && event.button != PointerButton::Primary
                {
                    presses.0.push((event.entity, event.button));
                }
            },
        );
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
        let position =
            app.world().get::<UiGlobalTransform>(b).unwrap().translation + Vec2::new(24.0, 32.0);
        let delta = position - at.position;
        moved(&mut app, id, &mut at, delta);
        let before = CanvasView::center(app.world(), a).unwrap();
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Press(PointerButton::Secondary),
        );
        let delta = Vec2::new(10.0, 20.0);
        at.position += delta;
        queue_pointer(&mut app, id, at.clone(), PointerAction::Move { delta });
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Release(PointerButton::Primary),
        );
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Press(PointerButton::Middle),
        );
        update(&mut app);
        assert_eq!(CanvasView::center(app.world(), a).unwrap(), before - delta);
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        assert_eq!(
            app.world().resource::<Presses>().0,
            [(b, PointerButton::Middle)]
        );
    }
}

#[test]
fn first_frame_presses_are_not_preconsumed_before_observer_cancellation() {
    for id in pointer_ids() {
        for take_capture in [false, true] {
            let (mut app, a, b, _, _, mut at) = fixture(id, 1.0, 1.0);
            moved(&mut app, id, &mut at, Vec2::ZERO);
            #[derive(Resource, Default)]
            struct Presses(usize);
            app.init_resource::<Presses>().add_observer(
                |event: On<bevy::picking::events::PointerPress>, mut presses: ResMut<Presses>| {
                    if event.entity == event.original_event_target()
                        && event.button == PointerButton::Secondary
                    {
                        presses.0 += 1;
                    }
                },
            );
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Space);
            let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
            app.add_observer(
                move |event: On<CanvasCenterChanged>,
                      mut world: bevy::ecs::world::DeferredWorld| {
                    if event.entity == a {
                        if take_capture {
                            world
                                .resource_mut::<PointerCaptureMap>()
                                .capture(id, b, hit.clone());
                        } else {
                            world
                                .resource_mut::<InputFocus>()
                                .set(b, bevy::input_focus::FocusCause::Navigated);
                        }
                    }
                },
            );
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Press(PointerButton::Primary),
            );
            let delta = Vec2::new(10.0, 0.0);
            at.position += delta;
            queue_pointer(&mut app, id, at.clone(), PointerAction::Move { delta });
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Press(PointerButton::Secondary),
            );
            update(&mut app);
            assert_eq!(app.world().resource::<Presses>().0, 1);
            if take_capture {
                assert_eq!(
                    app.world()
                        .resource::<PointerCaptureMap>()
                        .get(&id)
                        .unwrap()
                        .0,
                    b
                );
            } else {
                assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            }
        }
    }
}

#[test]
fn first_frame_capture_transfer_preserves_hover_exit_and_entry_events() {
    for id in pointer_ids() {
        let (mut app, a, b, _, _, mut at) = fixture(id, 1.0, 1.0);
        moved(&mut app, id, &mut at, Vec2::ZERO);
        #[derive(Resource, Default)]
        struct Transitions(Vec<(Entity, bool)>);
        app.init_resource::<Transitions>()
            .add_observer(
                move |event: On<bevy::picking::events::PointerOut>,
                      mut transitions: ResMut<Transitions>| {
                    if event.pointer.id == id && event.entity == event.original_event_target() {
                        transitions.0.push((event.entity, false));
                    }
                },
            )
            .add_observer(
                move |event: On<bevy::picking::events::PointerOver>,
                      mut transitions: ResMut<Transitions>| {
                    if event.pointer.id == id && event.entity == event.original_event_target() {
                        transitions.0.push((event.entity, true));
                    }
                },
            );
        app.add_observer(
            move |event: On<CanvasCenterChanged>, mut world: bevy::ecs::world::DeferredWorld| {
                if event.entity == a {
                    let hit = world
                        .resource::<PointerCaptureMap>()
                        .get(&id)
                        .unwrap()
                        .1
                        .clone();
                    world
                        .resource_mut::<PointerCaptureMap>()
                        .capture(id, b, hit);
                }
            },
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Press(PointerButton::Primary),
        );
        let delta = Vec2::splat(5.0);
        at.position += delta;
        queue_pointer(&mut app, id, at, PointerAction::Move { delta });
        update(&mut app);
        update(&mut app);
        assert_eq!(
            app.world().resource::<Transitions>().0,
            [(a, false), (b, true)]
        );
    }
}

#[test]
fn center_observers_can_cancel_or_transfer_capture_without_consuming_later_input() {
    for take_capture in [true, false] {
        for release in [false, true] {
            let id = pointer_ids()[1];
            let (mut app, a, b, _, _, mut at) = fixture(id, 1.0, 1.0);
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
            let position = app.world().get::<UiGlobalTransform>(b).unwrap().translation
                + Vec2::new(24.0, 32.0);
            let delta = position - at.position;
            moved(&mut app, id, &mut at, delta);
            let before = CanvasView::center(app.world(), a).unwrap();
            let hit = app
                .world()
                .resource::<PointerCaptureMap>()
                .get(&id)
                .unwrap()
                .1
                .clone();
            app.add_observer(
                move |event: On<CanvasCenterChanged>,
                      mut world: bevy::ecs::world::DeferredWorld| {
                    if event.entity == a {
                        if take_capture {
                            world
                                .resource_mut::<PointerCaptureMap>()
                                .capture(id, b, hit.clone());
                        } else {
                            world
                                .resource_mut::<InputFocus>()
                                .set(b, bevy::input_focus::FocusCause::Navigated);
                        }
                    }
                },
            );
            #[derive(Resource, Default)]
            struct Presses(Vec<Entity>);
            app.init_resource::<Presses>().add_observer(
                |event: On<bevy::picking::events::PointerPress>, mut presses: ResMut<Presses>| {
                    if event.entity == event.original_event_target()
                        && event.button == PointerButton::Secondary
                    {
                        presses.0.push(event.entity);
                    }
                },
            );
            let first = Vec2::new(10.0, 20.0);
            for delta in [first, Vec2::splat(5.0)] {
                at.position += delta;
                queue_pointer(&mut app, id, at.clone(), PointerAction::Move { delta });
            }
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Press(PointerButton::Secondary),
            );
            if release {
                queue_pointer(
                    &mut app,
                    id,
                    at.clone(),
                    PointerAction::Release(PointerButton::Primary),
                );
            }
            update(&mut app);
            assert_eq!(CanvasView::center(app.world(), a).unwrap(), before - first);
            assert_eq!(app.world().resource::<Presses>().0, [b]);
            if take_capture {
                assert_eq!(
                    app.world()
                        .resource::<PointerCaptureMap>()
                        .get(&id)
                        .unwrap()
                        .0,
                    b
                );
            }
        }
    }
}

#[test]
fn first_frame_press_release_without_drag_does_not_leave_pan_captured() {
    for id in pointer_ids() {
        let (mut app, a, _, _, _, at) = fixture(id, 1.0, 1.0);
        let outside = Location {
            position: Vec2::new(1000.0, 700.0),
            ..at.clone()
        };
        input(
            &mut app,
            id,
            &outside,
            PointerAction::Move { delta: Vec2::ZERO },
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        for action in [
            PointerAction::Move {
                delta: at.position - outside.position,
            },
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            queue_pointer(&mut app, id, at.clone(), action);
        }
        update(&mut app);
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        let mut at = at;
        moved(&mut app, id, &mut at, Vec2::new(20.0, 10.0));
        assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
    }
}

#[test]
fn first_frame_repress_starts_at_its_own_position_without_unpressed_motion() {
    for id in pointer_ids() {
        let (mut app, a, _, _, _, at) = fixture(id, 1.0, 1.0);
        let outside = Location {
            position: Vec2::new(1000.0, 700.0),
            ..at.clone()
        };
        input(
            &mut app,
            id,
            &outside,
            PointerAction::Move { delta: Vec2::ZERO },
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        let second = Location {
            position: at.position + Vec2::new(30.0, 20.0),
            ..at.clone()
        };
        let mut last = Location {
            position: second.position + Vec2::new(10.0, 5.0),
            ..at.clone()
        };
        for (point, action) in [
            (
                &at,
                PointerAction::Move {
                    delta: at.position - outside.position,
                },
            ),
            (&at, PointerAction::Press(PointerButton::Primary)),
            (&at, PointerAction::Release(PointerButton::Primary)),
            (
                &second,
                PointerAction::Move {
                    delta: second.position - at.position,
                },
            ),
            (&second, PointerAction::Press(PointerButton::Primary)),
            (
                &last,
                PointerAction::Move {
                    delta: last.position - second.position,
                },
            ),
        ] {
            queue_pointer(&mut app, id, point.clone(), action);
        }
        update(&mut app);
        assert_eq!(
            CanvasView::center(app.world(), a).unwrap(),
            Vec2::new(-10.0, -5.0)
        );
        moved(&mut app, id, &mut last, Vec2::new(4.0, 3.0));
        assert_eq!(
            CanvasView::center(app.world(), a).unwrap(),
            Vec2::new(-14.0, -8.0)
        );
        input(
            &mut app,
            id,
            &last,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    }
}

#[test]
fn center_api_commits_before_notice_and_rejects_invalid_values() {
    let (mut app, a, _, _, _, _) = fixture(pointer_ids()[1], 1.0, 1.0);
    #[derive(Resource, Default)]
    struct Notices(Vec<Vec2>);
    app.init_resource::<Notices>().add_observer(
        |event: On<CanvasCenterChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                CanvasView::center(&world, event.entity).unwrap(),
                event.center
            );
            world.resource_mut::<Notices>().0.push(event.center);
        },
    );
    assert!(!CanvasView::set_center_in_world(app.world_mut(), a, Vec2::ZERO).unwrap());
    assert!(CanvasView::set_center_in_world(app.world_mut(), a, Vec2::new(-40.0, 25.0)).unwrap());
    let logs = LogCapture::default();
    logs.run(|| {
        let error =
            CanvasView::set_center_in_world(app.world_mut(), a, Vec2::splat(f32::NAN)).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    });
    assert!(!logs.records().is_empty());
    assert_eq!(
        app.world().resource::<Notices>().0,
        [Vec2::new(-40.0, 25.0)]
    );
    let mut commands = app.world_mut().commands();
    CanvasView::set_center(&mut commands, a, Vec2::new(20.0, -30.0));
    app.world_mut().flush();
    assert_eq!(
        CanvasView::center(app.world(), a).unwrap(),
        Vec2::new(20.0, -30.0)
    );
    assert_eq!(app.world().resource::<Notices>().0.len(), 2);
}

fn scroll(app: &mut App, id: PointerId, at: &Location, y: f32) {
    input(
        app,
        id,
        at,
        PointerAction::Scroll {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y,
            phase: bevy::input::touch::TouchPhase::Moved,
        },
    );
}

#[test]
fn zoom_api_is_center_anchored_validated_and_notified_after_commit() {
    let (mut app, a, b, _, _, _) = fixture(pointer_ids()[1], 1.0, 1.0);
    CanvasView::set_center_in_world(app.world_mut(), a, Vec2::new(-40.0, 25.0)).unwrap();
    #[derive(Resource, Default)]
    struct ZoomNotices(Vec<f32>);
    app.init_resource::<ZoomNotices>().add_observer(
        |event: On<CanvasZoomChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(CanvasView::zoom(&world, event.entity).unwrap(), event.zoom);
            assert_eq!(
                CanvasView::center(&world, event.entity).unwrap(),
                Vec2::new(-40.0, 25.0)
            );
            world.resource_mut::<ZoomNotices>().0.push(event.zoom);
        },
    );
    assert_eq!(CanvasView::zoom(app.world(), a).unwrap(), 1.0);
    assert!(!CanvasView::set_zoom_in_world(app.world_mut(), a, 1.0).unwrap());
    assert!(CanvasView::set_zoom_in_world(app.world_mut(), a, 2.0).unwrap());
    let logs = LogCapture::default();
    logs.run(|| {
        for zoom in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            0.0,
            -1.0,
            0.099,
            10.001,
        ] {
            let error = CanvasView::set_zoom_in_world(app.world_mut(), a, zoom).unwrap_err();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            assert_eq!(CanvasView::zoom(app.world(), a).unwrap(), 2.0);
        }
    });
    assert_eq!(logs.records().len(), 7);
    assert_eq!(app.world().resource::<ZoomNotices>().0, [2.0]);
    let mut commands = app.world_mut().commands();
    CanvasView::set_zoom(&mut commands, a, 0.1);
    app.world_mut().flush();
    assert_eq!(CanvasView::zoom(app.world(), a).unwrap(), 0.1);
    assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 1.0);
    let dead = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(dead);
    let mut commands = app.world_mut().commands();
    CanvasView::set_zoom(&mut commands, dead, 3.0);
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
}

#[test]
fn wheel_keeps_cursor_document_point_stable_and_pan_uses_zoom() {
    for id in pointer_ids() {
        for (scale, ui_scale) in [(1.0, 1.0), (1.5, 1.0), (2.0, 1.5)] {
            let (mut app, a, b, shape, _, mut at) = fixture(id, scale, ui_scale);
            let transform = *app.world().get::<Transform>(shape).unwrap();
            let rectangle = *app.world().get::<CanvasRectangle>(shape).unwrap();
            let paint = *app.world().get::<CanvasPaint>(shape).unwrap();
            let local = Vec2::new(35.0, -20.0);
            at.position += local * ui_scale;
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            for _ in 0..8 {
                scroll(&mut app, id, &at, 1.0);
            }
            let zoom = CanvasView::zoom(app.world(), a).unwrap();
            assert!(zoom > 1.0);
            assert!(
                (CanvasView::center(app.world(), a).unwrap() + local / zoom - local).length()
                    < 0.001
            );
            assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 1.0);
            for _ in 0..8 {
                scroll(&mut app, id, &at, -1.0);
            }
            assert!((CanvasView::zoom(app.world(), a).unwrap() - 1.0).abs() < 0.0001);
            assert!(CanvasView::center(app.world(), a).unwrap().length() < 0.001);
            CanvasView::set_zoom_in_world(app.world_mut(), a, 2.0).unwrap();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Space);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            let before = CanvasView::center(app.world(), a).unwrap();
            moved(&mut app, id, &mut at, Vec2::new(20.0, 10.0) * ui_scale);
            assert!(
                (CanvasView::center(app.world(), a).unwrap() - before + Vec2::new(10.0, 5.0))
                    .length()
                    < 0.001
            );
            let before = CanvasView::center(app.world(), a).unwrap();
            scroll(&mut app, id, &at, 1.0);
            let after_wheel = CanvasView::center(app.world(), a).unwrap();
            assert_ne!(after_wheel, before);
            let zoom = CanvasView::zoom(app.world(), a).unwrap();
            moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0) * ui_scale);
            assert!(
                (CanvasView::center(app.world(), a).unwrap() - after_wheel
                    + Vec2::new(10.0, 5.0) / zoom)
                    .length()
                    < 0.001
            );
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert_eq!(*app.world().get::<Transform>(shape).unwrap(), transform);
            assert_eq!(
                *app.world().get::<CanvasRectangle>(shape).unwrap(),
                rectangle
            );
            assert_eq!(*app.world().get::<CanvasPaint>(shape).unwrap(), paint);
        }
    }
}

#[test]
fn wheel_coordinate_conversion_and_atomic_notices_survive_runtime_scale_changes() {
    let id = pointer_ids()[1];
    let (mut app, a, _, _, window, mut at) = fixture(id, 1.0, 1.0);
    let camera = app
        .world()
        .get::<bevy::ui::ComputedUiTargetCamera>(a)
        .unwrap()
        .get()
        .unwrap();
    app.world_mut().entity_mut(a).insert(bevy::ui::UiTransform {
        rotation: Rot2::degrees(25.0),
        scale: Vec2::new(1.2, 0.8),
        ..default()
    });
    #[derive(Resource, Default)]
    struct Anchor {
        point: Vec2,
        local: Vec2,
        notices: usize,
    }
    app.init_resource::<Anchor>()
        .add_observer(
            |event: On<CanvasZoomChanged>, mut world: bevy::ecs::world::DeferredWorld| {
                let anchor = world.resource::<Anchor>();
                assert_eq!(CanvasView::zoom(&world, event.entity).unwrap(), event.zoom);
                assert!(
                    (CanvasView::center(&world, event.entity).unwrap() + anchor.local / event.zoom
                        - anchor.point)
                        .length()
                        < 0.001
                );
                world.resource_mut::<Anchor>().notices += 1;
            },
        )
        .add_observer(
            |event: On<CanvasCenterChanged>, mut world: bevy::ecs::world::DeferredWorld| {
                let anchor = world.resource::<Anchor>();
                assert_eq!(
                    CanvasView::center(&world, event.entity).unwrap(),
                    event.center
                );
                assert!(
                    (event.center + anchor.local / CanvasView::zoom(&world, event.entity).unwrap()
                        - anchor.point)
                        .length()
                        < 0.001
                );
                world.resource_mut::<Anchor>().notices += 1;
            },
        );
    let local = Vec2::new(30.0, -15.0);
    for scale in [1.0, 1.5, 2.0, 1.0] {
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set_scale_factor_override(Some(scale));
        app.world_mut()
            .get_mut::<Camera>(camera)
            .unwrap()
            .computed
            .target_info
            .as_mut()
            .unwrap()
            .scale_factor = scale;
        update(&mut app);
        let physical = app
            .world()
            .get::<UiGlobalTransform>(a)
            .unwrap()
            .transform_point2(local * scale);
        at.position = (physical + Vec2::new(24.0, 32.0)) / scale;
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        let old = CanvasView::zoom(app.world(), a).unwrap();
        let point = CanvasView::center(app.world(), a).unwrap() + local / old;
        *app.world_mut().resource_mut::<Anchor>() = Anchor {
            point,
            local,
            notices: 0,
        };
        input(
            &mut app,
            id,
            &at,
            PointerAction::Scroll {
                unit: bevy::input::mouse::MouseScrollUnit::Pixel,
                x: 0.0,
                y: 100.0,
                phase: bevy::input::touch::TouchPhase::Moved,
            },
        );
        assert!((CanvasView::zoom(app.world(), a).unwrap() / old - 0.1_f32.exp()).abs() < 0.0001);
        assert_eq!(app.world().resource::<Anchor>().notices, 2);
    }
}

#[test]
fn captured_pan_interleaves_moves_and_wheel_in_the_original_frame_order() {
    for id in pointer_ids() {
        let (mut app, a, _, _, _, mut at) = fixture(id, 1.0, 1.0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        at.position += Vec2::new(20.0, 10.0);
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Move {
                delta: Vec2::new(20.0, 10.0),
            },
        );
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Scroll {
                unit: bevy::input::mouse::MouseScrollUnit::Line,
                x: 0.0,
                y: 2.0,
                phase: bevy::input::touch::TouchPhase::Moved,
            },
        );
        at.position += Vec2::new(10.0, 5.0);
        queue_pointer(
            &mut app,
            id,
            at.clone(),
            PointerAction::Move {
                delta: Vec2::new(10.0, 5.0),
            },
        );
        update(&mut app);
        let zoom = CanvasView::zoom(app.world(), a).unwrap();
        let expected = -Vec2::new(20.0, 10.0) + Vec2::new(20.0, 10.0) * (1.0 - zoom.recip())
            - Vec2::new(10.0, 5.0) / zoom;
        assert!((CanvasView::center(app.world(), a).unwrap() - expected).length() < 0.001);
    }
}

#[test]
fn wheel_targeting_a_nested_widget_reaches_it_without_zooming_its_canvas_parent() {
    for id in pointer_ids() {
        let (mut app, a, _, _, _, at) = fixture(id, 1.0, 1.0);
        #[derive(Resource, Default)]
        struct Scrolled(usize);
        app.init_resource::<Scrolled>();
        let nested = app
            .world_mut()
            .spawn_scene(bsn! {
                bevy::ui_widgets::Button
                Node { width:percent(100),height:percent(100) }
                template(move |_|Ok(ChildOf(a)))
            })
            .unwrap()
            .id();
        app.world_mut().entity_mut(nested).observe(
            |_: On<bevy::picking::events::PointerScroll>, mut seen: ResMut<Scrolled>| {
                seen.0 += 1;
            },
        );
        update(&mut app);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        scroll(&mut app, id, &at, 1.0);
        assert_eq!(app.world().resource::<Scrolled>().0, 1);
        assert_eq!(CanvasView::zoom(app.world(), a).unwrap(), 1.0);
        assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
    }
}

#[test]
fn wheel_bounds_and_guards_preserve_center_and_other_widgets_scroll() {
    for cause in 0..8 {
        let id = pointer_ids()[1];
        let (mut app, a, b, _, window, mut at) = fixture(id, 1.0, 1.0);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        CanvasView::set_zoom_in_world(app.world_mut(), a, 10.0).unwrap();
        scroll(&mut app, id, &at, 10.0);
        assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
        assert_eq!(CanvasView::zoom(app.world(), a).unwrap(), 10.0);
        CanvasView::set_zoom_in_world(app.world_mut(), a, 0.1).unwrap();
        scroll(&mut app, id, &at, -10.0);
        assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
        assert_eq!(CanvasView::zoom(app.world(), a).unwrap(), 0.1);
        CanvasView::set_zoom_in_world(app.world_mut(), a, 1.0).unwrap();
        match cause {
            0 => {
                app.world_mut().entity_mut(a).insert(Visibility::Hidden);
            }
            1 => {
                app.world_mut().entity_mut(a).insert(InteractionDisabled);
            }
            2 => {
                app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
            }
            3 => {
                CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
            }
            4 => {
                app.world_mut().get_mut::<Node>(a).unwrap().width = px(0);
            }
            5 => {
                app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None;
            }
            6 => {
                at.position = Vec2::new(1000.0, 700.0);
            }
            _ => {
                #[derive(Resource, Default)]
                struct Scrolled(usize);
                app.init_resource::<Scrolled>();
                app.world_mut().entity_mut(b).remove::<CanvasView>();
                CanvasView::set_document_in_world(app.world_mut(), b, None).unwrap();
                app.world_mut().entity_mut(b).observe(
                    |_: On<bevy::picking::events::PointerScroll>, mut seen: ResMut<Scrolled>| {
                        seen.0 += 1;
                    },
                );
                at.position = app.world().get::<UiGlobalTransform>(b).unwrap().translation
                    + Vec2::new(24.0, 32.0);
                input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
                scroll(&mut app, id, &at, 1.0);
                assert_eq!(app.world().resource::<Scrolled>().0, 1);
            }
        }
        update(&mut app);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        scroll(&mut app, id, &at, 1.0);
        assert_eq!(
            CanvasView::zoom(app.world(), a).unwrap(),
            1.0,
            "cause {cause}"
        );
        assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
    }
}
