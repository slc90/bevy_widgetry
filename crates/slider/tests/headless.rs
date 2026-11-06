#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_slider::{
    SliderError, SliderValueRejection, WidgetrySliderHeadless, WidgetrySliderHeadlessPlugin,
    WidgetrySliderHeadlessProps, WidgetrySliderState,
};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        WidgetrySliderHeadlessPlugin,
    ));
    app.init_asset::<bevy::scene::ScenePatch>();
    app
}
fn spawn(world: &mut World, props: WidgetrySliderHeadlessProps) -> Entity {
    spawn_scene(world,bsn! { @WidgetrySliderHeadless { @min: {props.min}, @max: {props.max}, @step: {props.step}, @value: {props.value} } }).unwrap()
}
#[derive(Resource, Default)]
struct Seen(Vec<(Entity, f32, bool)>);
fn observe(world: &mut World) {
    world.init_resource::<Seen>();
    world.add_observer(
        |event: On<bevy::ui_widgets::ValueChange<f32>>,
         state: Query<&WidgetrySliderState>,
         mut seen: ResMut<Seen>| {
            assert_eq!(state.get(event.source).unwrap().value(), event.value);
            seen.0.push((event.source, event.value, event.is_final));
        },
    );
}

#[test]
fn numeric_state_exists_without_ui_or_layout() {
    let mut app = app();
    let world = app.world_mut();
    let root = spawn_scene(
        world,
        bsn! { @WidgetrySliderHeadless { @max: 10.0, @step: {Some(3.0)}, @value: {Some(4.0)} } },
    )
    .unwrap();
    let state = world.get::<WidgetrySliderState>(root).unwrap();
    assert_eq!(
        (state.min(), state.max(), state.step(), state.value()),
        (0.0, 10.0, Some(3.0), 4.0)
    );
    assert!(world.get::<Node>(root).is_none());
    assert!(world.get::<ComputedNode>(root).is_none());
}

#[test]
fn independent_program_changes_commit_before_events_and_preserve_same_value_ticks() {
    let mut app = app();
    let world = app.world_mut();
    observe(world);
    let root = spawn(
        world,
        WidgetrySliderHeadlessProps {
            max: 10.0,
            step: Some(3.0),
            ..default()
        },
    );
    assert!(world.resource::<Seen>().0.is_empty());
    world.entity_mut(root).insert(bevy::ui::InteractionDisabled);
    for value in [3.0, 6.0, 9.0, 10.0, 7.0, 4.0, 1.0, 0.0] {
        world.increment_change_tick();
        assert!(WidgetrySliderHeadless::set_value_in_world(world, root, value).unwrap());
        let tick = world
            .entity(root)
            .get_ref::<WidgetrySliderState>()
            .unwrap()
            .last_changed();
        world.increment_change_tick();
        assert!(!WidgetrySliderHeadless::set_value_in_world(world, root, value).unwrap());
        assert_eq!(
            world
                .entity(root)
                .get_ref::<WidgetrySliderState>()
                .unwrap()
                .last_changed(),
            tick
        );
    }
    assert_eq!(world.resource::<Seen>().0.len(), 8);
    assert!(
        world
            .resource::<Seen>()
            .0
            .iter()
            .all(|(_, _, is_final)| *is_final)
    );
    assert!(world.get::<Children>(root).is_none());
}

