#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#[allow(dead_code)]
mod common;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn independent_projection_rejections_keep_exact_identities_and_queued_handler_origin() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(2, 2);
    app.set_error_handler(ErrorCapture::handler());
    let row = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .row_id(0)
        .unwrap();
    let before = app
        .world()
        .get::<WidgetryTableViewState>(view)
        .unwrap()
        .ordered_rows()
        .to_vec();
    let update = WidgetryTableViewUpdate {
        rows: vec![row, row],
        ..default()
    };
    let logs = LogCapture::default();
    let first = logs
        .run(|| WidgetryTable::<u32>::update_view(app.world_mut(), view, update.clone()))
        .unwrap_err();
    assert!(
        matches!(first.kind(),TableError::ViewRequest{view:actual,source:actual_source,rows:2,reason:TableViewRejection::DuplicateRow{row:actual_row,index:1}} if (*actual,*actual_source,*actual_row)==(view,source,row))
    );
    let second = logs
        .run(|| WidgetryTable::<u32>::update_view(app.world_mut(), view, update.clone()))
        .unwrap_err();
    assert_ne!(first.origin().id(), second.origin().id());
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(view)
            .unwrap()
            .ordered_rows(),
        before
    );
    let errors = ErrorCapture::default();
    logs.run(|| {
        errors.run(|| {
            WidgetryTable::<u32>::queue_update_view(&mut app.world_mut().commands(), view, update);
            app.world_mut().flush();
        })
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    let record = &records[0];
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
    assert_ne!(first.origin().id(), diagnostic.origin().id());
}

#[test]
fn live_layout_faults_keep_nan_bits_source_and_persistent_origin_until_repair() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(2, 2);
    app.set_error_handler(ErrorCapture::handler());
    for label in [PreUpdate.intern(), PostUpdate.intern()] {
        app.edit_schedule(label, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
    }
    let invalid = f32::from_bits(0x7fc0_0012);
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(view)
        .unwrap()
        .row_height = invalid;
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    let mut origin = None;
    logs.run(||for _ in 0..3 {
        errors.run(||app.update());let records=errors.take_records();assert!(!records.is_empty());
        for record in records {
            assert_eq!(record.error.severity(),bevy::ecs::error::Severity::Error);
            let diagnostic=record.managed().unwrap().diagnostics().next().unwrap();
            if let Some(old)=&origin {assert!(std::sync::Arc::ptr_eq(old,diagnostic.origin()));}else{origin=Some(diagnostic.origin().clone());}
            let mut cause=diagnostic.source();let mut found=false;
            while let Some(error)=cause {
                if let Some(TableError::Layout{fault})=error.downcast_ref::<TableError>() {
                    assert!(matches!(fault.as_ref(),TableLayoutFault::Dimension{field:TableLayoutField::RowHeight,value,..} if value.to_bits()==invalid.to_bits()));found=true;
                }
                cause=error.source();
            }assert!(found);
            let fields=diagnostic.contexts().iter().flat_map(|frame|frame.fields()).collect::<Vec<_>>();
            assert!(fields.iter().any(|(key,value)|key=="view"&&value==&view.to_string()));
            assert!(fields.iter().any(|(key,value)|key=="source"&&value.contains(&format!("{source:?}"))));
            assert_eq!(logs.diagnostic_records(diagnostic.origin().id(),DiagnosticPhase::Source).len(),1);
        }
    });
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(view)
        .unwrap()
        .row_height = 28.;
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take().is_empty());
    assert!(logs.records().iter().any(|record| {
        record.matches_field("error_id", &origin.as_ref().unwrap().id().to_string())
            && record.matches_field("phase", "recovered")
    }));
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(view)
        .unwrap()
        .row_height = invalid;
    logs.run(|| errors.run(|| app.update()));
    let records = errors.take_records();
    assert!(!records.is_empty());
    assert_ne!(
        records[0]
            .managed()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap()
            .origin()
            .id(),
        origin.unwrap().id()
    );
}

#[test]
fn stale_schema_and_invalid_width_requests_keep_exact_values_without_committing() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(2, 2);
    let column = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(1)
        .unwrap();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_column(1);
    let failure = WidgetryTable::<u32>::update_view(
        app.world_mut(),
        view,
        WidgetryTableViewUpdate {
            sort: Some(WidgetryTableSort {
                column,
                direction: WidgetryTableSortDirection::Descending,
            }),
            ..default()
        },
    )
    .unwrap_err();
    assert!(
        matches!(failure.kind(),TableError::ViewRequest{view:actual,source:actual_source,reason:TableViewRejection::UnknownSortColumn{column:actual_column,direction:WidgetryTableSortDirection::Descending},..} if (*actual,*actual_source,*actual_column)==(view,source,column))
    );
    let failure = WidgetryTable::<u32>::update_view(
        app.world_mut(),
        view,
        WidgetryTableViewUpdate {
            filters: [column].into(),
            ..default()
        },
    )
    .unwrap_err();
    assert!(
        matches!(failure.kind(),TableError::ViewRequest{reason:TableViewRejection::UnknownFilterColumn{column:actual},..} if *actual==column)
    );
    let selection = WidgetryTableSelection::Column(column);
    let failure =
        WidgetryTable::<u32>::set_selection(app.world_mut(), view, selection).unwrap_err();
    assert!(
        matches!(failure.kind(),TableError::ViewRequest{reason:TableViewRejection::Selection{selection:actual},..} if *actual==selection)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(view)
            .unwrap()
            .selection(),
        WidgetryTableSelection::None
    );
    for width in [0., f32::from_bits(0x7fc0_0013)] {
        let failure = WidgetryTable::<u32>::set_column_width(app.world_mut(), view, column, width)
            .unwrap_err();
        assert!(
            matches!(failure.kind(),TableError::ColumnWidth{view:actual,source:Some(actual_source),column:actual_column,width:actual_width} if (*actual,*actual_source,*actual_column)==(view,source,column)&&actual_width.to_bits()==width.to_bits())
        );
    }
}
