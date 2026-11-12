use bevy::prelude::*;
use bevy_ecs::error::{ErrorContext, Severity};
use bevy_widgetry::app_logging::{safe_backtrace, safe_diagnostic, safe_location};
use bevy_widgetry::diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{Diagnostic, dispatch_error},
};
use std::{error::Error, fmt, path::PathBuf};
mod probe;
pub(crate) use probe::run as run_probe;

type Source = Box<dyn Error + Send + Sync>;
pub(crate) type GalleryFailure = WidgetryFailure<GalleryError>;

#[derive(Debug)]
pub(crate) enum GalleryError {
    Operation(Source),
    Configuration {
        action: &'static str,
        path: PathBuf,
        source: Source,
    },
}

impl fmt::Display for GalleryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operation(_) => f.write_str("Gallery operation failed"),
            Self::Configuration { action, path, .. } => write!(
                f,
                "Gallery configuration {action} failed, path={}",
                path.display()
            ),
        }
    }
}
impl Error for GalleryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Operation(source) | Self::Configuration { source, .. } => source.as_ref(),
        })
    }
}

#[track_caller]
pub(crate) fn failure(kind: GalleryError, context: ErrorFrame) -> GalleryFailure {
    WidgetryFailure::new_with_source_reporter(kind, context, |failure| {
        error!(target: "widget_gallery", error_id = %failure.origin().id(), phase = "source", error = %safe_diagnostic(failure), "Gallery 操作失败");
    })
}

#[track_caller]
pub(crate) fn error(source: impl Into<Source>) -> BevyError {
    at(source, ErrorFrame::new("gallery.operation"))
}

#[track_caller]
pub(crate) fn at(source: impl Into<Source>, context: ErrorFrame) -> BevyError {
    failure(GalleryError::Operation(source.into()), context).into_bevy_error()
}

#[track_caller]
pub(crate) fn response_error(source: impl Into<Source>) -> bevy_remote::BrpError {
    response_error_at(source, ErrorFrame::new("gallery.remote.operation"))
}

#[track_caller]
pub(crate) fn response_error_at(
    source: impl Into<Source>,
    context: ErrorFrame,
) -> bevy_remote::BrpError {
    let failure = failure(GalleryError::Operation(source.into()), context);
    managed_response_error(bevy_widgetry::job::JobFailure::from(failure))
}

pub(crate) fn managed_response_error(
    error: impl Into<bevy_widgetry::job::JobFailure>,
) -> bevy_remote::BrpError {
    let failure = error.into();
    let response = bevy_remote::BrpError::resource_error(&failure);
    failure.record_handled("终止远程操作并构造失败响应", "调用方可读取失败结果并重试");
    response
}

pub(crate) fn bevy_response_error(error: BevyError) -> bevy_remote::BrpError {
    if error.severity() == Severity::Error
        && error
            .downcast_ref::<bevy_widgetry::diagnostics::bevy::WidgetryEcsError>()
            .is_some()
    {
        return managed_response_error(error);
    }
    let response = bevy_remote::BrpError::resource_error(&error);
    handler(
        error,
        ErrorContext::Command {
            name: "gallery.remote.operation".into(),
        },
    );
    response
}

pub(crate) fn handler(error: BevyError, context: ErrorContext) {
    dispatch_error(
        error,
        context,
        |diagnostic, error, context| {
            report(diagnostic, error, context);
        },
        |error, context| {
            match error.severity() {
                Severity::Ignore => {}
                Severity::Trace | Severity::Debug => {
                    debug!(target: "widget_gallery", phase = "unhandled", native_severity = ?error.severity(), error = %safe_diagnostic(&error), details = %safe_diagnostic(format_args!("{error:?}")), ecs_context = ?context, "Gallery 外来错误未被处理")
                }
                Severity::Info => {
                    info!(target: "widget_gallery", phase = "unhandled", error = %safe_diagnostic(&error), details = %safe_diagnostic(format_args!("{error:?}")), ecs_context = ?context, "Gallery 外来错误未被处理")
                }
                Severity::Warning => {
                    warn!(target: "widget_gallery", phase = "unhandled", error = %safe_diagnostic(&error), details = %safe_diagnostic(format_args!("{error:?}")), ecs_context = ?context, "Gallery 外来错误未被处理")
                }
                Severity::Error | Severity::Panic => {
                    error!(target: "widget_gallery", phase = "unhandled", error = %safe_diagnostic(&error), details = %safe_diagnostic(format_args!("{error:?}")), ecs_context = ?context, "Gallery 外来错误未被处理")
                }
            }
            if error.severity() == Severity::Panic {
                bevy::ecs::error::panic(error, context);
            }
        },
    );
}