#[test]
fn numeric_rejections_retain_snapshot_and_do_not_need_default_parts() {
    let mut app = app();
    let world = app.world_mut();
    observe(world);
    let root = spawn(
        world,
        WidgetrySliderHeadlessProps {
            max: 10.0,
            step: Some(3.0),
            value: Some(4.0),
            ..default()
        },
    );
    for value in [4.5, -1.0, 11.0, f32::NAN, f32::INFINITY] {
        let error = WidgetrySliderHeadless::set_value_in_world(world, root, value).unwrap_err();
        match error.kind() {
            SliderError::Numeric { config, .. } => {
                assert_eq!(config.min, 0.0);
                assert_eq!(config.max, 10.0);
                assert_eq!(config.step, Some(3.0));
                assert_eq!(config.value.to_bits(), value.to_bits());
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(
            WidgetrySliderHeadless::state(world, root).unwrap().value(),
            4.0
        );
    }
    let missing = world.spawn(WidgetrySliderHeadless).id();
    assert!(
        matches!(WidgetrySliderHeadless::value(world,missing).unwrap_err().kind(),SliderError::MissingPart{root:owner,component:"WidgetrySliderState",..} if *owner==missing)
    );
    let wrong = world.spawn_empty().id();
    world.despawn(wrong);
    assert!(
        matches!(WidgetrySliderHeadless::value(world,wrong).unwrap_err().kind(),SliderError::InvalidRoot{root:owner} if *owner==wrong)
    );
    assert!(world.resource::<Seen>().0.is_empty());
}

#[test]
fn independent_invalid_construction_cleans_roots_and_keeps_numeric_reasons() {
    let mut app = app();
    let world = app.world_mut();
    let cases = [
        (
            WidgetrySliderHeadlessProps {
                min: f32::NAN,
                ..default()
            },
            SliderValueRejection::NonFiniteRange,
        ),
        (
            WidgetrySliderHeadlessProps {
                min: 100.0,
                ..default()
            },
            SliderValueRejection::UnorderedRange,
        ),
        (
            WidgetrySliderHeadlessProps {
                min: -f32::MAX,
                max: f32::MAX,
                ..default()
            },
            SliderValueRejection::NonFiniteSpan,
        ),
        (
            WidgetrySliderHeadlessProps {
                step: Some(0.0),
                ..default()
            },
            SliderValueRejection::InvalidStep,
        ),
        (
            WidgetrySliderHeadlessProps {
                step: Some(f32::INFINITY),
                ..default()
            },
            SliderValueRejection::InvalidStep,
        ),
        (
            WidgetrySliderHeadlessProps {
                value: Some(f32::NAN),
                ..default()
            },
            SliderValueRejection::OutOfRange,
        ),
        (
            WidgetrySliderHeadlessProps {
                max: 10.0,
                step: Some(3.0),
                value: Some(4.5),
                ..default()
            },
            SliderValueRejection::Unreachable,
        ),
    ];
    for (props, expected) in cases {
        let before = world.entities().count_spawned();
        let failure=spawn_scene(world,bsn! { @WidgetrySliderHeadless { @min: {props.min}, @max: {props.max}, @step: {props.step}, @value: {props.value} } }).unwrap_err();
        let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
        let mut source: Option<&(dyn std::error::Error + 'static)> = Some(diagnostic);
        let mut reason = None;
        while let Some(error) = source {
            if let Some(SliderError::Numeric { reason: value, .. }) =
                error.downcast_ref::<SliderError>()
            {
                reason = Some(*value);
                break;
            }
            source = error.source();
        }
        assert_eq!(reason, Some(expected));
        assert_eq!(world.entities().count_spawned(), before);
    }
}

#[test]
fn queued_targets_are_checked_at_execution_and_other_owners_continue() {
    let mut app = app();
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    let world = app.world_mut();
    observe(world);
    let bad = spawn(world, default());
    let good = spawn(world, default());
    {
        let mut commands = world.commands();
        commands.entity(bad).remove::<WidgetrySliderHeadless>();
        WidgetrySliderHeadless::set_value(&mut commands, bad, 5.0);
        WidgetrySliderHeadless::set_value(&mut commands, good, 25.0);
    }
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| world.flush());
    let records = errors.take();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(WidgetrySliderHeadless::value(world, good).unwrap(), 25.0);
    assert_eq!(world.get::<WidgetrySliderState>(bad).unwrap().value(), 0.0);
    assert_eq!(world.resource::<Seen>().0, [(good, 25.0, true)]);
}
