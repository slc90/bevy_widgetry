#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_progress_bar::WidgetryProgressBarColorOverrides;
use bevy_widgetry_progress_bar::{ProgressBarError, WidgetryProgressBar, WidgetryProgressState};
use bevy_widgetry_test_utils::ErrorCapture;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};

#[test]
fn height_templates_and_commands_reach_host_with_configuration() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let before = app.world().entities().len();
    logs.run(|| {
        errors.run(|| {
            for height in [f32::NAN, f32::NEG_INFINITY, f32::INFINITY, 0.0, -1.0] {
                app.world_mut().commands().spawn_scene_with_error_handler(
                    bsn! { @WidgetryProgressBar { @height: height } },
                );
            }
            app.world_mut().flush();
        })
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 5);
    for record in records {
        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            diagnostic
                .contexts()
                .iter()
                .any(|frame| frame.operation() == "progress_bar.initialize"
                    && frame.fields().iter().any(|(name, _)| name == "failure"))
        );
    }
    assert_eq!(app.world().entities().len(), before);
}

#[test]
fn colors_keep_core_source_and_targets_distinguish_missing_state() {
    use std::error::Error;
    let mut app = bevy_widgetry_test_utils::scene_app();
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetryProgressBar }).unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        let error = WidgetryProgressBarColorOverrides::set_in_world(
            app.world_mut(),
            root,
            WidgetryProgressBarColorOverrides {
                fill: Some(Color::srgb(f32::NAN, 0.0, 0.0)),
                ..default()
            },
        )
        .unwrap_err();
        assert!(matches!(error.kind(), ProgressBarError::Core { .. }));
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
        assert!(error.contexts().iter().any(|frame| {
            frame
                .fields()
                .iter()
                .any(|(name, value)| name == "field" && value == "fill")
        }));
    });
    let bare = app.world_mut().spawn_empty().id();
    let incomplete = app.world_mut().spawn(WidgetryProgressBar).id();
    assert!(
        matches!(WidgetryProgressBar::state(app.world(), bare).unwrap_err().kind(), ProgressBarError::InvalidRoot { root } if *root == bare)
    );
    assert!(
        matches!(WidgetryProgressBar::state(app.world(), incomplete).unwrap_err().kind(), ProgressBarError::MissingState { root, component: "ProgressState" } if *root == incomplete)
    );
    let logs = LogCapture::default();
    logs.run(|| {
        WidgetryProgressBar::set_state_in_world(
            app.world_mut(),
            root,
            WidgetryProgressState::Determinate { value: 1.0 },
        )
        .unwrap();
        assert!(
            !WidgetryProgressBar::set_state_in_world(
                app.world_mut(),
                root,
                WidgetryProgressState::Determinate { value: 1.0 }
            )
            .unwrap()
        );
        WidgetryProgressBar::set_state_in_world(
            app.world_mut(),
            root,
            WidgetryProgressState::Indeterminate,
        )
        .unwrap();
        assert!(
            !WidgetryProgressBar::set_state_in_world(
                app.world_mut(),
                root,
                WidgetryProgressState::Indeterminate
            )
            .unwrap()
        );
    });
    assert!(logs.records().is_empty());
}

#[test]
fn rejected_values_keep_request_and_origin_at_boundary() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetryProgressBar }).unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        for value in [f32::NAN, f32::NEG_INFINITY, f32::INFINITY, -0.1, 1.1] {
            let error = WidgetryProgressBar::set_state_in_world(
                app.world_mut(),
                root,
                WidgetryProgressState::Determinate { value },
            )
            .unwrap_err();
            let ProgressBarError::InvalidValue { value: actual } = error.kind() else {
                panic!("unexpected error: {error}");
            };
            assert_eq!(actual.to_bits(), value.to_bits());
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
        }
    });
    assert!(
        logs.records()
            .iter()
            .all(|record| !record.fields.contains_key("backtrace"))
    );
    assert_eq!(
        WidgetryProgressBar::state(app.world(), root).unwrap(),
        WidgetryProgressState::Determinate { value: 0.0 }
    );
}

#[test]
fn broken_visuals_batch_roots_reuse_origin_and_recover() {
    use bevy_widgetry_progress_bar::WidgetryProgressBarPlugin;

    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryProgressBarPlugin);
    app.insert_resource(bevy_widgetry_theme::WIDGETRY_LIGHT_THEME);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let mut tracks = Vec::new();
    for _ in 0..2 {
        let root = spawn_scene(app.world_mut(), bsn! { @WidgetryProgressBar }).unwrap();
        let track = app.world().get::<Children>(root).unwrap()[0];
        app.world_mut()
            .entity_mut(track)
            .remove::<BackgroundColor>();
        tracks.push(track);
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| app.world_mut().run_schedule(PostUpdate)));
    let first = errors.take_records();
    assert_eq!(first.len(), 1);
    let origins: Vec<_> = first[0]
        .managed()
        .unwrap()
        .diagnostics()
        .map(|diagnostic| diagnostic.origin().clone())
        .collect();
    assert_eq!(origins.len(), 2);
    app.insert_resource(bevy_widgetry_theme::WIDGETRY_DARK_THEME);
    logs.run(|| errors.run(|| app.world_mut().run_schedule(PostUpdate)));
    let repeated = errors.take_records();
    assert_eq!(repeated.len(), 1);
    for (diagnostic, origin) in repeated[0].managed().unwrap().diagnostics().zip(&origins) {
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), origin));
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
    for track in &tracks {
        app.world_mut()
            .entity_mut(*track)
            .insert(BackgroundColor(Color::NONE));
    }
    app.insert_resource(bevy_widgetry_theme::WIDGETRY_LIGHT_THEME);
    logs.run(|| errors.run(|| app.world_mut().run_schedule(PostUpdate)));
    assert!(errors.take().is_empty());
    for origin in &origins {
        assert_eq!(
            logs.records_for_error(origin.id())
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            1
        );
    }
    app.world_mut()
        .entity_mut(tracks[0])
        .remove::<BackgroundColor>();
    app.insert_resource(bevy_widgetry_theme::WIDGETRY_DARK_THEME);
    logs.run(|| errors.run(|| app.world_mut().run_schedule(PostUpdate)));
    let new = errors.take_records();
    let origin = new[0]
        .managed()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap()
        .origin();
    assert!(
        origins
            .iter()
            .all(|old| !std::sync::Arc::ptr_eq(old, origin))
    );
}