fn report(diagnostic: &dyn Diagnostic, error: &BevyError, context: &ErrorContext) {
    // Bevy exposes no context getter. Managed bridges disable its boundary backtrace,
    // so the Display suffix is exactly the managed reason, which must not be replayed.
    let displayed = error.to_string();
    let reason = error
        .downcast_ref::<bevy_widgetry::diagnostics::bevy::WidgetryEcsError>()
        .map(ToString::to_string);
    let bevy_context = reason
        .as_ref()
        .and_then(|reason| displayed.trim_end().strip_suffix(reason))
        .unwrap_or_default()
        .trim();
    error!(target: "widget_gallery", error_id = %diagnostic.origin().id(), phase = "unhandled", ecs_context = ?context,
        bevy_context = %safe_diagnostic(bevy_context),
        origin = %safe_location(diagnostic.origin().location()), backtrace = %safe_backtrace(diagnostic.origin().backtrace()),
        propagation = ?diagnostic.contexts().get(1..).unwrap_or_default(), "Gallery 错误最终未被处理");
}

pub(crate) fn startup(error: BevyError, logging_ready: bool) {
    if let Some(managed) =
        error.downcast_ref::<bevy_widgetry::diagnostics::bevy::WidgetryEcsError>()
    {
        for diagnostic in managed.diagnostics() {
            if logging_ready {
                error!(target: "widget_gallery", error_id = %diagnostic.origin().id(), phase = "unhandled", boundary = "startup",
                    origin = %safe_location(diagnostic.origin().location()), backtrace = %safe_backtrace(diagnostic.origin().backtrace()),
                    propagation = ?diagnostic.contexts().get(1..).unwrap_or_default(), "Gallery 启动终止");
            } else {
                eprintln!(
                    "ERROR phase=unhandled boundary=startup error_id={} error={} origin={} backtrace={} Gallery 启动终止",
                    diagnostic.origin().id(),
                    safe_diagnostic(diagnostic),
                    safe_location(diagnostic.origin().location()),
                    safe_backtrace(diagnostic.origin().backtrace())
                );
            }
        }
    } else if logging_ready {
        error!(target: "widget_gallery", phase = "unhandled", boundary = "startup", error = %safe_diagnostic(&error), details = %safe_diagnostic(format_args!("{error:?}")), "Gallery 启动终止");
    } else {
        eprintln!(
            "ERROR phase=unhandled boundary=startup error={} details={} Gallery 启动终止",
            safe_diagnostic(&error),
            safe_diagnostic(format_args!("{error:?}"))
        );
    }
}

pub(crate) fn logging_shutdown(failure: &bevy_widgetry::app_logging::LoggingFailure) {
    eprintln!(
        "ERROR phase=unhandled boundary=logging_shutdown error_id={} origin={} backtrace={} Gallery 日志保存失败",
        failure.origin().id(),
        safe_location(failure.origin().location()),
        safe_backtrace(failure.origin().backtrace())
    );
}

