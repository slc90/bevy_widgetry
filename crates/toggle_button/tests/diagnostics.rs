#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::{color::WidgetryStyleOwner, diagnostics::CoreError};
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use bevy_widgetry_toggle_button::{
    ToggleButtonError, ToggleButtonFailure, WidgetryToggleButton,
    WidgetryToggleButtonColorOverrides, WidgetryToggleButtonPlugin,
};
use std::error::Error;

fn fixture() -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryToggleButtonPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryToggleButton })
        .unwrap()
        .id();
    (app, root)
}

#[test]
fn unchecked_and_equal_requests_are_silent_while_invalid_targets_keep_the_operation() {
    let (mut app, root) = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(!WidgetryToggleButton::selected(app.world(), root).unwrap());
        assert!(!WidgetryToggleButton::set_selected_in_world(app.world_mut(), root, false).unwrap());
        assert!(!WidgetryToggleButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
        assert!(logs.records().is_empty());
        let wrong = app.world_mut().spawn_empty().id();
        for stale in [false, true] {
            if stale { app.world_mut().despawn(wrong); }
            let failure: ToggleButtonFailure = WidgetryToggleButton::set_selected_in_world(app.world_mut(), wrong, true).unwrap_err();
            assert!(matches!(failure.kind(), ToggleButtonError::InvalidTarget { root: target } if *target == wrong));
            assert!(failure.contexts().iter().any(|context| context.operation() == "toggle_button.set_selected"));
            assert_eq!(logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source).len(), 1);
        }
    });
}

#[test]
fn core_color_and_managed_owner_failures_keep_their_original_diagnostics() {
    let (mut app, root) = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        let mut colors = WidgetryToggleButtonColorOverrides::default();
        colors.selected_disabled.foreground = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let failure = WidgetryToggleButtonColorOverrides::set_in_world(app.world_mut(), root, colors).unwrap_err();
        assert!(matches!(failure.kind(), ToggleButtonError::Core { source: CoreError::InvalidColor { .. } }));
        assert!(failure.kind().source().unwrap().is::<CoreError>());
        let context = format!("{:?}", failure.contexts());
        assert!(context.contains("selected_disabled") && context.contains("foreground"));
        assert_eq!(*WidgetryToggleButtonColorOverrides::get(app.world(), root).unwrap(), WidgetryToggleButtonColorOverrides::default());
        let origin = failure.origin().clone();
        let report = failure.into_bevy_error();
        let diagnostic = report.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
        assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
        let owner = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(root).insert(WidgetryStyleOwner::<WidgetryToggleButton>::new(owner));
        let failure = WidgetryToggleButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap_err();
        assert!(matches!(failure.kind(), ToggleButtonError::ManagedColors { root: target, owner: actual } if *target == root && *actual == owner));
    });
}

#[test]
fn queued_state_and_color_failures_reach_the_host_with_source_identity() {
    let (mut app, _) = fixture();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    WidgetryToggleButton::set_selected(&mut app.world_mut().commands(), stale, true);
    WidgetryToggleButtonColorOverrides::clear(&mut app.world_mut().commands(), stale);
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
        assert!(
            matches!(diagnostic.source().unwrap().downcast_ref::<ToggleButtonError>(), Some(ToggleButtonError::InvalidTarget { root }) if *root == stale)
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
fn broken_style_owners_are_batched_persist_until_repair_and_recover_without_a_state_change() {
    let (mut app, a) = fixture();
    let b = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryToggleButton })
        .unwrap()
        .id();
    app.update();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app.world_mut().entity_mut(a).remove::<BackgroundColor>();
    app.world_mut().entity_mut(b).remove::<BorderColor>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| {
        app.update();
        app.update();
        let records = errors.take_records();
        assert_eq!(records.len(), 2);
        let ids: Vec<Vec<_>> = records.iter().map(|record| {
            assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
            let diagnostics: Vec<_> = record.error.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().collect();
            assert_eq!(diagnostics.len(), 2);
            diagnostics.iter().map(|diagnostic| {
                let source = diagnostic.source().unwrap().downcast_ref::<ToggleButtonError>().unwrap();
                assert!(matches!(source, ToggleButtonError::MissingStyle { root, .. } if [a,b].contains(root)));
                assert!(source.source().unwrap().is::<bevy::ecs::query::QueryEntityError>());
                assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
                diagnostic.origin().id().clone()
            }).collect()
        }).collect();
        assert_eq!(ids[0], ids[1]);
        assert_ne!(ids[0][0], ids[0][1]);
        app.world_mut().entity_mut(a).insert(BackgroundColor(Color::NONE));
        app.world_mut().entity_mut(b).insert(BorderColor::all(Color::NONE));
        app.update();
        app.update();
        assert!(errors.take_records().is_empty());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
        assert_eq!(app.world().get::<BackgroundColor>(a).unwrap().0, bevy_widgetry_theme::WIDGETRY_DARK_THEME.button.normal.background);
    }));
}

#[test]
fn invalid_initial_colors_preserve_core_identity_through_scene_cleanup() {
    let (mut app, _) = fixture();
    let count = app.world().entities().len();
    let logs = LogCapture::default();
    logs.run(|| {
        let mut colors = WidgetryToggleButtonColorOverrides::default();
        colors.selected_pressed.border = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let error = bevy_widgetry_core::scene::spawn_scene(
            app.world_mut(),
            bsn! {
                @WidgetryToggleButton { @colors: {colors.clone()} }
            },
        )
        .unwrap_err();
        let diagnostic = error.diagnostics().diagnostics().next().unwrap();
        let mut source: &(dyn Error + 'static) = diagnostic;
        while !source.is::<ToggleButtonFailure>() {
            source = source.source().unwrap();
        }
        let failure = source.downcast_ref::<ToggleButtonFailure>().unwrap();
        assert!(matches!(
            failure.kind(),
            ToggleButtonError::Core {
                source: CoreError::InvalidColor { .. }
            }
        ));
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
