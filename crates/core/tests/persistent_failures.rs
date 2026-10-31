#![allow(clippy::disallowed_macros)]

use bevy::{ecs::schedule::SingleThreadedExecutor, prelude::*};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_diagnostics::{
    ErrorFrame, ErrorOrigin, WidgetryFailure,
    bevy::{Diagnostic, WidgetryEcsError, dispatch_error},
};
use bevy_widgetry_internal_log::widgetry_error;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::{cell::Cell, io, sync::Arc};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cause {
    Missing,
    Invalid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    request: u64,
    cause: Cause,
}

#[allow(clippy::unwrap_used)]
fn origin(error: &WidgetryEcsError) -> Arc<ErrorOrigin> {
    Arc::clone(error.diagnostics().next().unwrap().origin())
}

fn source() -> Arc<dyn Diagnostic> {
    Arc::new(WidgetryFailure::new(
        io::Error::other("same description"),
        ErrorFrame::new("owner failure"),
    ))
}

#[inline(never)]
fn boundary_error(native: bool, diagnostic: Arc<dyn Diagnostic>) -> BevyError {
    if native {
        BevyError::error(io::Error::other("native control"))
    } else {
        WidgetryEcsError::from_shared(diagnostic).into_bevy_error()
    }
}

#[test]
fn managed_boundary_reuses_origin_without_capturing_a_boundary_stack() {
    if std::env::var_os("WIDGETRY_PERSISTENT_STACK_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .env("WIDGETRY_PERSISTENT_STACK_CHILD", "1")
            .env("RUST_BACKTRACE", "full")
            .arg("--exact")
            .arg("managed_boundary_reuses_origin_without_capturing_a_boundary_stack")
            .arg("--nocapture")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let diagnostic = source();
    let original = Arc::clone(diagnostic.origin());
    assert!(
        boundary_error(true, diagnostic.clone())
            .to_string()
            .contains("persistent_failures::boundary_error")
    );
    let error = boundary_error(false, diagnostic);
    assert!(
        !error
            .to_string()
            .contains("persistent_failures::boundary_error")
    );
    let managed = error
        .downcast_ref::<WidgetryEcsError>()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap();
    assert!(Arc::ptr_eq(managed.origin(), &original));
    assert_eq!(
        managed.origin().backtrace().status(),
        std::backtrace::BacktraceStatus::Captured
    );
}

#[test]
fn explicit_keys_reuse_the_original_diagnostic_without_running_the_factory() {
    let logs = LogCapture::default();
    logs.run(|| {
        let mut state = FailureState::default();
        let made = Cell::new(0);
        let key = Key {
            request: 1,
            cause: Cause::Missing,
        };
        let create = || {
            made.set(made.get() + 1);
            source()
        };
        let first = state.failure(key, create);
        let first_origin = origin(&first);
        let repeated = state.failure(key, create);
        assert_eq!(made.get(), 1);
        assert!(Arc::ptr_eq(&first_origin, &origin(&repeated)));
        assert!(std::ptr::eq(
            first_origin.backtrace(),
            origin(&repeated).backtrace()
        ));
        assert!(
            repeated
                .diagnostics()
                .next()
                .unwrap()
                .source()
                .unwrap()
                .downcast_ref::<io::Error>()
                .is_some()
        );

        let different_reason = state.failure(
            Key {
                cause: Cause::Invalid,
                ..key
            },
            create,
        );
        let different_request = state.failure(
            Key {
                request: 2,
                cause: Cause::Invalid,
            },
            create,
        );
        assert_eq!(made.get(), 3);
        assert_ne!(first_origin.id(), origin(&different_reason).id());
        assert_ne!(
            origin(&different_reason).id(),
            origin(&different_request).id()
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.phase() == Some(DiagnosticPhase::Source))
                .count(),
            3
        );
        assert!(
            logs.records()
                .iter()
                .all(|record| record.phase() != Some(DiagnosticPhase::Handled))
        );
    });
}

#[test]
fn independent_owners_with_the_same_key_and_text_keep_separate_origins_and_release_them() {
    let logs = LogCapture::default();
    logs.run(|| {
        let mut left = FailureState::default();
        let mut right = FailureState::default();
        let key = Key {
            request: 4,
            cause: Cause::Missing,
        };
        let a = left.failure(key, source);
        let b = right.failure(key, source);
        let a_origin = origin(&a);
        assert_ne!(a_origin.id(), origin(&b).id());
        let weak = Arc::downgrade(&a_origin);
        drop(a_origin);
        drop(a);
        drop(left);
        assert!(weak.upgrade().is_none());
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::Level::INFO)
                .count(),
            0
        );
    });
}

#[derive(Clone, Copy)]
enum Signal {
    Failed(Key),
    Waiting,
    Recovered,
}

#[test]
fn terminal_reporting_and_business_handling_have_independent_once_flags() {
    let logs = LogCapture::default();
    logs.run(|| {
        for handling_first in [false, true] {
            let shared = source();
            let id = shared.origin().id().clone();
            if handling_first {
                shared.record_handled("fallback", "cached content retained");
            }
            let mut reports = 0;
            for _ in 0..2 {
                dispatch_error(
                    WidgetryEcsError::from_shared(shared.clone()).into_bevy_error(),
                    bevy::ecs::error::ErrorContext::Command {
                        name: "retained failure".into(),
                    },
                    |_, _, _| {
                        reports += 1;
                    },
                    |_, _| panic!("managed diagnostic became native"),
                );
            }
            shared.record_handled("fallback", "cached content retained");
            shared.record_handled("fallback", "cached content retained");
            assert_eq!(reports, 1);
            assert_eq!(
                logs.diagnostic_records(&id, DiagnosticPhase::Handled).len(),
                1
            );
        }
    });
}

