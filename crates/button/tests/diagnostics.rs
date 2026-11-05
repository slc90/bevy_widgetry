#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_button::{
    ButtonError, ButtonFailure, WidgetryButton, WidgetryButtonColorOverrides,
    WidgetryButtonLoading, WidgetryButtonPlugin,
};
use bevy_widgetry_core::{color::WidgetryStyleOwner, diagnostics::CoreError};
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;

fn fixture() -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    (app, root)
}

#[test]
fn invalid_targets_and_managed_color_owners_are_distinct_domain_failures() {
    let (mut app, root) = fixture();
    let wrong = app.world_mut().spawn(Node::default()).id();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    let logs = LogCapture::default();
    logs.run(|| {
        for target in [wrong, stale] {
            let failure: ButtonFailure = WidgetryButtonLoading::set_in_world(app.world_mut(), target, true).unwrap_err();
            assert!(matches!(failure.kind(), ButtonError::InvalidTarget { root } if *root == target));
            assert_eq!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source).len(), 1);
        }
        app.world_mut().entity_mut(root).insert(WidgetryStyleOwner::<WidgetryButton>::new(wrong));
        let failure = WidgetryButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap_err();
        assert!(matches!(failure.kind(), ButtonError::ManagedColors { root: target, owner } if *target == root && *owner == wrong));
        assert_eq!(*WidgetryButtonColorOverrides::get(app.world(), root).unwrap(), WidgetryButtonColorOverrides::default());
    });
}

#[test]
fn core_color_failure_keeps_source_origin_and_request_context_without_writing_colors() {
    let (mut app, root) = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        let mut colors = WidgetryButtonColorOverrides::default();
        colors.hovered.border = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let failure = WidgetryButtonColorOverrides::set_in_world(app.world_mut(), root, colors).unwrap_err();
        assert!(matches!(failure.kind(), ButtonError::Core { source: CoreError::InvalidColor { color } } if color.to_linear().red.is_nan()));
        assert!(failure.kind().source().unwrap().is::<CoreError>());
        let contexts = format!("{:?}", failure.contexts());
        assert!(contexts.contains(&root.to_string()));
        assert!(contexts.contains("hovered"));
        assert!(contexts.contains("border"));
        assert_eq!(*WidgetryButtonColorOverrides::get(app.world(), root).unwrap(), WidgetryButtonColorOverrides::default());
        assert_eq!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Handled).is_empty());
        let origin = failure.origin().clone();
        let report = failure.into_bevy_error();
        let diagnostic = report.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
        assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
    });
}

#[test]
fn actual_loading_and_color_commands_reach_the_host_with_the_source_identity() {
    let (mut app, _) = fixture();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    WidgetryButtonLoading::set(&mut app.world_mut().commands(), stale, true);
    WidgetryButtonColorOverrides::clear(&mut app.world_mut().commands(), stale);
    assert!(logs.records().is_empty());
    logs.run(|| errors.run(|| app.world_mut().flush()));
    let records = errors.take_records();
    assert_eq!(records.len(), 2);
    for record in records {
        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = record
            .error
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        let source = diagnostic
            .source()
            .unwrap()
            .downcast_ref::<ButtonError>()
            .unwrap();
        assert!(matches!(source, ButtonError::InvalidTarget { root } if *root == stale));
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
fn missing_managed_outputs_keep_the_actual_query_source_and_leave_other_outputs_unchanged() {
    let (mut app, root) = fixture();
    let before = *app.world().get::<BackgroundColor>(root).unwrap();
    app.world_mut().entity_mut(root).remove::<BorderColor>();
    let colors = bevy_widgetry_theme::WidgetryThemeMode::Dark
        .colors()
        .button
        .pressed;
    let failure =
        bevy_widgetry_button::internal::apply_owned_button_colors(app.world_mut(), root, colors)
            .unwrap_err();
    assert!(
        matches!(failure.kind(), ButtonError::MissingColorOutputs { root: target, missing, .. } if *target == root && missing == &["BorderColor"])
    );
    assert!(
        failure
            .kind()
            .source()
            .unwrap()
            .is::<bevy::ecs::query::QueryEntityError>()
    );
    assert_eq!(*app.world().get::<BackgroundColor>(root).unwrap(), before);
}

#[test]
fn invalid_initial_colors_keep_the_core_origin_through_scene_failure_cleanup() {
    let (mut app, _) = fixture();
    let before = app.world().entities().len();
    let logs = LogCapture::default();
    logs.run(|| {
        let mut colors = WidgetryButtonColorOverrides::default();
        colors.disabled.foreground = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let failure = bevy_widgetry_core::scene::spawn_scene(
            app.world_mut(),
            bsn! { @WidgetryButton { @colors: colors } },
        )
        .unwrap_err();
        let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
        let mut cause: &(dyn Error + 'static) = diagnostic;
        while !cause.is::<ButtonFailure>() {
            cause = cause.source().unwrap();
        }
        let source = cause.downcast_ref::<ButtonFailure>().unwrap();
        assert!(matches!(
            source.kind(),
            ButtonError::Core {
                source: CoreError::InvalidColor { .. }
            }
        ));
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), source.origin()));
        assert_eq!(
            logs.diagnostic_records(source.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(source.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        assert_eq!(app.world().entities().len(), before);
    });
}

#[test]
fn normal_loading_disabled_and_same_value_color_requests_are_silent() {
    let (mut app, root) = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        for loading in [false, true, true, false, false] {
            WidgetryButtonLoading::set_in_world(app.world_mut(), root, loading).unwrap();
            assert!(app.world().get::<InteractionDisabled>(root).is_some());
        }
        assert!(!WidgetryButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
        app.update();
        assert!(logs.records().is_empty());
    });
}
