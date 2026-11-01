#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::tasks::{AsyncComputeTaskPool, IoTaskPool, TaskPool};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use bevy_widgetry_job::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
use std::{
    error::Error,
    io,
    sync::{Arc, mpsc},
    time::Duration,
};

fn receive(receiver: &mut JobReceiver<()>) -> JobFailure {
    match receiver.recv_blocking().unwrap() {
        JobRead::Finished(JobOutcome::Failed(failure)) => failure,
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn real_execution_backends_share_the_work_origin_with_snapshots_and_two_subscribers() {
    IoTaskPool::get_or_init(TaskPool::new);
    AsyncComputeTaskPool::get_or_init(TaskPool::new);
    for kind in [JobKind::Thread, JobKind::Io, JobKind::Cpu] {
        let manager = JobManager::default();
        let logs = LogCapture::default();
        let dispatcher = logs.dispatcher();
        let (created, original) = mpsc::channel();
        let gate = Arc::new(event_listener::Event::new());
        let release = gate.listen();
        let work = move || {
            let failure = WidgetryFailure::new(
                io::Error::new(io::ErrorKind::PermissionDenied, "source refused"),
                ErrorFrame::new("test.work"),
            );
            created.send(failure.origin().clone()).unwrap();
            Err(JobFailure::from(failure))
        };
        let (id, mut first) = match kind {
            JobKind::Thread => manager.spawn_thread::<(), _>(JobOptions::default(), move |_| {
                bevy::tasks::block_on(release);
                tracing::dispatcher::with_default(&dispatcher, work)
            }),
            kind => {
                let work = move |_| async move {
                    release.await;
                    tracing::dispatcher::with_default(&dispatcher, work)
                };
                match kind {
                    JobKind::Io => manager.spawn_io::<(), _, _>(JobOptions::default(), work),
                    _ => manager.spawn_cpu::<(), _, _>(JobOptions::default(), work),
                }
            }
        }
        .unwrap();
        let mut second = manager.subscribe::<()>(id).unwrap();
        gate.notify(usize::MAX);
        let origin = original.recv_timeout(Duration::from_secs(5)).unwrap();
        let a = receive(&mut first);
        let b = receive(&mut second);
        let JobOutcome::Failed(snapshot) = manager.status(id).unwrap().outcome.unwrap() else {
            panic!("missing failed snapshot");
        };
        for failure in [&a, &b, &snapshot] {
            let diagnostic = failure.diagnostics().next().unwrap();
            assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
            let mut cause: &(dyn Error + 'static) = diagnostic;
            let original = loop {
                if let Some(source) = cause.downcast_ref::<io::Error>() {
                    break source;
                }
                cause = cause.source().unwrap();
            };
            assert_eq!(original.kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(
                diagnostic.origin().backtrace().status(),
                std::backtrace::BacktraceStatus::Captured
            );
        }
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        logs.run(|| a.record_handled("终止测试操作", "失败结果已消费"));
        logs.run(|| b.record_handled("重复消费", "同一结果"));
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .len(),
            1
        );
        assert_eq!(
            snapshot.into_bevy_error().severity(),
            bevy::ecs::error::Severity::Error
        );
    }
}

#[test]
fn combined_failures_keep_both_sources_and_origins_without_new_source_events() {
    let logs = LogCapture::default();
    logs.run(|| {
        let a = WidgetryFailure::new(io::Error::from_raw_os_error(5), ErrorFrame::new("business"));
        let b = WidgetryFailure::new(io::Error::from_raw_os_error(6), ErrorFrame::new("delivery"));
        let origins = [a.origin().clone(), b.origin().clone()];
        let failure = JobFailure::from(a).combine(JobFailure::from(b));
        let diagnostics: Vec<_> = failure.diagnostics().collect();
        assert_eq!(diagnostics.len(), 2);
        for (diagnostic, origin) in diagnostics.iter().zip(&origins) {
            assert!(Arc::ptr_eq(diagnostic.origin(), origin));
            assert!(diagnostic.source().unwrap().is::<io::Error>());
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
        assert_eq!(
            failure.into_bevy_error().severity(),
            bevy::ecs::error::Severity::Error
        );
    });
}

#[test]
fn external_errors_and_caught_panics_have_typed_observation_boundaries() {
    IoTaskPool::get_or_init(TaskPool::new);
    let manager = JobManager::default();
    let (_, mut receiver) = manager
        .spawn_io::<(), _, _>(JobOptions::default(), |_| async {
            Err(JobFailure::external(
                io::Error::from_raw_os_error(5),
                ErrorFrame::new("external.open"),
            ))
        })
        .unwrap();
    let failure = receive(&mut receiver);
    assert!(failure.source().unwrap().source().is_some());
    let (_, mut receiver) = manager
        .spawn_thread::<(), _>(JobOptions::default(), |_| panic!("third party panic"))
        .unwrap();
    let failure = receive(&mut receiver);
    let diagnostic = failure.diagnostics().next().unwrap();
    assert!(
        matches!(diagnostic.source().unwrap().downcast_ref::<JobError>(), Some(JobError::PanicObserved { message }) if message == "third party panic")
    );
    assert!(diagnostic.contexts().iter().any(|frame| {
        frame
            .fields()
            .iter()
            .any(|(name, value)| name == "backtrace_kind" && value == "panic_observation")
    }));
}

#[test]
fn native_bevy_contexts_keep_all_reasons_without_the_display_backtrace() {
    use bevy::ecs::error::{BevyError, ContextExt, Severity};
    let result: Result<(), BevyError> = Err(BevyError::new_with_backtrace(
        Severity::Error,
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "original native cause\n   17: application::context detail",
        ),
        std::backtrace::Backtrace::force_capture(),
    ));
    let error = result
        .context("inner native context")
        .context("outer native context")
        .unwrap_err();
    let logs = LogCapture::default();
    logs.run(|| {
        let failure = JobFailure::from(error);
        let message = failure.to_string();
        for reason in [
            "outer native context",
            "inner native context",
            "original native cause",
            "17: application::context detail",
        ] {
            assert!(message.contains(reason), "{message}");
        }
        assert!(
            !message
                .contains("native_bevy_contexts_keep_all_reasons_without_the_display_backtrace")
        );
        let diagnostic = failure.diagnostics().next().unwrap();
        let Some(JobError::NativeBevy { error, .. }) =
            diagnostic.source().unwrap().downcast_ref::<JobError>()
        else {
            panic!("missing original BevyError");
        };
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::PermissionDenied
        );
        let records = logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source);
        assert_eq!(records.len(), 1);
        assert!(records[0].fields["error"].contains("original native cause"));
    });
}

#[test]
fn native_numbered_reason_text_survives_real_backtraces_and_contexts() {
    use bevy::ecs::error::{BevyError, ContextExt, Severity};
    struct NativeReason;
    impl std::fmt::Display for NativeReason {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("native root\n   17: application::detail\ncontext: [application text]")
        }
    }
    impl std::fmt::Debug for NativeReason {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(self, f)
        }
    }
    impl Error for NativeReason {}
    for captured in [false, true] {
        for contexts in [0, 1, 2] {
            let trace = if captured {
                std::backtrace::Backtrace::force_capture()
            } else {
                std::backtrace::Backtrace::disabled()
            };
            let mut error = BevyError::new_with_backtrace(Severity::Error, NativeReason, trace);
            for context in ["inner context", "outer context"]
                .into_iter()
                .take(contexts)
            {
                error = Err::<(), _>(error).context(context).unwrap_err();
            }
            let logs = LogCapture::default();
            logs.run(|| {
                let failure = JobFailure::from(error);
                let message = failure.to_string();
                for reason in [
                    "native root",
                    "17: application::detail",
                    "context: [application text]",
                ] {
                    assert!(
                        message.contains(reason),
                        "captured={captured}, contexts={contexts}: {message}"
                    );
                }
                for context in ["inner context", "outer context"]
                    .into_iter()
                    .take(contexts)
                {
                    assert!(message.contains(context), "{message}");
                }
                assert!(
                    !message.contains(
                        "native_numbered_reason_text_survives_real_backtraces_and_contexts"
                    ),
                    "{message}"
                );
                let diagnostic = failure.diagnostics().next().unwrap();
                let Some(JobError::NativeBevy { error, .. }) =
                    diagnostic.source().unwrap().downcast_ref::<JobError>()
                else {
                    panic!("missing native object");
                };
                assert!(error.is::<NativeReason>());
                let events =
                    logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source);
                assert_eq!(events.len(), 1);
                assert!(events[0].fields["error"].contains("17: application::detail"));
                assert!(
                    !events[0].fields["error"].contains(
                        "native_numbered_reason_text_survives_real_backtraces_and_contexts"
                    )
                );
            });
        }
    }
}

