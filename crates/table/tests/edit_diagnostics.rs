#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#[allow(dead_code)]
mod common;
use bevy::prelude::*;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn editable(app: &mut App, source: Entity, view: Entity) -> WidgetryTableCell {
    app.world_mut()
        .entity_mut(view)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    }
}

#[test]
fn simultaneous_editor_read_failures_all_reach_the_host_without_creating_pending_requests() {
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy_widgetry_test_utils::ErrorCapture;
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(1, 1);
    let other = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTable::<u32> { @source: source }
            Node { width: px(286), height: px(144) }
        })
        .unwrap()
        .id();
    app.update();
    app.update();
    app.register_table_cell_editor(WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::SingleLine,
        |_: &String| bsn_list! { Text("draft") },
        |_, _| {
            Err(WidgetryTableEditorReadError::external(
                std::io::Error::other("read failed"),
            ))
        },
    ));
    for view in [view, other] {
        let cell = editable(&mut app, source, view);
        WidgetryTable::<u32>::begin_edit(app.world_mut(), view, cell).unwrap();
        app.world_mut()
            .trigger(WidgetryTableSubmitEdit { entity: view });
    }
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| app.update()));
    let records = errors.take_records();
    assert_eq!(records.len(), 2);
    let mut views = std::collections::HashSet::new();
    for record in records {
        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        let frame = diagnostic
            .contexts()
            .iter()
            .find(|frame| frame.operation() == "table.read_editor")
            .unwrap();
        views.insert(
            frame
                .fields()
                .iter()
                .find(|(name, _)| name == "view")
                .unwrap()
                .1
                .clone(),
        );
        let mut cause = diagnostic.source();
        let mut found = false;
        while let Some(error) = cause {
            found |= error.downcast_ref::<std::io::Error>().is_some();
            cause = error.source();
        }
        assert!(found);
    }
    assert_eq!(
        views,
        [view.to_string(), other.to_string()].into_iter().collect()
    );
    for view in [view, other] {
        let state = app.world().get::<WidgetryTableEditState>(view).unwrap();
        assert!(state.pending().is_none());
        assert!(state.error().is_none());
    }
}

#[test]
fn invalid_cancel_request_is_a_single_managed_failure_with_view_and_stage() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, _, _) = common::fixture(1, 1);
    let view = app.world_mut().spawn_empty().id();
    let logs = LogCapture::default();
    let error = logs
        .run(|| WidgetryTable::<u32>::cancel_edit(app.world_mut(), view))
        .unwrap_err();
    let diagnostic = error.diagnostics().next().unwrap();
    let fields = diagnostic
        .contexts()
        .iter()
        .flat_map(|frame| frame.fields())
        .collect::<Vec<_>>();
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "view" && value == &view.to_string())
    );
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "stage" && value == "Cancel")
    );
    assert_eq!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
}

#[test]
fn failed_editor_scenes_keep_native_strategy_and_preserve_existing_business_entities() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(1, 1);
    let cell = editable(&mut app, source, view);
    let business = app.world_mut().spawn(Name::new("business")).id();
    let mut ids = std::collections::HashSet::new();
    for panic in [false, true] {
        app.register_table_cell_editor(WidgetryTableCellEditor::new(
            WidgetryTableEditorMode::SingleLine,
            move |_: &String| bsn_list! {
                template(move |ctx| {
                    let container = ctx.entity.id();
                    ctx.entity.world_scope(|world| {
                        world.entity_mut(business).insert((Name::new("committed"), ChildOf(container)));
                    });
                    Ok(Name::new("editor attempt"))
                })
                Children [template(move |_| -> Result<Name, BevyError> {
                    let error = std::io::Error::other("editor scene");
                    Err(if panic { BevyError::panic(error, Box::new(31u32)) } else { BevyError::error(error).with_payload(Box::new(31u32)) })
                })]
            },
            |_, _| Ok(String::new()),
        ));
        let logs = LogCapture::default();
        let failure = logs
            .run(|| WidgetryTable::<u32>::begin_edit(app.world_mut(), view, cell))
            .unwrap_err();
        assert_eq!(
            app.world().get::<Name>(business).unwrap().as_str(),
            "committed"
        );
        assert!(app.world().get::<ChildOf>(business).is_none());
        assert!(app.world().get::<WidgetryTableEditState>(view).is_none());
        assert!(
            !app.world_mut()
                .query::<&Name>()
                .iter(app.world())
                .any(|name| name.as_str() == "editor attempt")
        );
        let diagnostic = failure.diagnostics().next().unwrap();
        assert!(ids.insert(diagnostic.origin().id().clone()));
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        let mut cause = diagnostic.source();
        let mut found = false;
        while let Some(error) = cause {
            if let Some(scene) = error.downcast_ref::<bevy_widgetry_core::scene::SceneError>() {
                assert!(
                    scene
                        .native_template_error()
                        .unwrap()
                        .downcast_ref::<std::io::Error>()
                        .is_some()
                );
                found = true;
            }
            cause = error.source();
        }
        assert!(found);
        let mut native = failure.into_bevy_error();
        assert_eq!(
            native.severity(),
            if panic {
                bevy::ecs::error::Severity::Panic
            } else {
                bevy::ecs::error::Severity::Error
            }
        );
        assert_eq!(
            *native.take_payload().unwrap().downcast::<u32>().unwrap(),
            31
        );
    }
}

