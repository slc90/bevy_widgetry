#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#[allow(dead_code)]
mod common;
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
use std::{
    fmt,
    sync::{Arc, Mutex},
};
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
struct FormattingFault(u32);
impl fmt::Display for FormattingFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "formatter fault {}", self.0)
    }
}
impl std::error::Error for FormattingFault {}

#[test]
fn managed_formatter_keeps_its_original_origin_through_cell_context() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(1, 1);
    WidgetryTable::<u32>::set_selection(app.world_mut(), view, WidgetryTableSelection::All)
        .unwrap();
    let origin = Arc::new(Mutex::new(None));
    let observed = origin.clone();
    app.register_table_copy_formatter(WidgetryTableCopyFormatter::new(move |_: &String| {
        let failure = WidgetryFailure::new(FormattingFault(17), ErrorFrame::new("business.format"));
        *observed.lock().unwrap() = Some(failure.origin().clone());
        Err(WidgetryTableCopyFormatError::diagnostic(failure))
    }));
    let logs = LogCapture::default();
    let error = logs
        .run(|| WidgetryTable::<u32>::selection_tsv(app.world_mut(), view))
        .unwrap_err();
    let diagnostic = error.diagnostics().next().unwrap();
    assert!(Arc::ptr_eq(
        origin.lock().unwrap().as_ref().unwrap(),
        diagnostic.origin()
    ));
    assert_eq!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
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
            .any(|(key, value)| key == "source" && value.contains(&format!("{source:?}")))
    );
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "row" && value != "None")
    );
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "column" && value != "None")
    );
    let mut cause = diagnostic.source();
    let mut found = false;
    while let Some(error) = cause {
        if let Some(source) = error.downcast_ref::<FormattingFault>() {
            assert_eq!(source.0, 17);
            found = true;
        }
        cause = error.source();
    }
    assert!(found);
    assert!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
            .is_empty()
    );
}

#[test]
fn external_formatter_keeps_its_type_cell_and_independent_request_identity() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(1, 1);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    WidgetryTable::<u32>::set_selection(
        app.world_mut(),
        view,
        WidgetryTableSelection::Cell {
            row: cell.row,
            column: cell.column,
        },
    )
    .unwrap();
    app.world_mut()
        .remove_resource::<bevy::clipboard::Clipboard>();
    app.register_table_copy_formatter(WidgetryTableCopyFormatter::new(|_: &String| {
        Err(WidgetryTableCopyFormatError::external(FormattingFault(29)))
    }));
    let logs = LogCapture::default();
    let mut ids = std::collections::HashSet::new();
    for _ in 0..2 {
        let error = logs
            .run(|| WidgetryTable::<u32>::copy_selection(app.world_mut(), view))
            .unwrap_err();
        let diagnostic = error.diagnostics().next().unwrap();
        assert!(ids.insert(diagnostic.origin().id().clone()));
        let mut cause = diagnostic.source();
        let mut found = false;
        let mut classified = false;
        while let Some(error) = cause {
            if let Some(source) = error.downcast_ref::<FormattingFault>() {
                assert_eq!(source.0, 29);
                found = true;
            }
            if let Some(TableCopyFault::Cell {
                cell: actual,
                reason: TableCopyCellFault::Callback { .. },
            }) = error.downcast_ref::<TableCopyFault>()
            {
                assert_eq!(*actual, cell);
                classified = true;
            }
            cause = error.source();
        }
        assert!(found && classified);
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
fn queued_copy_reports_the_same_managed_callback_origin_at_the_ecs_boundary() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, view, _) = common::fixture(1, 1);
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    WidgetryTable::<u32>::set_selection(app.world_mut(), view, WidgetryTableSelection::All)
        .unwrap();
    let origin = Arc::new(Mutex::new(None));
    let observed = origin.clone();
    app.register_table_copy_formatter(WidgetryTableCopyFormatter::new(move |_: &String| {
        let failure = WidgetryFailure::new(FormattingFault(41), ErrorFrame::new("business.format"));
        *observed.lock().unwrap() = Some(failure.origin().clone());
        Err(WidgetryTableCopyFormatError::diagnostic(failure))
    }));
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            WidgetryTable::<u32>::queue_copy_selection(&mut app.world_mut().commands(), view);
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
    assert!(Arc::ptr_eq(
        origin.lock().unwrap().as_ref().unwrap(),
        diagnostic.origin()
    ));
    assert_eq!(
        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
}
