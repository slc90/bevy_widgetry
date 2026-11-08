#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
use bevy_widgetry_ui_scale::{UiScaleError, WidgetryUiScale};
use rstest::rstest;
use std::sync::Arc;

#[rstest]
#[case(0.0)]
#[case(-1.0)]
#[case(f32::NAN)]
#[case(f32::INFINITY)]
#[case(f32::NEG_INFINITY)]
fn invalid_scale_preserves_the_previous_setting_and_diagnostic(#[case] requested: f32) {
    let capture = LogCapture::default();
    let mut scale = WidgetryUiScale::default();
    let failure = capture.run(|| scale.set(requested).unwrap_err());
    assert_eq!(scale, WidgetryUiScale::default());
    assert!(
        matches!(failure.kind(), UiScaleError::InvalidScale { requested: value } if value.to_bits() == requested.to_bits())
    );
    let origin = Arc::clone(failure.origin());
    let failure = capture.run(|| failure.with_context(ErrorFrame::new("host.configure")));
    assert!(Arc::ptr_eq(&origin, failure.origin()));
    assert_eq!(
        capture
            .diagnostic_records(origin.id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert!(
        capture
            .diagnostic_records(origin.id(), DiagnosticPhase::Handled)
            .is_empty()
    );
}

#[rstest]
#[case(f32::MIN_POSITIVE)]
#[case(f32::MAX)]
#[case(0.75)]
#[case(1.375)]
fn library_accepts_finite_positive_scales_beyond_gallery_choices(#[case] requested: f32) {
    assert_eq!(WidgetryUiScale::new(requested).unwrap().get(), requested);
}

#[test]
fn host_receives_original_scale_diagnostic_from_a_real_system() {
    let errors = ErrorCapture::default();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .set_error_handler(ErrorCapture::handler())
        .init_resource::<WidgetryUiScale>()
        .add_systems(Startup, |mut scale: ResMut<WidgetryUiScale>| -> Result {
            scale
                .set(0.0)
                .map_err(|failure| failure.into_bevy_error())?;
            Ok(())
        });
    app.edit_schedule(Startup, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    errors.run(|| app.update());
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
    let failure = (diagnostic as &dyn std::error::Error)
        .downcast_ref::<bevy_widgetry_ui_scale::UiScaleFailure>()
        .unwrap();
    assert!(matches!(
        failure.kind(),
        UiScaleError::InvalidScale { requested: 0.0 }
    ));
    assert_eq!(app.world().resource::<WidgetryUiScale>().get(), 1.0);
}
