#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{ecs::error::Severity, prelude::*, ui_widgets::ValueChange};
use bevy_widgetry_core::scene::{WidgetrySceneCommandsExt, spawn_scene};
use bevy_widgetry_number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Resource, Default)]
struct Seen(Vec<(Entity, u128, u128, bool)>);

#[test]
fn exact_large_values_commit_before_one_root_notification() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);

    let world = app.world_mut();
    world.init_resource::<Seen>();
    world.add_observer(
        |event: On<ValueChange<u128>>, mut world: bevy::ecs::world::DeferredWorld| -> Result {
            let current = WidgetryNumberInput::<u128>::value(&world, event.source)?;
            world.resource_mut::<Seen>().0.push((
                event.source,
                event.value,
                current,
                event.is_final,
            ));
            Ok(())
        },
    );
    let root = spawn_scene(
        world,
        bsn! { @WidgetryNumberInput<u128> { @value: {Some(u128::MAX - 1)}, @step: 7 } },
    )
    .unwrap();
    assert!(world.resource::<Seen>().0.is_empty());
    assert_eq!(
        WidgetryNumberInput::<u128>::value(world, root).unwrap(),
        u128::MAX - 1
    );
    world.entity_mut(root).insert(bevy::ui::InteractionDisabled);
    assert!(WidgetryNumberInput::<u128>::set_value_in_world(world, root, u128::MAX).unwrap());
    assert!(!WidgetryNumberInput::<u128>::set_value_in_world(world, root, u128::MAX).unwrap());
    assert_eq!(
        world.resource::<Seen>().0,
        vec![(root, u128::MAX, u128::MAX, true)]
    );
}

#[test]
fn all_supported_types_preserve_values_and_allow_off_grid_writes() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    let world = app.world_mut();
    macro_rules! check {
        ($($t:ty),+) => {$(
            let root = spawn_scene(world, bsn! { @WidgetryNumberInput<$t> { @value: {Some(2 as $t)}, @step: {3 as $t} } }).unwrap();
            assert!(WidgetryNumberInput::<$t>::set_value_in_world(world, root, 4 as $t).unwrap());
            assert_eq!(WidgetryNumberInput::<$t>::value(world, root).unwrap(), 4 as $t);
            assert!(!WidgetryNumberInput::<$t>::set_value_in_world(world, root, 4 as $t).unwrap());
        )+};
    }
    check!(
        i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
    );
    let root = spawn_scene(
        world,
        bsn! { @WidgetryNumberInput<i128> { @value: {Some(i128::MIN)} } },
    )
    .unwrap();
    assert_eq!(
        WidgetryNumberInput::<i128>::value(world, root).unwrap(),
        i128::MIN
    );
}

#[test]
fn invalid_construction_cleans_roots_and_fixed_ranges_are_legal() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    let world = app.world_mut();
    let before = world.entities().len();
    assert!(spawn_scene(world, bsn! { @WidgetryNumberInput<i32> }).is_err());
    for (value, min, max, step) in [
        (0.0, Some(2.0), Some(1.0), 1.0),
        (0.0, None, None, 0.0),
        (0.0, None, None, -1.0),
        (0.0, None, None, f64::NAN),
        (0.0, None, None, f64::INFINITY),
        (f64::NAN, None, None, 1.0),
        (f64::INFINITY, None, None, 1.0),
        (f64::NEG_INFINITY, None, None, 1.0),
        (0.0, Some(f64::NEG_INFINITY), None, 1.0),
        (0.0, None, Some(f64::NAN), 1.0),
        (-1.0, Some(0.0), None, 1.0),
        (2.0, None, Some(1.0), 1.0),
    ] {
        assert!(spawn_scene(world, bsn! { @WidgetryNumberInput<f64> { @value: {Some(value)}, @min: {min}, @max: {max}, @step: {step} } }).is_err());
        assert_eq!(world.entities().len(), before);
    }
    let root = spawn_scene(
        world,
        bsn! { @WidgetryNumberInput<i32> { @value: {Some(3)}, @min: {Some(3)}, @max: {Some(3)} } },
    )
    .unwrap();
    assert!(!WidgetryNumberInput::<i32>::set_value_in_world(world, root, 3).unwrap());
    assert!(WidgetryNumberInput::<i32>::set_value_in_world(world, root, 4).is_err());
}

#[test]
fn invalid_writes_and_targets_preserve_state_and_events() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    let world = app.world_mut();
    let root = spawn_scene(world, bsn! { @WidgetryNumberInput<f32> { @value: {Some(1.0)}, @min: {Some(0.0)}, @max: {Some(2.0)} } }).unwrap();
    world.init_resource::<Seen>();
    world.add_observer(|_: On<ValueChange<f32>>, mut seen: ResMut<Seen>| {
        seen.0.push((Entity::PLACEHOLDER, 0, 0, false));
    });
    let logs = LogCapture::default();
    logs.run(|| {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 3.0] {
            let error =
                WidgetryNumberInput::<f32>::set_value_in_world(world, root, value).unwrap_err();
            assert_eq!(error.severity(), Severity::Error);
            assert_eq!(WidgetryNumberInput::<f32>::value(world, root).unwrap(), 1.0);
        }
    });
    assert_eq!(logs.records().len(), 5);
    let bare = world.spawn_empty().id();
    let broken = world.spawn(WidgetryNumberInput::<f32>::default()).id();
    let stale = world.spawn_empty().id();
    world.despawn(stale);
    for target in [bare, broken, stale] {
        assert!(WidgetryNumberInput::<f32>::value(world, target).is_err());
        assert!(WidgetryNumberInput::<f32>::set_value_in_world(world, target, 1.0).is_err());
    }
    assert!(WidgetryNumberInput::<f64>::value(world, root).is_err());
    assert!(world.resource::<Seen>().0.is_empty());
}

#[test]
fn commands_validate_at_execution_and_use_host_error_handler() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryNumberInput<i32> { @value: {Some(2)}, @min: {Some(0)} } },
    )
    .unwrap();
    let stale = app.world_mut().spawn_empty().id();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let reserved;
    {
        let mut commands = Commands::new(&mut queue, app.world());
        WidgetryNumberInput::<i32>::set_value(&mut commands, root, 7);
        WidgetryNumberInput::<i32>::set_value(&mut commands, root, -1);
        WidgetryNumberInput::<i32>::set_value(&mut commands, stale, 1);
        reserved = commands
            .spawn_scene_with_error_handler(
                bsn! { @WidgetryNumberInput<i32> { @value: {Some(8)} } },
            )
            .id();
        WidgetryNumberInput::<i32>::set_value(&mut commands, reserved, 9);
    }
    assert_eq!(
        WidgetryNumberInput::<i32>::value(app.world(), root).unwrap(),
        2
    );
    app.world_mut().despawn(stale);
    let errors = ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    let errors = errors.take();
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == Severity::Error)
    );
    assert_eq!(
        WidgetryNumberInput::<i32>::value(app.world(), root).unwrap(),
        7
    );
    assert_eq!(
        WidgetryNumberInput::<i32>::value(app.world(), reserved).unwrap(),
        9
    );
}
