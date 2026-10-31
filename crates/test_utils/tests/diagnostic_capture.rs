#![allow(clippy::disallowed_macros)]

use bevy::ecs::change_detection::Tick;
use bevy::ecs::error::{ErrorContext, Severity};
use bevy::log::tracing::instrument::WithSubscriber;
use bevy::log::tracing::{self, Level};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{DiagnosticBatch, into_bevy_error},
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
use std::{io, sync::Arc};

fn source(label: &str) -> WidgetryFailure<io::Error> {
    WidgetryFailure::new(
        io::Error::other("相同错误文案"),
        ErrorFrame::new("测试请求").field("label", label),
    )
}

#[test]
fn capture_scopes_queries_and_explicit_worker_dispatch_preserve_identity() {
    let logs = LogCapture::default();
    let failure = logs.run(|| source("main"));
    let id = failure.origin().id().clone();
    let records = logs.diagnostic_records(&id, DiagnosticPhase::Source);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].level, Level::ERROR);
    assert!(records[0].matches_field("phase", "source"));
    assert!(!records[0].fields.contains_key("backtrace"));
    logs.run(|| failure.record_handled("恢复", "使用旧内容"));
    assert_eq!(
        logs.diagnostic_records(&id, DiagnosticPhase::Handled).len(),
        1
    );
    assert!(
        !logs.diagnostic_records(&id, DiagnosticPhase::Handled)[0]
            .fields
            .contains_key("backtrace")
    );

    let excluded = logs.run(|| {
        std::thread::spawn(|| source("no dispatcher").origin().id().clone())
            .join()
            .unwrap()
    });
    assert!(logs.records_for_error(&excluded).is_empty());
    let dispatch = logs.dispatcher();
    let worker = std::thread::spawn(move || {
        tracing::dispatcher::with_default(&dispatch, || source("worker"))
    })
    .join()
    .unwrap();
    assert_eq!(
        logs.diagnostic_records(worker.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );

    let dispatch = logs.dispatcher();
    let future_id = std::thread::spawn(move || {
        bevy::tasks::block_on(
            async { source("future").origin().id().clone() }.with_subscriber(dispatch),
        )
    })
    .join()
    .unwrap();
    assert_eq!(logs.records_for_error(&future_id).len(), 1);

    let host = LogCapture::with_targets(["host"]);
    host.run(|| {
        tracing::debug!(target: "host", count = 2, "处理完成");
        tracing::error!(target: "unselected", "不应被捕获");
    });
    assert_eq!(host.records().len(), 1);
    assert_eq!(host.records()[0].level, Level::DEBUG);
    assert!(host.records()[0].matches_field("count", "2"));
    logs.run(|| tracing::error!(target: "host", "默认不捕获宿主"));
    assert!(
        logs.records()
            .iter()
            .all(|record| record.target == "bevy_widgetry")
    );

    let barrier = Arc::new(std::sync::Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let capture = LogCapture::default();
                let errors = ErrorCapture::default();
                let failure = capture.run(|| {
                    barrier.wait();
                    source("concurrent")
                });
                let records = capture.records();
                assert_eq!(records.len(), 1);
                assert!(records[0].matches_field("error_id", &failure.origin().id().to_string()));
                let origin = Arc::clone(failure.origin());
                errors.run(|| {
                    barrier.wait();
                    ErrorCapture::handler()(
                        into_bevy_error(failure),
                        ErrorContext::Command {
                            name: "thread".into(),
                        },
                    );
                });
                let captured = errors.take_records();
                assert_eq!(captured.len(), 1);
                assert!(Arc::ptr_eq(
                    captured[0]
                        .managed()
                        .unwrap()
                        .diagnostics()
                        .next()
                        .unwrap()
                        .origin(),
                    &origin
                ));
                origin.id().clone()
            })
        })
        .collect();
    let ids: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_ne!(ids[0], ids[1]);

    let outer = ErrorCapture::default();
    let inner = ErrorCapture::default();
    outer.run(|| {
        inner.run(|| {
            ErrorCapture::handler()(
                BevyError::warning("foreign"),
                ErrorContext::Command {
                    name: "inner".into(),
                },
            )
        });
        ErrorCapture::handler()(
            into_bevy_error(source("outer")),
            ErrorContext::System {
                name: "outer".into(),
                last_run: Tick::new(23),
            },
        );
    });
    let captured = inner.take_records();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].error.severity(), Severity::Warning);
    assert!(captured[0].managed().is_none());
    assert_eq!(captured[0].context.kind(), "command");
    let captured = outer.take_records();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].error.severity(), Severity::Error);
    assert_eq!(captured[0].managed().unwrap().diagnostics().count(), 1);
    assert_eq!(
        captured[0].context,
        ErrorContext::System {
            name: "outer".into(),
            last_run: Tick::new(23)
        }
    );
    assert!(DiagnosticBatch::default().into_bevy_error().is_none());
}