#[test]
fn validation_rejection_and_legal_cancel_are_silent_but_old_receipts_are_classified() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(1, 1);
    let cell = editable(&mut app, source, view);
    app.register_table_cell_editor(WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::SingleLine,
        |_: &String| bsn_list! { Text("draft") },
        |_, _| {
            Err(WidgetryTableEditorReadError::Rejected(
                "invalid value".to_owned(),
            ))
        },
    ));
    let logs = LogCapture::default();
    logs.run(|| {
        WidgetryTable::<u32>::begin_edit(app.world_mut(), view, cell).unwrap();
        assert!(!WidgetryTable::<u32>::submit_edit(app.world_mut(), view).unwrap());
        let state = app.world().get::<WidgetryTableEditState>(view).unwrap();
        assert_eq!(state.error(), Some("invalid value"));
        assert!(state.pending().is_none());
        assert!(WidgetryTable::<u32>::cancel_edit(app.world_mut(), view).unwrap());
        assert!(!WidgetryTable::<u32>::cancel_edit(app.world_mut(), view).unwrap());
        assert!(!WidgetryTable::<u32>::submit_edit(app.world_mut(), view).unwrap());
    });
    assert!(logs.records().is_empty());
    app.register_table_cell_editor(WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::SingleLine,
        |_: &String| bsn_list! { Text("draft") },
        |_, _| Ok("valid".to_owned()),
    ));
    WidgetryTable::<u32>::begin_edit(app.world_mut(), view, cell).unwrap();
    WidgetryTable::<u32>::submit_edit(app.world_mut(), view).unwrap();
    let request = app
        .world()
        .get::<WidgetryTableEditState>(view)
        .unwrap()
        .pending()
        .unwrap();
    WidgetryTable::<u32>::resolve_edit(
        app.world_mut(),
        view,
        request,
        WidgetryTableEditResult::Rejected("retry".to_owned()),
    )
    .unwrap();
    WidgetryTable::<u32>::submit_edit(app.world_mut(), view).unwrap();
    let current = app
        .world()
        .get::<WidgetryTableEditState>(view)
        .unwrap()
        .pending();
    let failure = WidgetryTable::<u32>::resolve_edit(
        app.world_mut(),
        view,
        request,
        WidgetryTableEditResult::Accepted,
    )
    .unwrap_err();
    let diagnostic = failure.diagnostics().next().unwrap();
    let mut cause = diagnostic.source();
    let mut found = false;
    while let Some(error) = cause {
        if let Some(TableEditFault::Receipt {
            reason,
            received,
            pending,
        }) = error.downcast_ref::<TableEditFault>()
        {
            assert_eq!(*reason, TableEditReceiptReason::WrongRequest);
            assert_eq!(*received, request);
            assert_eq!(*pending, current);
            found = true;
        }
        cause = error.source();
    }
    assert!(found);
    assert_eq!(
        app.world()
            .get::<WidgetryTableEditState>(view)
            .unwrap()
            .pending(),
        current
    );
}
