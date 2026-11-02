#![allow(clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

#[test]
fn invalid_scope_is_a_file_dialog_domain_failure() {
    let error = WidgetryFileDialogState::new(WidgetryFileDialogProps {
        storage_scope: Some(String::new()),
        ..Default::default()
    })
    .err()
    .unwrap();
    assert!(
        matches!(error.kind(), FileDialogError::ScopeLength { scope, bytes: 0 } if scope.is_empty())
    );
}

#[test]
fn filter_configuration_failures_preserve_ids_values_and_capacity() {
    let filter = WidgetryFileDialogFilter {
        id: WidgetryFileDialogFilterId("text".into()),
        label: "Text".into(),
        suffixes: vec![".txt".into()],
        default_extension: None,
    };
    let error = WidgetryFileDialogState::new(WidgetryFileDialogProps {
        filters: vec![filter.clone()],
        ..Default::default()
    })
    .err()
    .unwrap();
    assert!(
        matches!(error.kind(), FileDialogError::Filter { filter: actual, violation: FileDialogFilterViolation::InvalidSuffix, suffix: Some(suffix) } if **actual == filter && suffix == ".txt")
    );
    let mut valid = filter;
    valid.suffixes = vec!["txt".into()];
    let duplicate = WidgetryFileDialogState::new(WidgetryFileDialogProps {
        filters: vec![valid.clone(), valid.clone()],
        ..Default::default()
    })
    .err()
    .unwrap();
    assert!(matches!(duplicate.kind(), FileDialogError::DuplicateFilter { id } if *id == valid.id));
    let capacity = WidgetryFileDialogState::new(WidgetryFileDialogProps {
        filters: vec![valid; 129],
        ..Default::default()
    })
    .err()
    .unwrap();
    assert!(matches!(
        capacity.kind(),
        FileDialogError::Capacity {
            resource: "filters",
            actual: 129,
            limit: 128
        }
    ));
}

#[test]
fn invalid_scene_retains_origin_and_cleans_only_its_new_root() {
    let mut app = scene_app();
    let sentinel = app.world_mut().spawn(Name::new("sentinel")).id();
    let before = app.world().entities().len();
    let logs = LogCapture::default();
    let error = logs
        .run(|| {
            bevy_widgetry_core::scene::spawn_scene(
                app.world_mut(),
                bsn! { @WidgetryFileDialog { @storage_scope: {Some(String::new())} } },
            )
        })
        .err()
        .unwrap();
    assert_eq!(app.world().entities().len(), before);
    assert!(app.world().get_entity(sentinel).is_ok());
    let diagnostics: Vec<_> = error.diagnostics().diagnostics().collect();
    assert_eq!(diagnostics.len(), 1);
    let diagnostic = diagnostics[0];
    assert_eq!(
        diagnostic.origin().backtrace().status(),
        std::backtrace::BacktraceStatus::Captured
    );
    assert!(diagnostic.origin().location().file().ends_with("model.rs"));
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

#[test]
fn queued_invalid_request_preserves_domain_fields_at_real_handler() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app.world_mut().spawn_empty().id();
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    WidgetryFileDialog::queue(
        &mut app.world_mut().commands(),
        root,
        WidgetryFileDialogAction::Cancel,
    );
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
    assert!(
        matches!(diagnostic.source().unwrap().downcast_ref::<FileDialogError>(), Some(FileDialogError::Api { root: Some(actual), action: Some(action), fault: FileDialogApiFault::InvalidRoot, .. }) if *actual == root && **action == WidgetryFileDialogAction::Cancel)
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

#[test]
fn job_failure_stays_distinct_through_operation_and_command_boundaries() {
    let logs = LogCapture::default();
    logs.run(|| {
        let failure = bevy_widgetry_job::JobManager::with_buffer_capacity(0)
            .err()
            .unwrap();
        let origin = failure.origin().clone();
        let error = FileDialogOperationFailure::Job(failure.into());
        let mut app = App::new();
        app.set_error_handler(ErrorCapture::handler());
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result { Err(error.into()) });
        let errors = ErrorCapture::default();
        errors.run(|| app.world_mut().flush());
        let records = errors.take_records();
        let managed = records[0].error.downcast_ref::<WidgetryEcsError>().unwrap();
        let diagnostic = managed.diagnostics().next().unwrap();
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
        assert!(
            diagnostic
                .source()
                .unwrap()
                .is::<bevy_widgetry_job::SubmitError>()
        );
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    });
}

#[test]
fn storage_failures_keep_scope_revision_and_path_without_mutation() {
    let mut storage = WidgetryFileDialogStorage::default();
    storage.import("scope".into(), Default::default()).unwrap();
    let revision = storage.revision("scope").unwrap();
    let failure = storage
        .acknowledge_saved("scope", revision + 1)
        .unwrap_err();
    assert!(
        matches!(failure.kind(), FileDialogError::StorageAcknowledgement { scope, requested, current } if scope == "scope" && *requested == revision + 1 && *current == revision)
    );
    assert_eq!(storage.saved_revision("scope"), Some(0));
    let path = std::path::PathBuf::from("relative");
    let failure = storage
        .import(
            "scope".into(),
            WidgetryFileDialogStorageSnapshot {
                pinned: vec![path.clone()],
                ..Default::default()
            },
        )
        .unwrap_err();
    assert!(
        matches!(failure.kind(), FileDialogError::PinnedPath { path: actual, duplicate: false } if actual == &path)
    );
    assert_eq!(storage.revision("scope"), Some(revision));
}

#[test]
fn stale_normal_replies_destroyed_targets_and_noops_remain_quiet() {
    let mut app = scene_app();
    app.insert_resource(WidgetryFileDialogRuntimeOptions {
        automatic: false,
        ..Default::default()
    })
    .add_plugins(WidgetryFileDialogHeadlessPlugin);
    let root =
        bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! { @WidgetryFileDialog })
            .unwrap();
    let old = WidgetryFileDialogState::new(Default::default())
        .unwrap()
        .token();
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(
            !WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::ClearSelection
            )
            .unwrap()
        );
        assert!(
            !WidgetryFileDialog::deliver(
                app.world_mut(),
                root,
                WidgetryFileDialogReply::Started {
                    token: old,
                    path: "C:/obsolete".into()
                }
            )
            .unwrap()
        );
        assert!(
            WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)
                .unwrap()
        );
        assert!(
            !WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)
                .unwrap()
        );
        app.world_mut().despawn(root);
        assert!(
            !WidgetryFileDialog::deliver(
                app.world_mut(),
                root,
                WidgetryFileDialogReply::Started {
                    token: old,
                    path: "C:/obsolete".into()
                }
            )
            .unwrap()
        );
    });
    assert!(logs.records().is_empty());
}
