#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_number_input::{
    NumberInputError, NumberInputRejection, NumberInputValue, WidgetryNumberInput,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture, scene_app};
use bevy_widgetry_text_field::WidgetryTextFieldHeadless;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn large_integer_rejection_keeps_exact_typed_request_and_limits() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetryNumberInput::<u128> { @value: {Some(u128::MAX - 1)}, @max: {Some(u128::MAX - 1)} } }).unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        let error =
            WidgetryNumberInput::<u128>::set_value_in_world(app.world_mut(), root, u128::MAX)
                .unwrap_err();
        let NumberInputError::Numeric { config, reason } = error.kind() else {
            panic!("{error}");
        };
        assert_eq!(*reason, NumberInputRejection::OutOfRange);
        assert_eq!(config.numeric_type, "u128");
        assert_eq!(config.value, Some(NumberInputValue::Unsigned(u128::MAX)));
        assert_eq!(config.max, Some(NumberInputValue::Unsigned(u128::MAX - 1)));
        let origin = error.origin().clone();
        let error = error.into_bevy_error();
        let diagnostic = error
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(&origin, diagnostic.origin()));
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .is_empty()
        );
    });
    assert_eq!(
        WidgetryNumberInput::<u128>::value(app.world(), root).unwrap(),
        u128::MAX - 1
    );
}

#[test]
fn every_supported_numeric_type_keeps_its_original_snapshot_category() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    macro_rules! check {
        ($variant:ident; $($t:ty),+ $(,)?) => {$(
            let root = spawn_scene(app.world_mut(), bsn! { @WidgetryNumberInput::<$t> { @value: {Some(1 as $t)}, @max: {Some(2 as $t)} } }).unwrap();
            let error = WidgetryNumberInput::<$t>::set_value_in_world(app.world_mut(), root, 3 as $t).unwrap_err();
            let NumberInputError::Numeric { config, reason } = error.kind() else { panic!("{error}"); };
            assert_eq!(*reason, NumberInputRejection::OutOfRange);
            assert_eq!(config.numeric_type, std::any::type_name::<$t>());
            assert_eq!(config.value, Some(NumberInputValue::$variant(3 as _)));
            assert_eq!(config.max, Some(NumberInputValue::$variant(2 as _)));
        )+};
    }
    check!(Signed; i8, i16, i32, i64, i128, isize);
    check!(Unsigned; u8, u16, u32, u64, u128, usize);
    check!(Float32; f32);
    check!(Float64; f64);
}

#[test]
fn float_rejections_keep_nan_payload_and_signed_limits() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryNumberInput::<f32> { @value: {Some(0.0)} } },
    )
    .unwrap();
    let requested = f32::from_bits(0xffc0_4321);
    let error = WidgetryNumberInput::<f32>::set_value_in_world(app.world_mut(), root, requested)
        .unwrap_err();
    let NumberInputError::Numeric { config, .. } = error.kind() else {
        panic!("{error}");
    };
    let Some(NumberInputValue::Float32(value)) = config.value else {
        panic!("{error}");
    };
    assert_eq!(value.to_bits(), requested.to_bits());
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetryNumberInput::<i128> { @value: {Some(i128::MIN + 1)}, @min: {Some(i128::MIN + 1)} } }).unwrap();
    let error = WidgetryNumberInput::<i128>::set_value_in_world(app.world_mut(), root, i128::MIN)
        .unwrap_err();
    let NumberInputError::Numeric { config, .. } = error.kind() else {
        panic!("{error}");
    };
    assert_eq!(config.value, Some(NumberInputValue::Signed(i128::MIN)));
    assert_eq!(config.min, Some(NumberInputValue::Signed(i128::MIN + 1)));
}

fn original(
    diagnostic: &dyn bevy_widgetry_diagnostics::bevy::Diagnostic,
) -> &bevy_widgetry_number_input::NumberInputFailure {
    use std::error::Error;
    let mut cause: &dyn Error = diagnostic;
    loop {
        if let Some(error) = cause.downcast_ref::<bevy_widgetry_number_input::NumberInputFailure>()
        {
            return error;
        }
        cause = cause.source().unwrap();
    }
}

