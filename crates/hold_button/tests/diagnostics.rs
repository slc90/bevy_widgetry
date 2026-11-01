#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::CoreError;
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_hold_button::{
    HoldButtonError, HoldButtonFailure, WidgetryHoldButton, WidgetryHoldButtonColorOverrides,
    WidgetryHoldButtonPlugin,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;

fn fixture() -> (App, Entity) {
    let mut app = scene_app();
    app.init_asset::<bevy::shader::Shader>();
    app.add_plugins(WidgetryHoldButtonPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryHoldButton })
        .unwrap()
        .id();
    (app, root)
}

#[test]
fn invalid_diameters_keep_input_and_source_identity_and_leave_no_empty_root() {
    let (mut app, _) = fixture();
    for diameter in [f32::NAN, f32::INFINITY, 0.0, -10.0] {
        let count = app.world().entities().len();
        let logs = LogCapture::default();
        logs.run(|| {
            let error = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! {
                @WidgetryHoldButton { @diameter: diameter }
            }).unwrap_err();
            let diagnostic = error.diagnostics().diagnostics().next().unwrap();
            let mut source: &(dyn Error + 'static) = diagnostic;
            while !source.is::<HoldButtonFailure>() { source = source.source().unwrap(); }
            let failure = source.downcast_ref::<HoldButtonFailure>().unwrap();
            assert!(matches!(failure.kind(), HoldButtonError::InvalidDiameter { diameter: actual, .. } if actual.to_bits() == diameter.to_bits()));
            assert!(std::sync::Arc::ptr_eq(failure.origin(), diagnostic.origin()));
            assert_eq!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source).len(), 1);
            assert!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Handled).is_empty());
            assert_eq!(app.world().entities().len(), count);
        });
    }
}

#[test]
fn invalid_colors_keep_core_cause_context_and_old_configuration() {
    let (mut app, root) = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(!WidgetryHoldButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
        assert!(logs.records().is_empty());
        let mut colors = WidgetryHoldButtonColorOverrides::default();
        colors.pressed.wave = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let failure = WidgetryHoldButtonColorOverrides::set_in_world(app.world_mut(), root, colors)
            .unwrap_err();
        assert!(matches!(
            failure.kind(),
            HoldButtonError::Core {
                source: CoreError::InvalidColor { .. }
            }
        ));
        assert!(failure.kind().source().unwrap().is::<CoreError>());
        let context = format!("{:?}", failure.contexts());
        assert!(context.contains("pressed") && context.contains("wave"));
        assert_eq!(
            *WidgetryHoldButtonColorOverrides::get(app.world(), root).unwrap(),
            WidgetryHoldButtonColorOverrides::default()
        );
        let origin = failure.origin().clone();
        let report = failure.into_bevy_error();
        let diagnostic = report
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    });
}

#[test]
fn queued_invalid_color_target_reaches_host_with_original_identity() {
    let (mut app, _) = fixture();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    app.set_error_handler(ErrorCapture::handler());
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    WidgetryHoldButtonColorOverrides::clear(&mut app.world_mut().commands(), stale);
    logs.run(|| errors.run(|| app.world_mut().flush()));
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let diagnostic = records[0]
        .error
        .downcast_ref::<WidgetryEcsError>()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap();
    assert!(
        matches!(diagnostic.source().unwrap().downcast_ref::<HoldButtonError>(), Some(HoldButtonError::InvalidTarget { root }) if *root == stale)
    );
    assert_eq!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
            .is_empty()
    );
}
