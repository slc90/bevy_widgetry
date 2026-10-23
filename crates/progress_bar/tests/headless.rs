#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{ecs::error::Severity, prelude::*, ui_widgets::ValueChange};
use bevy_widgetry_core::scene::{
    WidgetrySceneCommandsExt, WidgetrySceneEntityCommandsExt, spawn_scene,
};
use bevy_widgetry_progress_bar::{
    WidgetryProgressBar, WidgetryProgressBarPlugin, WidgetryProgressState,
};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn determinate(value: f32) -> WidgetryProgressState {
    WidgetryProgressState::Determinate { value }
}

fn spawn(world: &mut World, state: WidgetryProgressState) -> Entity {
    spawn_scene(
        world,
        bsn! { Name::new("CallerProgress") @WidgetryProgressBar { @state: {state} } },
    )
    .unwrap()
}

#[derive(Resource, Default)]
struct Events(usize);

fn watch(world: &mut World) {
    world.init_resource::<Events>();
    world.add_observer(|_: On<ValueChange<f32>>, mut events: ResMut<Events>| events.0 += 1);
}

#[test]
fn bsn_initial_states_and_bidirectional_updates_are_silent() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryProgressBarPlugin);
    watch(app.world_mut());
    let default = spawn_scene(app.world_mut(), bsn! { @WidgetryProgressBar }).unwrap();
    assert_eq!(
        WidgetryProgressBar::state(app.world(), default).unwrap(),
        determinate(0.0)
    );
    for initial in [determinate(0.65), WidgetryProgressState::Indeterminate] {
        let entity = spawn(app.world_mut(), initial);
        assert_eq!(
            WidgetryProgressBar::state(app.world(), entity).unwrap(),
            initial
        );
        assert_eq!(
            app.world().get::<Name>(entity).unwrap().as_str(),
            "CallerProgress"
        );
        for state in [
            determinate(1.0),
            determinate(0.4),
            determinate(0.0),
            WidgetryProgressState::Indeterminate,
            determinate(0.75),
        ] {
            assert!(
                WidgetryProgressBar::set_state_in_world(app.world_mut(), entity, state).unwrap()
            );
            assert_eq!(
                WidgetryProgressBar::state(app.world(), entity).unwrap(),
                state
            );
            assert!(
                !WidgetryProgressBar::set_state_in_world(app.world_mut(), entity, state).unwrap()
            );
        }
    }
    assert_eq!(app.world().resource::<Events>().0, 0);
}

#[test]
fn invalid_inputs_and_targets_reject_without_mutation() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    watch(app.world_mut());
    let entity = spawn(app.world_mut(), determinate(0.65));
    let log = LogCapture::default();
    log.run(|| {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
            let error = WidgetryProgressBar::set_state_in_world(
                app.world_mut(),
                entity,
                determinate(value),
            )
            .unwrap_err();
            assert_eq!(error.severity(), Severity::Error);
            assert!(error.to_string().contains("[0, 1]"));
            assert_eq!(
                WidgetryProgressBar::state(app.world(), entity).unwrap(),
                determinate(0.65)
            );
        }
    });
    assert_eq!(log.records().len(), 5);
    let bare = app.world_mut().spawn_empty().id();
    let incomplete = app.world_mut().spawn(WidgetryProgressBar).id();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    for target in [bare, incomplete, stale] {
        assert_eq!(
            WidgetryProgressBar::state(app.world(), target)
                .unwrap_err()
                .severity(),
            Severity::Error
        );
        assert_eq!(
            WidgetryProgressBar::set_state_in_world(app.world_mut(), target, determinate(0.0))
                .unwrap_err()
                .severity(),
            Severity::Error
        );
    }
    assert_eq!(app.world().resource::<Events>().0, 0);
}

#[test]
fn invalid_initialization_cleans_roots_and_deferred_errors_reach_host() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let before = app.world().entities().len();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
        let state = determinate(value);
        let error = spawn_scene(
            app.world_mut(),
            bsn! { @WidgetryProgressBar { @state: {state} } },
        )
        .unwrap_err();
        assert!(error.to_string().contains("[0, 1]"));
        assert_eq!(app.world().entities().len(), before);
    }
    let existing = app.world_mut().spawn(Name::new("Existing")).id();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let reserved = {
        let mut commands = Commands::new(&mut queue, app.world());
        commands.entity(existing).apply_scene_with_error_handler(
            bsn! { @WidgetryProgressBar { @state: {determinate(-1.0)} } },
        );
        commands
            .spawn_scene_with_error_handler(
                bsn! { @WidgetryProgressBar { @state: {determinate(f32::NAN)} } },
            )
            .id()
    };
    let errors = ErrorCapture::default();
    let log = LogCapture::default();
    log.run(|| errors.run(|| queue.apply(app.world_mut())));
    assert_eq!(log.records().len(), 2);
    let failures = errors.take();
    assert_eq!(failures.len(), 2);
    assert!(failures.iter().all(|e| e.severity() == Severity::Error));
    assert!(!app.world().entities().contains(reserved));
    assert!(app.world().entities().contains(existing));
}

#[test]
fn commands_validate_on_execution_including_new_and_stale_targets() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.set_error_handler(ErrorCapture::handler());
    watch(app.world_mut());
    let entity = spawn(app.world_mut(), determinate(0.1));
    let stale = spawn(app.world_mut(), determinate(0.0));
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let reserved = {
        let mut commands = Commands::new(&mut queue, app.world());
        WidgetryProgressBar::set_state(&mut commands, entity, determinate(0.9));
        WidgetryProgressBar::set_state(&mut commands, entity, determinate(f32::NAN));
        WidgetryProgressBar::set_state(&mut commands, stale, WidgetryProgressState::Indeterminate);
        let reserved = commands
            .spawn_scene_with_error_handler(bsn! { @WidgetryProgressBar })
            .id();
        WidgetryProgressBar::set_state(
            &mut commands,
            reserved,
            WidgetryProgressState::Indeterminate,
        );
        reserved
    };
    assert_eq!(
        WidgetryProgressBar::state(app.world(), entity).unwrap(),
        determinate(0.1)
    );
    app.world_mut().despawn(stale);
    let errors = ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    let failures = errors.take();
    assert_eq!(failures.len(), 2);
    assert!(failures.iter().all(|e| e.severity() == Severity::Error));
    assert_eq!(
        WidgetryProgressBar::state(app.world(), entity).unwrap(),
        determinate(0.9)
    );
    assert_eq!(
        WidgetryProgressBar::state(app.world(), reserved).unwrap(),
        WidgetryProgressState::Indeterminate
    );
    assert_eq!(app.world().resource::<Events>().0, 0);
}
