#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    picking::pointer::*, prelude::*, ui::Pressed, ui_widgets::Activate, window::RequestRedraw,
};
use bevy_widgetry_hold_button::{
    WidgetryHoldButtonHeadless, WidgetryHoldButtonHeadlessPlugin, WidgetryHoldButtonProgress,
    WidgetryHoldButtonState,
};
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, picking_app, pointer_ids, queue_pointer, spawn_picking_camera,
};
use std::time::Duration;

#[derive(Resource, Default)]
struct Activations(usize);

fn fixture(id: PointerId) -> (App, Entity, Entity, Location) {
    fixture_with_timing(id, Duration::from_secs(3), Duration::from_millis(1200))
}

fn fixture_with_timing(
    id: PointerId,
    charge: Duration,
    decay: Duration,
) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryHoldButtonHeadlessPlugin)
        .init_resource::<Activations>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ));
    app.add_observer(
        |event: On<Activate>,
         progress: Query<&WidgetryHoldButtonProgress>,
         state: Query<&WidgetryHoldButtonState>,
         mut count: ResMut<Activations>| {
            assert_eq!(progress.get(event.entity).unwrap().value(), 1.0);
            assert!(state.get(event.entity).unwrap().has_succeeded());
            count.0 += 1;
        },
    );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 400).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryHoldButtonHeadless { @charge_duration: charge, @decay_duration: decay }
        Node { width: px(220), height: px(48), position_type: PositionType::Absolute, left: px(40), top: px(40) }
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    app.update();
    app.update();
    assert!(
        !app.world()
            .contains_resource::<Assets<bevy::shader::Shader>>()
    );
    assert!(app.world().get::<Children>(root).is_none());
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::NONE
    );
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(43.0, 43.0),
    };
    (app, root, window, at)
}

#[test]
fn public_duration_props_initialize_each_instance_and_preserve_zero_semantics() {
    for (charge, decay) in [
        (Duration::ZERO, Duration::ZERO),
        (Duration::from_secs(1), Duration::ZERO),
        (Duration::from_secs(1), Duration::from_millis(500)),
    ] {
        let id = PointerId::Mouse;
        let (mut app, root, _, at) = fixture_with_timing(id, charge, decay);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        if charge.is_zero() {
            assert_eq!(value(&app, root), 1.0);
            assert_eq!(app.world().resource::<Activations>().0, 1);
        } else {
            frames(&mut app, 4);
            assert!((value(&app, root) - 0.4).abs() < 0.001);
        }
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        if decay.is_zero() || charge.is_zero() {
            assert_eq!(value(&app, root), 0.0);
        } else {
            assert!((value(&app, root) - 0.5).abs() < 0.001);
            app.update();
            assert!((value(&app, root) - 0.3).abs() < 0.001);
            frames(&mut app, 2);
            assert_eq!(value(&app, root), 0.0);
        }
    }
}
fn input(app: &mut App, id: PointerId, at: &Location, action: PointerAction) {
    queue_pointer(app, id, at.clone(), action);
    app.update();
}
fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}
fn value(app: &App, root: Entity) -> f32 {
    app.world()
        .get::<WidgetryHoldButtonProgress>(root)
        .unwrap()
        .value()
}

#[test]
fn rectangular_corners_charge_without_style_and_reactive_wakes_stop_at_rest() {
    for id in pointer_ids() {
        let (mut app, root, _, at) = fixture(id);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        frames(&mut app, 10);
        assert!((value(&app, root) - 1.0 / 3.0).abs() < 0.001);
        assert!(
            app.world()
                .get::<WidgetryHoldButtonState>(root)
                .unwrap()
                .is_holding()
        );
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        app.update();
        assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
        frames(&mut app, 19);
        assert_eq!(app.world().resource::<Activations>().0, 1);
        assert_eq!(
            app.world()
                .get::<WidgetryHoldButtonState>(root)
                .unwrap()
                .success_count(),
            1
        );
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        app.update();
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
        frames(&mut app, 40);
        assert_eq!(app.world().resource::<Activations>().0, 1);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(value(&app, root), 0.0);
        assert!(
            !app.world()
                .get::<WidgetryHoldButtonState>(root)
                .unwrap()
                .has_succeeded()
        );
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        app.update();
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }
}

#[test]
fn early_release_decays_and_rectangular_exit_requires_fresh_press() {
    let id = PointerId::Mouse;
    let (mut app, root, _, mut at) = fixture(id);
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    frames(&mut app, 9);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let before = value(&app, root);
    assert!(before > 0.1);
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    app.update();
    assert!(value(&app, root) < before);
    assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!(value(&app, root) > 0.0);
    at.position = Vec2::new(300.0, 60.0);
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    assert!(app.world().get::<Pressed>(root).is_none());
    at.position = Vec2::new(250.0, 60.0);
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    frames(&mut app, 40);
    assert_eq!(app.world().resource::<Activations>().0, 0);
    assert_eq!(value(&app, root), 0.0);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    frames(&mut app, 30);
    assert_eq!(app.world().resource::<Activations>().0, 1);
}
