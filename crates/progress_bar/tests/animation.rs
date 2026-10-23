#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_progress_bar::{
    WidgetryProgressBar, WidgetryProgressBarPlugin, WidgetryProgressState,
};
use bevy_widgetry_test_utils::{picking_app, spawn_picking_camera};
use std::time::Duration;

fn value(value: f32) -> WidgetryProgressState {
    WidgetryProgressState::Determinate { value }
}

fn fixture(initial: WidgetryProgressState) -> (App, Entity, Entity) {
    let mut app = picking_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(30),
    ));
    app.add_plugins(WidgetryProgressBarPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(800), 1.0);
    let bar = spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetryProgressBar { @state: initial }
            Node { width: px(300) }
            template(move |_| Ok(UiTargetCamera(camera)))
        },
    )
    .unwrap();
    app.update();
    app.update();
    let track = app.world().get::<Children>(bar).unwrap()[0];
    let clip = app.world().get::<Children>(track).unwrap()[0];
    (app, bar, clip)
}

#[allow(clippy::panic)]
fn visible(app: &App, clip: Entity) -> f32 {
    match app.world().get::<Node>(clip).unwrap().width {
        Val::Percent(value) => value / 100.0,
        other => panic!("unexpected progress width {other:?}"),
    }
}

#[test]
fn real_state_commits_immediately_while_visual_eases_and_then_stops_redrawing() {
    let (mut app, bar, clip) = fixture(value(0.3));
    assert!((visible(&app, clip) - 0.3).abs() < 0.0001);
    for target in [0.6, 1.0, 0.2, 0.0] {
        let start = visible(&app, clip);
        WidgetryProgressBar::set_state_in_world(app.world_mut(), bar, value(target)).unwrap();
        assert_eq!(
            WidgetryProgressBar::state(app.world(), bar).unwrap(),
            value(target)
        );
        app.update();
        assert!((visible(&app, clip) - start).abs() < 0.0001);
        app.update();
        let first = visible(&app, clip);
        assert!((first - start).abs() > (target - start).abs() * 0.3);
        assert!((first - target).abs() > 0.0001);
        for _ in 0..4 {
            app.update();
        }
        assert!((visible(&app, clip) - target).abs() < 0.0001);
    }
    for _ in 0..4 {
        app.update();
    }
    assert!(
        app.world()
            .resource::<bevy::ecs::message::Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
}

#[test]
fn retargeting_uses_the_visible_position_and_noop_does_not_restart() {
    let (mut app, bar, clip) = fixture(value(0.3));
    WidgetryProgressBar::set_state_in_world(app.world_mut(), bar, value(0.6)).unwrap();
    app.update();
    app.update();
    let middle = visible(&app, clip);
    assert!(middle > 0.3 && middle < 0.6);
    WidgetryProgressBar::set_state_in_world(app.world_mut(), bar, value(0.8)).unwrap();
    app.update();
    let expected = middle + (0.8 - middle) * (1.0 - 0.8_f32.powi(3));
    assert!((visible(&app, clip) - expected).abs() < 0.0001);
    app.update();
    let continued = visible(&app, clip);
    assert!(continued > middle && continued < 0.8);
    assert!(!WidgetryProgressBar::set_state_in_world(app.world_mut(), bar, value(0.8)).unwrap());
    for _ in 0..4 {
        app.update();
    }
    assert!((visible(&app, clip) - 0.8).abs() < 0.0001);
}

#[test]
fn first_change_after_idle_does_not_consume_idle_time() {
    let (mut app, bar, clip) = fixture(value(0.65));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs(2),
    ));
    WidgetryProgressBar::set_state_in_world(app.world_mut(), bar, value(1.0)).unwrap();
    app.update();
    assert!((visible(&app, clip) - 0.65).abs() < 0.0001);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(30),
    ));
    app.update();
    assert!(visible(&app, clip) > 0.65 && visible(&app, clip) < 1.0);
}

#[test]
fn resizing_and_recoloring_mid_transition_keep_the_target_and_timing() {
    use bevy_widgetry_progress_bar::WidgetryProgressBarColorOverrides;
    use bevy_widgetry_theme::WidgetryThemeMode;
    let (mut app, bar, clip) = fixture(value(0.2));
    WidgetryProgressBar::set_state_in_world(app.world_mut(), bar, value(0.9)).unwrap();
    app.update();
    app.update();
    let before = visible(&app, clip);
    app.world_mut().get_mut::<Node>(bar).unwrap().width = px(500);
    app.world_mut().get_mut::<Node>(bar).unwrap().height = px(24);
    app.insert_resource(WidgetryThemeMode::PinkDream);
    WidgetryProgressBarColorOverrides::set_in_world(
        app.world_mut(),
        bar,
        WidgetryProgressBarColorOverrides {
            border: Some(Color::WHITE),
            ..default()
        },
    )
    .unwrap();
    app.update();
    let after = visible(&app, clip);
    assert!(after > before && after < 0.9);
    assert_eq!(
        WidgetryProgressBar::state(app.world(), bar).unwrap(),
        value(0.9)
    );
    let computed = app.world().get::<ComputedNode>(bar).unwrap();
    assert_eq!(
        computed.size() * computed.inverse_scale_factor,
        Vec2::new(500.0, 24.0)
    );
    let clipped = app.world().get::<ComputedNode>(clip).unwrap();
    assert!((clipped.size().x * clipped.inverse_scale_factor - 498.0 * after).abs() < 1.0);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(visible(&app, clip), 0.9);
}

#[test]
fn a_new_target_every_frame_still_advances_the_visible_fill() {
    let (mut app, bar, clip) = fixture(value(0.0));
    let mut previous = 0.0;
    for index in 1..=10 {
        let target = index as f32 / 10.0;
        WidgetryProgressBar::set_state_in_world(app.world_mut(), bar, value(target)).unwrap();
        app.update();
        let current = visible(&app, clip);
        if index > 1 {
            assert!(current > previous, "frame {index}: {current} <= {previous}");
        }
        assert!(current < target);
        previous = current;
    }
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(visible(&app, clip), 1.0);
}
