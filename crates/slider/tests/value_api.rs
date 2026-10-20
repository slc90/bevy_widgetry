#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{ecs::error::Severity, prelude::*, ui::InteractionDisabled, ui_widgets::ValueChange};
use bevy_widgetry_core::scene::{
    WidgetrySceneCommandsExt, WidgetrySceneEntityCommandsExt, spawn_scene,
};
use bevy_widgetry_slider::{
    WidgetrySlider, WidgetrySliderOrientation, WidgetrySliderPlugin, WidgetrySliderProps,
};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};

fn slider(world: &mut World, props: WidgetrySliderProps) -> Entity {
    spawn_scene(world, bsn! { @WidgetrySlider { @min: {props.min}, @max: {props.max}, @step: {props.step}, @value: {props.value}, @orientation: {props.orientation} } }).unwrap()
}

#[derive(Resource, Default)]
struct Seen(Vec<(Entity, f32, f32, bool)>);

fn observe(world: &mut World) {
    world.init_resource::<Seen>();
    world.add_observer(
        |event: On<ValueChange<f32>>, mut world: bevy::ecs::world::DeferredWorld| -> Result {
            let value = WidgetrySlider::value(&world, event.source)?;
            world
                .resource_mut::<Seen>()
                .0
                .push((event.source, event.value, value, event.is_final));
            Ok(())
        },
    );
}

#[test]
fn bsn_initialization_and_queries_preserve_configuration() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let world = app.world_mut();
    observe(world);
    let default = slider(world, default());
    assert_eq!(WidgetrySlider::value(world, default).unwrap(), 0.0);
    let explicit = slider(
        world,
        WidgetrySliderProps {
            min: -10.0,
            max: 10.0,
            value: Some(-3.5),
            orientation: WidgetrySliderOrientation::Vertical,
            ..Default::default()
        },
    );
    assert_eq!(WidgetrySlider::value(world, explicit).unwrap(), -3.5);
    assert_eq!(
        WidgetrySlider::orientation(world, explicit).unwrap(),
        WidgetrySliderOrientation::Vertical
    );
    assert!(world.resource::<Seen>().0.is_empty());
}

#[test]
fn changes_commit_before_notification_and_disabled_does_not_block_api() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let world = app.world_mut();
    observe(world);
    let entity = slider(
        world,
        WidgetrySliderProps {
            max: 10.0,
            step: Some(3.0),
            ..default()
        },
    );
    world.entity_mut(entity).insert(InteractionDisabled);
    for value in [3.0, 6.0, 9.0, 10.0, 7.0, 4.0, 1.0, 0.0] {
        assert!(WidgetrySlider::set_value_in_world(world, entity, value).unwrap());
        assert!(!WidgetrySlider::set_value_in_world(world, entity, value).unwrap());
    }
    let seen = &world.resource::<Seen>().0;
    assert_eq!(seen.len(), 8);
    for &(source, value, committed, is_final) in seen {
        assert_eq!(source, entity);
        assert_eq!(value, committed);
        assert!(is_final);
    }
}

#[test]
fn invalid_values_and_targets_are_errors_without_mutation_or_events() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let world = app.world_mut();
    observe(world);
    let entity = slider(
        world,
        WidgetrySliderProps {
            max: 10.0,
            step: Some(3.0),
            value: Some(4.0),
            ..default()
        },
    );
    let log = LogCapture::default();
    log.run(|| {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 11.0, 4.5] {
            let error = WidgetrySlider::set_value_in_world(world, entity, value).unwrap_err();
            assert_eq!(error.severity(), Severity::Error);
            assert_eq!(WidgetrySlider::value(world, entity).unwrap(), 4.0);
        }
    });
    assert_eq!(log.records().len(), 6);
    let bare = world.spawn_empty().id();
    let incomplete = world.spawn(WidgetrySlider).id();
    let stale = world.spawn_empty().id();
    world.despawn(stale);
    for invalid in [bare, incomplete, stale] {
        assert!(WidgetrySlider::value(world, invalid).is_err());
        assert!(WidgetrySlider::set_value_in_world(world, invalid, 0.0).is_err());
    }
    assert!(world.resource::<Seen>().0.is_empty());
}