#[test]
fn initial_configuration_rejections_preserve_original_values_and_reason() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    use bevy_widgetry_number_input::WidgetryNumberInputProps;
    for (props, expected) in [
        (
            WidgetryNumberInputProps::<f64>::default(),
            NumberInputRejection::MissingValue,
        ),
        (
            WidgetryNumberInputProps {
                value: Some(1.0),
                min: Some(f64::NEG_INFINITY),
                ..default()
            },
            NumberInputRejection::NonFiniteBounds,
        ),
        (
            WidgetryNumberInputProps {
                value: Some(1.0),
                min: Some(5.0),
                max: Some(2.0),
                ..default()
            },
            NumberInputRejection::ReversedBounds,
        ),
        (
            WidgetryNumberInputProps {
                value: Some(1.0),
                step: f64::NAN,
                ..default()
            },
            NumberInputRejection::InvalidStep,
        ),
        (
            WidgetryNumberInputProps {
                value: Some(f64::INFINITY),
                ..default()
            },
            NumberInputRejection::OutOfRange,
        ),
    ] {
        let error = spawn_scene(app.world_mut(), bsn! { @WidgetryNumberInput::<f64> { @value: {props.value}, @min: {props.min}, @max: {props.max}, @step: {props.step}, @decimals: {props.decimals} } }).unwrap_err().into_bevy_error();
        let diagnostic = error
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        let NumberInputError::Numeric { config, reason } = original(diagnostic).kind() else {
            panic!("{error}");
        };
        assert_eq!(*reason, expected);
        assert_eq!(config.numeric_type, "f64");
        let NumberInputValue::Float64(step) = config.step else {
            panic!("{error}");
        };
        assert_eq!(step.to_bits(), props.step.to_bits());
        assert!(std::sync::Arc::ptr_eq(
            diagnostic.origin(),
            original(diagnostic).origin()
        ));
    }
    let error = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryNumberInput::<u64> { @value: {Some(1)}, @decimals: {Some(255)} } },
    )
    .unwrap_err()
    .into_bevy_error();
    let diagnostic = error
        .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap();
    assert!(
        matches!(original(diagnostic).kind(), NumberInputError::Numeric { config, reason: NumberInputRejection::IntegerDecimals } if config.decimals == Some(255) && config.numeric_type == "u64")
    );
}

#[test]
fn text_field_failure_keeps_source_origin_and_both_operation_contexts() {
    use bevy_widgetry_core::input_target::WidgetryInputTarget;
    use bevy_widgetry_number_input::WidgetryNumberInputPlugin;
    use bevy_widgetry_test_utils::ErrorCapture;
    use bevy_widgetry_text_field::{TextFieldError, WidgetryTextField};
    use std::error::Error;
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryNumberInput::<f64> { @value: {Some(1.0)} } },
    )
    .unwrap();
    let field = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|entity| app.world().get::<WidgetryTextField>(*entity).is_some())
        .unwrap();
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    app.world_mut()
        .entity_mut(editor)
        .remove::<bevy::text::EditableText>();
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| {
        errors.run(|| {
            WidgetryNumberInput::<f64>::set_value(&mut app.world_mut().commands(), root, 2.0);
            app.world_mut().flush();
        })
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
    assert!(
        matches!(original(diagnostic).kind(), NumberInputError::TextField { source: TextFieldError::MissingPart { entity, component: "EditableText", .. } } if *entity == editor)
    );
    assert!(
        original(diagnostic)
            .kind()
            .source()
            .unwrap()
            .is::<TextFieldError>()
    );
    for operation in ["text_field.editor_binding", "number_input.editor"] {
        assert!(
            diagnostic
                .contexts()
                .iter()
                .any(|frame| frame.operation() == operation)
        );
    }
    assert_eq!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert_eq!(logs.records().len(), 1);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        1.0
    );
}

#[test]
fn invalid_input_target_is_rejected_even_when_the_text_fields_own_editor_is_valid() {
    use bevy_widgetry_core::input_target::WidgetryInputTarget;
    use bevy_widgetry_number_input::WidgetryNumberInputPlugin;
    use bevy_widgetry_text_field::WidgetryTextField;
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    for (stale, disabled) in [(false, false), (true, false), (false, true), (true, true)] {
        let root = spawn_scene(
            app.world_mut(),
            bsn! { @WidgetryNumberInput::<f64> { @value: {Some(1.0)} } },
        )
        .unwrap();
        let field = app
            .world()
            .get::<Children>(root)
            .unwrap()
            .iter()
            .find(|entity| app.world().get::<WidgetryTextField>(*entity).is_some())
            .unwrap();
        let target = app.world_mut().spawn_empty().id();
        if stale {
            app.world_mut().despawn(target);
        }
        if disabled {
            app.world_mut()
                .entity_mut(root)
                .insert(bevy::ui::InteractionDisabled);
        }
        app.world_mut()
            .entity_mut(field)
            .insert(WidgetryInputTarget { target });
        assert!(WidgetryTextFieldHeadless::editor(app.world(), field).is_ok());
        let before = WidgetryTextFieldHeadless::text(app.world(), field).unwrap();
        let logs = LogCapture::default();
        logs.run(|| {
            let error = WidgetryNumberInput::<f64>::set_value_in_world(app.world_mut(), root, 2.0).unwrap_err();
            assert!(matches!(error.kind(), NumberInputError::MissingPart { root: owner, entity, component: "EditableText" } if *owner == root && *entity == target));
            assert_eq!(logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source).len(), 1);
        });
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
            1.0
        );
        assert_eq!(
            WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
            before
        );
    }
}
