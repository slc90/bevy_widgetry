#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::scene::{WidgetrySceneCommandsExt, spawn_scene};
use bevy_widgetry_slider::WidgetrySliderHeadless;
use bevy_widgetry_slider::{
    SliderError, SliderFailure, SliderValueRejection, WidgetrySlider, WidgetrySliderColorOverrides,
    WidgetrySliderPlugin, WidgetrySliderProps, WidgetrySliderStateColorOverrides,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn rejected_requests_keep_exact_numeric_configuration_and_identity() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetrySlider { @min: 0.0, @max: 10.0, @step: {Some(3.0)}, @value: {Some(4.0)} } },
    )
    .unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        for (value, reason) in [
            (f32::NAN, SliderValueRejection::OutOfRange),
            (f32::INFINITY, SliderValueRejection::OutOfRange),
            (-1.0, SliderValueRejection::OutOfRange),
            (11.0, SliderValueRejection::OutOfRange),
            (4.5, SliderValueRejection::Unreachable),
        ] {
            let error = WidgetrySliderHeadless::set_value_in_world(app.world_mut(), root, value)
                .unwrap_err();
            let SliderError::Numeric {
                config,
                reason: actual,
            } = error.kind()
            else {
                panic!("{error}");
            };
            assert_eq!(*actual, reason);
            assert_eq!(config.min, 0.0);
            assert_eq!(config.max, 10.0);
            assert_eq!(config.step, Some(3.0));
            assert_eq!(config.value.to_bits(), value.to_bits());
            let origin = error.origin().clone();
            let error = error.into_bevy_error();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            let managed = error
                .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                .unwrap();
            assert!(std::sync::Arc::ptr_eq(
                managed.diagnostics().next().unwrap().origin(),
                &origin
            ));
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
            assert_eq!(
                WidgetrySliderHeadless::value(app.world(), root).unwrap(),
                4.0
            );
        }
    });
    assert!(
        logs.records()
            .iter()
            .all(|record| !record.fields.contains_key("backtrace"))
    );
}

#[test]
fn configuration_failures_cross_scene_boundary_without_losing_values() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| {
        errors.run(|| {
            app.world_mut()
                .commands()
                .spawn_scene_with_error_handler(bsn! { @WidgetrySlider { @min: 5.0, @max: 1.0 } });
            app.world_mut().flush();
        })
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
    assert_eq!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert!(
        diagnostic.contexts().iter().any(|frame| frame
            .fields()
            .iter()
            .any(|(name, value)| name == "failure"
                && value.contains("min: 5.0")
                && value.contains("max: 1.0")))
    );
}

fn original(diagnostic: &dyn bevy_widgetry_diagnostics::bevy::Diagnostic) -> &SliderFailure {
    use std::error::Error;
    let mut cause: &dyn Error = diagnostic;
    loop {
        if let Some(failure) = cause.downcast_ref::<SliderFailure>() {
            return failure;
        }
        cause = cause.source().unwrap();
    }
}

#[test]
fn all_configuration_rejections_keep_category_and_inputs() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    for (props, reason) in [
        (
            WidgetrySliderProps {
                min: f32::NAN,
                ..default()
            },
            SliderValueRejection::NonFiniteRange,
        ),
        (
            WidgetrySliderProps {
                min: 5.0,
                max: 1.0,
                ..default()
            },
            SliderValueRejection::UnorderedRange,
        ),
        (
            WidgetrySliderProps {
                min: -f32::MAX,
                max: f32::MAX,
                ..default()
            },
            SliderValueRejection::NonFiniteSpan,
        ),
        (
            WidgetrySliderProps {
                step: Some(f32::INFINITY),
                ..default()
            },
            SliderValueRejection::InvalidStep,
        ),
        (
            WidgetrySliderProps {
                step: Some(0.0),
                ..default()
            },
            SliderValueRejection::InvalidStep,
        ),
        (
            WidgetrySliderProps {
                value: Some(-1.0),
                ..default()
            },
            SliderValueRejection::OutOfRange,
        ),
        (
            WidgetrySliderProps {
                max: 10.0,
                step: Some(3.0),
                value: Some(4.5),
                ..default()
            },
            SliderValueRejection::Unreachable,
        ),
    ] {
        let error = spawn_scene(app.world_mut(), bsn! { @WidgetrySlider { @min: {props.min}, @max: {props.max}, @step: {props.step}, @value: {props.value} } }).unwrap_err().into_bevy_error();
        let diagnostic = error
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        let SliderError::Numeric {
            config,
            reason: actual,
        } = original(diagnostic).kind()
        else {
            panic!("{error}");
        };
        assert_eq!(*actual, reason);
        assert_eq!(config.min.to_bits(), props.min.to_bits());
        assert_eq!(config.max.to_bits(), props.max.to_bits());
        assert_eq!(config.step.map(f32::to_bits), props.step.map(f32::to_bits));
        assert_eq!(
            config.value.to_bits(),
            props.value.unwrap_or(props.min).to_bits()
        );
    }
}

