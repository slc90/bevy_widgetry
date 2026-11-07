#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{prelude::*, window::RequestRedraw};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_progress_bar::*;

#[test]
fn independent_progress_has_real_state_without_visual_children_or_animation_wakes() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryProgressBarHeadlessPlugin)
        .add_message::<RequestRedraw>();
    for initial in [
        WidgetryProgressState::default(),
        WidgetryProgressState::Determinate { value: 0.65 },
        WidgetryProgressState::Indeterminate,
    ] {
        let root = spawn_scene(
            app.world_mut(),
            bsn! {@WidgetryProgressBarHeadless {@state:{initial}}},
        )
        .unwrap();
        assert_eq!(
            WidgetryProgressBarHeadless::state(app.world(), root).unwrap(),
            initial
        );
        for state in [
            WidgetryProgressState::Determinate { value: 1.0 },
            WidgetryProgressState::Determinate { value: 0.2 },
            WidgetryProgressState::default(),
            WidgetryProgressState::Indeterminate,
        ] {
            WidgetryProgressBarHeadless::set_state(&mut app.world_mut().commands(), root, state);
            app.world_mut().flush();
            assert_eq!(
                WidgetryProgressBarHeadless::state(app.world(), root).unwrap(),
                state
            );
            assert!(
                !WidgetryProgressBarHeadless::set_state_in_world(app.world_mut(), root, state)
                    .unwrap()
            );
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1] {
            assert!(
                WidgetryProgressBarHeadless::set_state_in_world(
                    app.world_mut(),
                    root,
                    WidgetryProgressState::Determinate { value }
                )
                .is_err()
            );
            assert_eq!(
                WidgetryProgressBarHeadless::state(app.world(), root).unwrap(),
                WidgetryProgressState::Indeterminate
            );
        }
        app.update();
        app.update();
        assert!(app.world().get::<Children>(root).is_none());
        assert!(app.world().get::<WidgetryProgressBar>(root).is_none());
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }
}

#[test]
fn independent_progress_rejects_invalid_initialization_and_stale_queued_targets() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryProgressBarHeadlessPlugin);
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    let before = app.world().entities().len();
    assert!(spawn_scene(app.world_mut(),bsn! {@WidgetryProgressBarHeadless {@state:WidgetryProgressState::Determinate {value:2.0}}}).is_err());
    assert_eq!(app.world().entities().len(), before);
    let root = spawn_scene(app.world_mut(), bsn! {@WidgetryProgressBarHeadless}).unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryProgressBarHeadless::set_state(
        &mut Commands::new(&mut queue, app.world()),
        root,
        WidgetryProgressState::Indeterminate,
    );
    app.world_mut().despawn(root);
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    let records = errors.take();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].severity(), bevy::ecs::error::Severity::Error);
}
