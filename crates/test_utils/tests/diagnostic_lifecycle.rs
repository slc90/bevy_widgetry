#![allow(clippy::disallowed_macros)]

use bevy::{
    ecs::error::Severity,
    log::{
        tracing,
        tracing_subscriber::{self, prelude::*},
    },
    prelude::*,
};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{DiagnosticBatch, dispatch_error},
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::{backtrace::BacktraceStatus, io, sync::Arc};

#[inline(never)]
fn source() -> WidgetryFailure<io::Error> {
    WidgetryFailure::new(
        io::Error::other("独立请求失败"),
        ErrorFrame::new("子进程源头"),
    )
}

#[test]
fn global_subscriber_observes_original_diagnostics_through_real_command_fixture() {
    const CHILD: &str = "WIDGETRY_TEST_UTILS_DIAGNOSTIC_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "global_subscriber_observes_original_diagnostics_through_real_command_fixture",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env_remove("RUST_BACKTRACE")
            .env_remove("RUST_LIB_BACKTRACE")
            .env_remove("BEVY_BACKTRACE")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    assert!(std::env::var_os("RUST_BACKTRACE").is_none());
    assert!(std::env::var_os("RUST_LIB_BACKTRACE").is_none());
    let logs = LogCapture::with_targets(["bevy_widgetry", "fixture_host"]);
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(logs.clone()))
        .unwrap();

    let handled = source();
    let handled_id = handled.origin().id().clone();
    let released = Arc::downgrade(handled.origin());
    handled.record_handled("取消本次请求", "保留现有内容");
    drop(handled);
    assert!(released.upgrade().is_none());
    assert_eq!(
        logs.diagnostic_records(&handled_id, DiagnosticPhase::Source)
            .len(),
        1
    );
    assert_eq!(
        logs.diagnostic_records(&handled_id, DiagnosticPhase::Handled)
            .len(),
        1
    );
    assert!(
        logs.diagnostic_records(&handled_id, DiagnosticPhase::Unhandled)
            .is_empty()
    );

    let mut batch = DiagnosticBatch::default();
    let mut origins = Vec::new();
    for _ in 0..2 {
        let failure = source();
        origins.push(Arc::clone(failure.origin()));
        batch.push(failure.with_context(ErrorFrame::new("传播到命令")));
    }
    assert_ne!(origins[0].id(), origins[1].id());
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    errors.run(|| {
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result<(), BevyError> {
                match batch.into_bevy_error() {
                    Some(error) => Err(error),
                    None => Ok(()),
                }
            });
        app.world_mut().flush();
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    let record = records.into_iter().next().unwrap();
    assert_eq!(record.error.severity(), Severity::Error);
    assert_eq!(record.context.kind(), "command");
    assert_eq!(record.managed().unwrap().diagnostics().count(), 2);
    for (diagnostic, origin) in record.managed().unwrap().diagnostics().zip(&origins) {
        assert!(Arc::ptr_eq(diagnostic.origin(), origin));
        assert_eq!(origin.backtrace().status(), BacktraceStatus::Captured);
        assert!(
            origin
                .location()
                .file()
                .ends_with("diagnostic_lifecycle.rs")
        );
        assert!(
            diagnostic
                .source()
                .unwrap()
                .downcast_ref::<io::Error>()
                .is_some()
        );
    }
    dispatch_error(
        record.error,
        record.context,
        |diagnostic, _, context| {
            tracing::error!(target: "fixture_host", error_id = %diagnostic.origin().id(), phase = "unhandled", backtrace = %diagnostic.origin().backtrace(), context = %context, "错误最终未处理");
        },
        |_, _| panic!("managed error became foreign"),
    );
    for origin in &origins {
        let source = logs.diagnostic_records(origin.id(), DiagnosticPhase::Source);
        assert_eq!(source.len(), 1);
        assert!(!source[0].fields.contains_key("backtrace"));
        assert!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        let terminal = logs.diagnostic_records(origin.id(), DiagnosticPhase::Unhandled);
        assert_eq!(terminal.len(), 1);
        assert!(terminal[0].fields["backtrace"].contains("diagnostic_lifecycle::source"));
        assert!(!terminal[0].fields.contains_key("error"));
    }
}