pub(crate) fn combine(primary: BevyError, secondary: BevyError) -> BevyError {
    use bevy_widgetry::diagnostics::bevy::WidgetryEcsError;
    if primary.severity() == Severity::Error
        && secondary.severity() == Severity::Error
        && let (Some(first), Some(second)) = (
            primary.downcast_ref::<WidgetryEcsError>(),
            secondary.downcast_ref::<WidgetryEcsError>(),
        )
    {
        let mut combined = first.clone();
        for diagnostic in second.shared_diagnostics() {
            combined.push_shared(diagnostic);
        }
        return combined.into_bevy_error();
    }
    handler(
        primary,
        ErrorContext::Command {
            name: "gallery.operation.cleanup".into(),
        },
    );
    secondary
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry::diagnostics::bevy::{DiagnosticBatch, WidgetryEcsError};
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
    use std::io;

    #[test]
    fn emergency_startup_and_logging_shutdown_use_safe_views_without_a_subscriber() {
        const CHILD: &str = "WIDGETRY_GALLERY_EMERGENCY_CHILD";
        if std::env::var_os(CHILD).is_some() {
            startup(error(io::Error::other("emergency-managed-reason")), false);
            startup(
                BevyError::new_with_backtrace(
                    Severity::Error,
                    io::Error::other("emergency-native-reason"),
                    std::backtrace::Backtrace::force_capture(),
                ),
                false,
            );
            use bevy_widgetry::app_logging::{
                LoggingError, LoggingFailure, LoggingSource, LoggingStage,
            };
            let failure = LoggingFailure::new(
                LoggingError {
                    stage: LoggingStage::Flush,
                    path: PathBuf::from(r"D:\runtime logs\output.log"),
                    source: LoggingSource::Io(std::sync::Arc::new(io::Error::other(
                        "emergency-flush-reason",
                    ))),
                },
                ErrorFrame::new("gallery.emergency_probe"),
            );
            logging_shutdown(&failure);
            return;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "diagnostics::tests::emergency_startup_and_logging_shutdown_use_safe_views_without_a_subscriber", "--nocapture"])
            .env(CHILD, "1")
            .output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stderr}");
        assert!(stderr.contains("boundary=startup error_id="), "{stderr}");
        assert!(
            stderr.contains("boundary=logging_shutdown error_id="),
            "{stderr}"
        );
        assert!(stderr.contains("emergency-managed-reason"));
        assert!(stderr.contains("emergency-native-reason"));
        assert!(
            stderr.contains("origin=gallery/src/diagnostics.rs:"),
            "{stderr}"
        );
        assert!(!stderr.contains(env!("CARGO_MANIFEST_DIR")), "{stderr}");
        assert!(!stderr.contains(r".cargo\registry\src"), "{stderr}");
        assert!(!stderr.contains(r".rustup\toolchains"), "{stderr}");
    }

    #[test]
    fn native_trace_error_remains_diagnosable_without_a_trace_call() {
        let logs = LogCapture::with_targets(["widget_gallery"]);
        logs.run(|| {
            handler(
                BevyError::trace(io::Error::other("native trace reason")),
                ErrorContext::Command {
                    name: "probe.native.trace".into(),
                },
            );
        });
        let records = logs.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level, bevy::log::Level::DEBUG);
        assert_eq!(records[0].fields["native_severity"], "Trace");
        assert!(records[0].fields["error"].contains("native trace reason"));
        assert!(records[0].fields.contains_key("ecs_context"));
    }

    #[test]
    fn native_error_is_reported_and_native_panic_resumes_its_original_payload() {
        let logs = LogCapture::with_targets(["widget_gallery"]);
        let context = ErrorContext::Command {
            name: "probe.native".into(),
        };
        logs.run(|| {
            handler(
                BevyError::error(io::Error::other("native reason")),
                context.clone(),
            )
        });
        let reports = logs.records();
        assert_eq!(reports.len(), 1);
        assert!(reports[0].fields["error"].contains("native reason"));
        assert!(reports[0].fields.contains_key("ecs_context"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            logs.run(|| {
                handler(
                    BevyError::panic("native panic", Box::new(String::from("original payload"))),
                    context,
                )
            });
        }));
        assert_eq!(
            *result.unwrap_err().downcast::<String>().unwrap(),
            "original payload"
        );
    }

    #[test]
    fn managed_batch_reports_each_original_stack_once_without_replaying_the_reason() {
        let logs = LogCapture::with_targets(["widget_gallery"]);
        let (origins, error) = logs.run(|| {
            let mut batch = DiagnosticBatch::default();
            let mut origins = Vec::new();
            for _ in 0..2 {
                let failure = failure(
                    GalleryError::Operation(io::Error::other("distinctive reason").into()),
                    ErrorFrame::new("probe.source"),
                );
                origins.push(failure.origin().clone());
                batch.push(failure);
            }
            (origins, batch.into_bevy_error().unwrap())
        });
        let shared = error.downcast_ref::<WidgetryEcsError>().unwrap().clone();
        let context = ErrorContext::Command {
            name: "gallery.probe".into(),
        };
        logs.run(|| {
            handler(error, context.clone());
            handler(shared.into_bevy_error(), context);
        });
        for origin in origins {
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            let reports = logs.diagnostic_records(origin.id(), DiagnosticPhase::Unhandled);
            assert_eq!(reports.len(), 1);
            assert!(reports[0].fields.contains_key("backtrace"));
            assert!(reports[0].fields.contains_key("ecs_context"));
            assert!(!reports[0].fields.contains_key("error"));
            assert!(!reports[0].fields.contains_key("details"));
        }
    }

    #[test]
    fn operation_keeps_the_typed_source_and_source_location() {
        let logs = LogCapture::with_targets(["widget_gallery"]);
        let line = line!() + 1;
        let error = logs.run(|| error(io::Error::from(io::ErrorKind::PermissionDenied)));
        let diagnostic = error
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert_eq!(error.severity(), Severity::Error);
        assert_eq!(diagnostic.origin().location().line(), line);
        assert!(
            diagnostic
                .source()
                .unwrap()
                .source()
                .unwrap()
                .is::<io::Error>()
        );
        assert_eq!(
            diagnostic.origin().backtrace().status(),
            std::backtrace::BacktraceStatus::Captured
        );
    }

    #[test]
    fn handler_is_inherited_by_subapps_without_overwriting_an_independent_handler() {
        use bevy::app::{AppLabel, SubApp};
        #[derive(AppLabel, Clone, Debug, PartialEq, Eq, Hash)]
        struct Child;
        #[derive(AppLabel, Clone, Debug, PartialEq, Eq, Hash)]
        struct Independent;
        let mut app = App::new();
        let mut independent = SubApp::new();
        independent
            .world_mut()
            .insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::ignore,
            ));
        app.insert_sub_app(Independent, independent);
        app.set_error_handler(handler);
        app.insert_sub_app(Child, SubApp::new());
        assert!(std::ptr::fn_addr_eq(
            app.world()
                .resource::<bevy::ecs::error::FallbackErrorHandler>()
                .0,
            handler as bevy::ecs::error::ErrorHandler
        ));
        assert!(std::ptr::fn_addr_eq(
            app.sub_app(Child)
                .world()
                .resource::<bevy::ecs::error::FallbackErrorHandler>()
                .0,
            handler as bevy::ecs::error::ErrorHandler
        ));
        assert!(std::ptr::fn_addr_eq(
            app.sub_app(Independent)
                .world()
                .resource::<bevy::ecs::error::FallbackErrorHandler>()
                .0,
            bevy::ecs::error::ignore as bevy::ecs::error::ErrorHandler
        ));
    }
}