#[test]
fn handling_does_not_recover_the_owner_and_recovery_context_is_lazy() {
    let mut state = FailureState::default();
    let logs = LogCapture::default();
    let recovery_contexts = Cell::new(0);
    let recovered = || {
        recovery_contexts.set(recovery_contexts.get() + 1);
        ErrorFrame::new("owner recovery")
    };
    logs.run(|| {
        assert!(!state.recover(recovered));
        state.failure(
            Key {
                request: 1,
                cause: Cause::Missing,
            },
            source,
        );
        let diagnostic = state.active().unwrap();
        let id = diagnostic.origin().id().clone();
        diagnostic.record_handled("fallback", "previous projection retained");
        diagnostic.record_handled("fallback", "previous projection retained");
        assert!(state.active().is_some());
        assert_eq!(
            logs.diagnostic_records(&id, DiagnosticPhase::Handled).len(),
            1
        );
        assert_eq!(recovery_contexts.get(), 0);
        assert!(
            logs.records()
                .iter()
                .all(|record| record.level != bevy::log::Level::INFO)
        );
        assert!(state.recover(recovered));
        assert!(!state.recover(recovered));
        assert_eq!(recovery_contexts.get(), 1);
    });
}

#[derive(Resource)]
struct Owner {
    signal: Signal,
    failure: FailureState<Key>,
    made: usize,
}

fn observe_owner(mut owner: ResMut<Owner>) -> Result {
    let Owner {
        signal,
        failure,
        made,
    } = &mut *owner;
    match *signal {
        Signal::Failed(key) => Err(failure
            .failure(key, || {
                *made += 1;
                source()
            })
            .into_bevy_error()),
        Signal::Waiting => Ok(()),
        Signal::Recovered => {
            failure.recover(|| ErrorFrame::new("owner ready"));
            Ok(())
        }
    }
}

fn report_captured(errors: &ErrorCapture, reports: &mut Vec<Arc<ErrorOrigin>>) -> usize {
    let records = errors.take_records();
    let count = records.len();
    for record in records {
        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(record.context.kind(), "system");
        let mut native = None;
        dispatch_error(
            record.error,
            record.context,
            |diagnostic, _, _| {
                reports.push(Arc::clone(diagnostic.origin()));
                widgetry_error!(error_id = %diagnostic.origin().id(), phase = "unhandled", backtrace = %diagnostic.origin().backtrace(), "持续异常最终未处理");
            },
            |error, _| native = Some(error),
        );
        assert!(
            native.is_none(),
            "persistent failure became native: {native:?}"
        );
    }
    count
}

#[test]
fn real_system_repeats_stay_errors_waiting_keeps_state_and_recovery_starts_a_new_failure() {
    let key = Key {
        request: 9,
        cause: Cause::Missing,
    };
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.insert_resource(Owner {
        signal: Signal::Failed(key),
        failure: FailureState::default(),
        made: 0,
    });
    app.add_systems(Update, observe_owner);
    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    let mut reports = Vec::new();
    logs.run(|| {
        errors.run(|| {
            app.update();
            app.update();
        });
        assert_eq!(report_captured(&errors, &mut reports), 2);
        assert_eq!(app.world().resource::<Owner>().made, 1);
        assert_eq!(reports.len(), 1);
        let original = Arc::clone(
            app.world()
                .resource::<Owner>()
                .failure
                .active()
                .unwrap()
                .origin(),
        );
        assert!(Arc::ptr_eq(&original, &reports[0]));

        app.world_mut().resource_mut::<Owner>().signal = Signal::Waiting;
        errors.run(|| app.update());
        assert_eq!(report_captured(&errors, &mut reports), 0);
        assert!(Arc::ptr_eq(
            app.world()
                .resource::<Owner>()
                .failure
                .active()
                .unwrap()
                .origin(),
            &original
        ));
        app.world_mut().resource_mut::<Owner>().signal = Signal::Failed(key);
        errors.run(|| app.update());
        assert_eq!(report_captured(&errors, &mut reports), 1);
        assert_eq!(reports.len(), 1);

        app.world_mut().resource_mut::<Owner>().signal = Signal::Recovered;
        errors.run(|| {
            app.update();
            app.update();
        });
        assert_eq!(report_captured(&errors, &mut reports), 0);
        assert!(app.world().resource::<Owner>().failure.active().is_none());
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            1
        );

        app.world_mut().resource_mut::<Owner>().signal = Signal::Failed(key);
        errors.run(|| app.update());
        assert_eq!(report_captured(&errors, &mut reports), 1);
        assert_eq!(reports.len(), 2);
        assert_eq!(app.world().resource::<Owner>().made, 2);
        assert_ne!(reports[0].id(), reports[1].id());
        for report in &reports {
            assert_eq!(
                logs.diagnostic_records(report.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert_eq!(
                logs.diagnostic_records(report.id(), DiagnosticPhase::Unhandled)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(report.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
    });
}