#[test]
fn managed_bevy_contexts_survive_job_conversion_and_the_real_command_boundary() {
    use bevy::ecs::error::{BevyError, ContextExt, Severity};
    use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
    use bevy_widgetry_test_utils::ErrorCapture;
    let logs = LogCapture::default();
    logs.run(|| {
        for captured in [false, true] {
            let a = WidgetryFailure::new(
                io::Error::from_raw_os_error(5),
                ErrorFrame::new("original.work.a"),
            );
            let b = WidgetryFailure::new(
                io::Error::from_raw_os_error(6),
                ErrorFrame::new("original.work.b"),
            );
            let origins = [a.origin().clone(), b.origin().clone()];
            let mut report = WidgetryEcsError::new(a);
            report.push_shared(Arc::new(b));
            let error = BevyError::new_with_backtrace(
                Severity::Error,
                report,
                if captured {
                    std::backtrace::Backtrace::force_capture()
                } else {
                    std::backtrace::Backtrace::disabled()
                },
            );
            let error = Err::<(), _>(error)
                .context("inner operation\n   17: application::body")
                .with_context(|| "outer file path C:/recording.widx")
                .unwrap_err();
            let failure = JobFailure::from(error);
            let text = failure.to_string();
            assert!(text.contains("inner operation"), "{text}");
            assert!(text.contains("17: application::body"), "{text}");
            assert!(text.contains("outer file path C:/recording.widx"), "{text}");
            assert!(
                !text.contains(
                    "managed_bevy_contexts_survive_job_conversion_and_the_real_command_boundary"
                ),
                "{text}"
            );
            let mut app = bevy::prelude::App::new();
            app.set_error_handler(ErrorCapture::handler());
            let errors = ErrorCapture::default();
            errors.run(|| {
                app.world_mut().commands().queue(
                    move |_: &mut bevy::prelude::World| -> Result<(), BevyError> {
                        Err(failure.into_bevy_error())
                    },
                );
                app.world_mut().flush();
            });
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].error.severity(), Severity::Error);
            let report = records[0].error.downcast_ref::<WidgetryEcsError>().unwrap();
            assert_eq!(report.diagnostics().count(), 2);
            for (diagnostic, origin) in report.diagnostics().zip(&origins) {
                assert!(Arc::ptr_eq(diagnostic.origin(), origin));
                let mut cause: &(dyn Error + 'static) = diagnostic;
                while !cause.is::<io::Error>() {
                    cause = cause.source().unwrap();
                }
                assert!(diagnostic.contexts().iter().any(|frame| {
                    frame
                        .fields()
                        .iter()
                        .any(|(_, value)| value.contains("outer file path C:/recording.widx"))
                }));
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
        }
    });
}