#[test]
fn colors_preserve_core_cause_and_palette_while_valid_operations_stay_silent() {
    let _guard = TEST_LOCK.lock().unwrap();
    use std::error::Error;
    let mut app = scene_app();
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetrySlider }).unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        let error = WidgetrySliderColorOverrides::set_in_world(
            app.world_mut(),
            root,
            WidgetrySliderColorOverrides {
                pressed: WidgetrySliderStateColorOverrides {
                    thumb: Some(Color::srgb(f32::NAN, 0.0, 0.0)),
                    ..default()
                },
                ..default()
            },
        )
        .unwrap_err();
        assert!(
            error
                .kind()
                .source()
                .unwrap()
                .is::<bevy_widgetry_core::diagnostics::CoreError>()
        );
        assert_eq!(
            logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        for (field, value) in [("field", "thumb"), ("state", "pressed")] {
            assert!(error.contexts().iter().any(|frame| {
                frame
                    .fields()
                    .iter()
                    .any(|(name, actual)| name == field && actual == value)
            }));
        }
    });
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(!WidgetrySliderHeadless::set_value_in_world(app.world_mut(), root, 0.0).unwrap());
        WidgetrySliderHeadless::set_value_in_world(app.world_mut(), root, 100.0).unwrap();
        assert!(!WidgetrySliderHeadless::set_value_in_world(app.world_mut(), root, 100.0).unwrap());
        assert!(!WidgetrySliderColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
    });
    assert!(logs.records().is_empty());
}

#[test]
fn visual_failures_batch_all_roots_reuse_identity_and_recover() {
    let _guard = TEST_LOCK.lock().unwrap();

    let mut app = scene_app();
    app.add_plugins(WidgetrySliderPlugin);
    app.insert_resource(bevy_widgetry_theme::WIDGETRY_LIGHT_THEME);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let mut thumbs = Vec::new();
    for _ in 0..2 {
        let root = spawn_scene(app.world_mut(), bsn! { @WidgetrySlider }).unwrap();
        let track = app.world().get::<Children>(root).unwrap()[0];
        let thumb = app.world().get::<Children>(track).unwrap()[1];
        app.world_mut().entity_mut(thumb).remove::<Outline>();
        thumbs.push(thumb);
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| app.world_mut().run_schedule(PostUpdate)));
    let first = errors.take_records();
    assert_eq!(first.len(), 1);
    let origins: Vec<_> = first[0].managed().unwrap().diagnostics().map(|diagnostic| {
        assert!(matches!(original(diagnostic).kind(), SliderError::MissingPart { entity, component: "Outline", .. } if thumbs.contains(entity)));
        diagnostic.origin().clone()
    }).collect();
    assert_eq!(origins.len(), 2);
    app.insert_resource(bevy_widgetry_theme::WIDGETRY_DARK_THEME);
    logs.run(|| errors.run(|| app.world_mut().run_schedule(PostUpdate)));
    let repeated = errors.take_records();
    for diagnostic in repeated[0].managed().unwrap().diagnostics() {
        assert!(
            origins
                .iter()
                .any(|origin| std::sync::Arc::ptr_eq(origin, diagnostic.origin()))
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
    for thumb in thumbs {
        app.world_mut().entity_mut(thumb).insert(Outline::default());
    }
    app.insert_resource(bevy_widgetry_theme::WIDGETRY_LIGHT_THEME);
    logs.run(|| errors.run(|| app.world_mut().run_schedule(PostUpdate)));
    assert!(errors.take().is_empty());
    for origin in origins {
        assert_eq!(
            logs.records_for_error(origin.id())
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            1
        );
    }
}