#[test]
fn construction_rejects_invalid_configuration_and_cleans_reservations() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let world = app.world_mut();
    let bad = [
        WidgetrySliderProps {
            min: f32::NAN,
            ..default()
        },
        WidgetrySliderProps {
            max: f32::INFINITY,
            ..default()
        },
        WidgetrySliderProps {
            min: 100.0,
            ..default()
        },
        WidgetrySliderProps {
            min: 101.0,
            ..default()
        },
        WidgetrySliderProps {
            min: -f32::MAX,
            max: f32::MAX,
            ..default()
        },
        WidgetrySliderProps {
            step: Some(0.0),
            ..default()
        },
        WidgetrySliderProps {
            step: Some(-1.0),
            ..default()
        },
        WidgetrySliderProps {
            step: Some(f32::NAN),
            ..default()
        },
        WidgetrySliderProps {
            step: Some(f32::INFINITY),
            ..default()
        },
        WidgetrySliderProps {
            value: Some(f32::NEG_INFINITY),
            ..default()
        },
        WidgetrySliderProps {
            value: Some(101.0),
            ..default()
        },
        WidgetrySliderProps {
            max: 10.0,
            step: Some(3.0),
            value: Some(4.5),
            ..default()
        },
    ];
    let before = world.entities().len();
    for props in bad {
        assert!(spawn_scene(world, bsn! { @WidgetrySlider { @min: {props.min}, @max: {props.max}, @step: {props.step}, @value: {props.value} } }).is_err());
        assert_eq!(world.entities().len(), before);
    }
}

#[test]
fn queued_writes_validate_when_applied_and_route_failures_to_host() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetrySliderPlugin);
    app.set_error_handler(ErrorCapture::handler());
    observe(app.world_mut());
    let entity = slider(app.world_mut(), default());
    let stale = slider(app.world_mut(), default());
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, app.world());
        WidgetrySlider::set_value(&mut commands, entity, 4.5);
        WidgetrySlider::set_value(&mut commands, stale, 1.0);
        WidgetrySlider::set_value(&mut commands, entity, f32::NAN);
        let reserved = commands
            .spawn_scene_with_error_handler(bsn! { @WidgetrySlider { @min: 5.0, @max: 6.0 } })
            .id();
        WidgetrySlider::set_value(&mut commands, reserved, 5.5);
    }
    assert_eq!(WidgetrySlider::value(app.world(), entity).unwrap(), 0.0);
    app.world_mut().despawn(stale);
    let capture = ErrorCapture::default();
    capture.run(|| queue.apply(app.world_mut()));
    let errors = capture.take();
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == Severity::Error)
    );
    assert_eq!(WidgetrySlider::value(app.world(), entity).unwrap(), 4.5);
    assert_eq!(app.world().resource::<Seen>().0.len(), 2);
}

#[test]
fn deferred_invalid_scenes_report_one_error_log_per_failed_construction() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let existing = app.world_mut().spawn(Name::new("ExistingRoot")).id();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let reserved = {
        let mut commands = Commands::new(&mut queue, app.world());
        commands
            .entity(existing)
            .apply_scene_with_error_handler(bsn! { @WidgetrySlider { @step: {Some(-1.0)} } });
        commands
            .spawn_scene_with_error_handler(bsn! { @WidgetrySlider { @min: 10.0, @max: 0.0 } })
            .id()
    };
    let log = LogCapture::default();
    let errors = ErrorCapture::default();
    log.run(|| errors.run(|| queue.apply(app.world_mut())));
    assert_eq!(log.records().len(), 2);
    let failures = errors.take();
    assert_eq!(failures.len(), 2);
    assert!(
        failures
            .iter()
            .all(|error| error.severity() == Severity::Error)
    );
    assert!(!app.world().entities().contains(reserved));
    assert!(app.world().entities().contains(existing));
}
