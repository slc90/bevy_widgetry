#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_check_box::WidgetryTriStateCheckboxHeadless;
use bevy_widgetry_check_box::{
    CheckBoxError, CheckBoxFailure, WidgetryCheckBox, WidgetryCheckBoxColorOverrides,
    WidgetryCheckBoxPlugin, WidgetryCheckState, WidgetryTriStateCheckbox,
};
use bevy_widgetry_core::{color::WidgetryStyleOwner, diagnostics::CoreError};
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;

fn fixture() -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCheckBox })
        .unwrap()
        .id();
    (app, root)
}

#[test]
fn color_failures_keep_core_context_origin_and_managed_ownership() {
    let (mut app, root) = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(!WidgetryCheckBoxColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
        assert!(logs.records().is_empty());
        let mut colors = WidgetryCheckBoxColorOverrides::default();
        colors.indeterminate.hovered.mark = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let failure: CheckBoxFailure = WidgetryCheckBoxColorOverrides::set_in_world(app.world_mut(), root, colors).unwrap_err();
        assert!(matches!(failure.kind(), CheckBoxError::Core { source: CoreError::InvalidColor { .. } }));
        assert!(failure.kind().source().unwrap().is::<CoreError>());
        let context = format!("{:?}", failure.contexts());
        assert!(context.contains("indeterminate") && context.contains("hovered") && context.contains("mark"));
        assert_eq!(*WidgetryCheckBoxColorOverrides::get(app.world(), root).unwrap(), WidgetryCheckBoxColorOverrides::default());
        let origin = failure.origin().clone();
        let report = failure.into_bevy_error();
        let diagnostic = report.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
        assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
        let owner = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(root).insert(WidgetryStyleOwner::<WidgetryCheckBox>::new(owner));
        let failure = WidgetryCheckBoxColorOverrides::clear_in_world(app.world_mut(), root).unwrap_err();
        assert!(matches!(failure.kind(), CheckBoxError::ManagedColors { root: target, owner: actual } if *target == root && *actual == owner));
    });
}

#[test]
fn queued_state_and_color_failures_keep_domain_identity_at_the_host() {
    let (mut app, _) = fixture();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    WidgetryTriStateCheckboxHeadless::set_state(
        &mut app.world_mut().commands(),
        stale,
        WidgetryCheckState::Checked,
    );
    WidgetryCheckBoxColorOverrides::clear(&mut app.world_mut().commands(), stale);
    logs.run(|| errors.run(|| app.world_mut().flush()));
    let records = errors.take_records();
    assert_eq!(records.len(), 2);
    for record in records {
        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
        assert!(
            matches!(diagnostic.source().unwrap().downcast_ref::<CheckBoxError>(), Some(CheckBoxError::InvalidTarget { root }) if *root == stale)
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
}

#[test]
fn initial_colors_keep_original_core_identity_through_scene_cleanup() {
    let (mut app, _) = fixture();
    let count = app.world().entities().len();
    let logs = LogCapture::default();
    logs.run(|| {
        let mut colors = WidgetryCheckBoxColorOverrides::default();
        colors.checked.disabled.foreground = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let error = bevy_widgetry_core::scene::spawn_scene(
            app.world_mut(),
            bsn! { @WidgetryTriStateCheckbox { @colors: {colors.clone()} } },
        )
        .unwrap_err();
        let diagnostic = error.diagnostics().diagnostics().next().unwrap();
        let mut source: &(dyn Error + 'static) = diagnostic;
        while !source.is::<CheckBoxFailure>() {
            source = source.source().unwrap();
        }
        let failure = source.downcast_ref::<CheckBoxFailure>().unwrap();
        assert!(matches!(failure.kind(), CheckBoxError::Core { .. }));
        assert!(std::sync::Arc::ptr_eq(
            failure.origin(),
            diagnostic.origin()
        ));
        assert_eq!(
            logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(app.world().entities().len(), count);
    });
}
