use super::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[test]
fn rollback_failures_retain_os_sources_instead_of_only_combined_text() {
    let (mut power, _) = owner(&[
        Call::Set(RequestKind::System),
        Call::Clear(RequestKind::Display),
    ]);
    let error = power.enable().unwrap_err();
    use std::error::Error;
    let failures: Vec<_> = error.failures().collect();
    assert_eq!(failures.len(), 2);
    assert_ne!(failures[0].origin().id(), failures[1].origin().id());
    for failure in &failures {
        assert_eq!(
            failure
                .kind()
                .source()
                .unwrap()
                .downcast_ref::<io::Error>()
                .unwrap()
                .raw_os_error(),
            Some(5)
        );
        assert_eq!(
            failure.origin().backtrace().status(),
            std::backtrace::BacktraceStatus::Captured
        );
    }
    assert_eq!(failures[0].kind().operation, PowerOperation::Set);
    assert_eq!(failures[0].kind().phase, PowerPhase::Enable);
    assert_eq!(failures[0].kind().request, Some(RequestKind::System));
    assert!(failures[0].kind().display);
    assert!(!failures[0].kind().system);
    assert_eq!(failures[1].kind().operation, PowerOperation::Clear);
    assert_eq!(failures[1].kind().phase, PowerPhase::Rollback);
    assert_eq!(failures[1].kind().request, Some(RequestKind::Display));
}

#[test]
fn independent_power_failures_reach_real_command_handler_with_original_origins() {
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
    for rollback in [false, true] {
        let logs = LogCapture::default();
        logs.run(|| {
            let failures = if rollback {
                let (mut power, _) = owner(&[
                    Call::Set(RequestKind::System),
                    Call::Clear(RequestKind::Display),
                ]);
                power.enable().unwrap_err()
            } else {
                let (mut power, _) = owner(&[
                    Call::Clear(RequestKind::Display),
                    Call::Clear(RequestKind::System),
                ]);
                power.enable().unwrap();
                power.disable().unwrap_err()
            };
            let origins: Vec<_> = failures
                .failures()
                .map(|failure| failure.origin().clone())
                .collect();
            assert_eq!(origins.len(), 2);
            let mut app = App::new();
            app.set_error_handler(ErrorCapture::handler());
            let capture = ErrorCapture::default();
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result { Err(failures.into_bevy_error()) });
            capture.run(|| app.world_mut().flush());
            let records = capture.take_records();
            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].error.severity(),
                bevy::ecs::error::Severity::Error
            );
            let diagnostics: Vec<_> = records[0].managed().unwrap().diagnostics().collect();
            assert_eq!(diagnostics.len(), 2);
            for (diagnostic, origin) in diagnostics.into_iter().zip(origins) {
                assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
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
    }
}

#[test]
fn successful_rollback_does_not_consume_primary_failure_and_noops_are_quiet() {
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
    let logs = LogCapture::default();
    logs.run(|| {
        let (mut power, api) = owner(&[Call::Set(RequestKind::System)]);
        let failures = power.enable().unwrap_err();
        let failure = failures.failures().next().unwrap();
        assert!(power.request.is_none());
        assert_eq!(api.calls().last(), Some(&Call::Close));
        assert_eq!(
            logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        let count = logs.records().len();
        power.disable().unwrap();
        power.enable().unwrap();
        power.enable().unwrap();
        power.disable().unwrap();
        power.disable().unwrap();
        assert_eq!(logs.records().len(), count);
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    Create,
    Set(RequestKind),
    Clear(RequestKind),
    Close,
}

#[derive(Default)]
struct State {
    calls: Vec<Call>,
    failures: VecDeque<Call>,
}

#[derive(Clone, Default)]
struct FakeApi(Arc<Mutex<State>>);

struct FakeHandle(Arc<Mutex<State>>);

impl Drop for FakeHandle {
    fn drop(&mut self) {
        self.0.lock().unwrap().calls.push(Call::Close);
    }
}

impl FakeApi {
    fn call(&self, call: Call) -> io::Result<()> {
        let mut state = self.0.lock().unwrap();
        state.calls.push(call);
        if state.failures.front() == Some(&call) {
            state.failures.pop_front();
            return Err(io::Error::from_raw_os_error(5));
        }
        Ok(())
    }

    fn calls(&self) -> Vec<Call> {
        self.0.lock().unwrap().calls.clone()
    }
}

impl Api for FakeApi {
    type Handle = FakeHandle;

    fn create(&mut self) -> io::Result<Self::Handle> {
        self.call(Call::Create)?;
        Ok(FakeHandle(self.0.clone()))
    }

    fn set(&mut self, _handle: &Self::Handle, kind: RequestKind) -> io::Result<()> {
        self.call(Call::Set(kind))
    }

    fn clear(&mut self, _handle: &Self::Handle, kind: RequestKind) -> io::Result<()> {
        self.call(Call::Clear(kind))
    }
}

fn owner(failures: &[Call]) -> (Owner<FakeApi>, FakeApi) {
    let api = FakeApi::default();
    api.0.lock().unwrap().failures.extend(failures);
    (
        Owner {
            api: api.clone(),
            request: None,
        },
        api,
    )
}

#[test]
fn repeated_commands_are_balanced_and_can_reenable() {
    let (mut power, api) = owner(&[]);
    power.disable().unwrap();
    assert!(api.calls().is_empty());
    power.enable().unwrap();
    power.enable().unwrap();
    assert_eq!(
        api.calls(),
        [
            Call::Create,
            Call::Set(RequestKind::Display),
            Call::Set(RequestKind::System)
        ]
    );
    power.disable().unwrap();
    power.disable().unwrap();
    power.enable().unwrap();
    power.disable().unwrap();
    let cycle = [
        Call::Create,
        Call::Set(RequestKind::Display),
        Call::Set(RequestKind::System),
        Call::Clear(RequestKind::Display),
        Call::Clear(RequestKind::System),
        Call::Close,
    ];
    assert_eq!(api.calls(), cycle.repeat(2));
}

#[test]
fn failed_create_or_first_set_keeps_no_request_and_allows_retry() {
    for failure in [Call::Create, Call::Set(RequestKind::Display)] {
        let (mut power, api) = owner(&[failure]);
        let error = power.enable().unwrap_err();
        assert_eq!(
            error
                .failures()
                .next()
                .unwrap()
                .kind()
                .source
                .raw_os_error(),
            Some(5)
        );
        assert!(power.request.is_none());
        let expected = if failure == Call::Create {
            vec![Call::Create]
        } else {
            vec![Call::Create, failure, Call::Close]
        };
        assert_eq!(api.calls(), expected);
        power.enable().unwrap();
        power.disable().unwrap();
        assert!(power.request.is_none());
    }
}

#[test]
fn second_set_failure_rolls_back_and_closes_even_if_rollback_fails() {
    for rollback_fails in [false, true] {
        let mut failures = vec![Call::Set(RequestKind::System)];
        if rollback_fails {
            failures.push(Call::Clear(RequestKind::Display));
        }
        let (mut power, api) = owner(&failures);
        let error = power.enable().unwrap_err();
        let diagnostics: Vec<_> = error.failures().collect();
        assert_eq!(diagnostics[0].kind().request, Some(RequestKind::System));
        assert_eq!(diagnostics.len(), if rollback_fails { 2 } else { 1 });
        if rollback_fails {
            assert_eq!(diagnostics[1].kind().request, Some(RequestKind::Display));
            assert_eq!(diagnostics[1].kind().phase, PowerPhase::Rollback);
        }
        assert!(power.request.is_none());
        assert_eq!(
            api.calls(),
            [
                Call::Create,
                Call::Set(RequestKind::Display),
                Call::Set(RequestKind::System),
                Call::Clear(RequestKind::Display),
                Call::Close
            ]
        );
        power.disable().unwrap();
        power.enable().unwrap();
        power.disable().unwrap();
        assert_eq!(
            api.calls()
                .iter()
                .filter(|call| **call == Call::Close)
                .count(),
            2
        );
    }
}

#[test]
fn partial_clear_retries_only_uncleared_types() {
    for kind in [RequestKind::Display, RequestKind::System] {
        let (mut power, api) = owner(&[Call::Clear(kind)]);
        power.enable().unwrap();
        let error = power.disable().unwrap_err();
        assert_eq!(error.failures().next().unwrap().kind().request, Some(kind));
        assert!(!api.calls().contains(&Call::Close));
        let before_retry = api.calls().len();
        power.disable().unwrap();
        assert_eq!(
            &api.calls()[before_retry..],
            [Call::Clear(kind), Call::Close]
        );
        power.disable().unwrap();
        drop(power);
        assert_eq!(
            &api.calls()[before_retry..],
            [Call::Clear(kind), Call::Close]
        );
    }
}

#[test]
fn both_clear_failures_are_reported_and_retryable() {
    let (mut power, api) = owner(&[
        Call::Clear(RequestKind::Display),
        Call::Clear(RequestKind::System),
    ]);
    power.enable().unwrap();
    let error = power.disable().unwrap_err();
    let failures: Vec<_> = error.failures().collect();
    assert_eq!(failures.len(), 2);
    assert_eq!(failures[0].kind().request, Some(RequestKind::Display));
    assert_eq!(failures[1].kind().request, Some(RequestKind::System));
    assert_ne!(failures[0].origin().id(), failures[1].origin().id());
    assert!(
        failures
            .iter()
            .all(|failure| failure.kind().source.raw_os_error() == Some(5))
    );
    let before = api.calls().len();
    power.disable().unwrap();
    assert_eq!(
        &api.calls()[before..],
        [
            Call::Clear(RequestKind::Display),
            Call::Clear(RequestKind::System),
            Call::Close
        ]
    );
}

#[test]
fn enable_after_partial_disable_cleans_before_recreating() {
    let (mut power, api) = owner(&[
        Call::Clear(RequestKind::Display),
        Call::Clear(RequestKind::Display),
    ]);
    power.enable().unwrap();
    power.disable().unwrap_err();
    let before = api.calls().len();
    power.enable().unwrap_err();
    assert_eq!(&api.calls()[before..], [Call::Clear(RequestKind::Display)]);
    let before = api.calls().len();
    power.enable().unwrap();
    assert_eq!(
        &api.calls()[before..],
        [
            Call::Clear(RequestKind::Display),
            Call::Close,
            Call::Create,
            Call::Set(RequestKind::Display),
            Call::Set(RequestKind::System)
        ]
    );
    power.disable().unwrap();
}

#[test]
fn drop_clears_remaining_requests_and_closes_once() {
    for initially_enabled in [false, true] {
        let (mut power, api) = owner(&[]);
        if initially_enabled {
            power.enable().unwrap();
        }
        let before = api.calls().len();
        drop(power);
        let expected = if initially_enabled {
            vec![
                Call::Clear(RequestKind::Display),
                Call::Clear(RequestKind::System),
                Call::Close,
            ]
        } else {
            vec![]
        };
        assert_eq!(&api.calls()[before..], expected);
    }
    let (mut power, api) = owner(&[Call::Clear(RequestKind::Display)]);
    power.enable().unwrap();
    power.disable().unwrap_err();
    let before = api.calls().len();
    drop(power);
    assert_eq!(
        &api.calls()[before..],
        [Call::Clear(RequestKind::Display), Call::Close]
    );
}

#[test]
fn plugin_default_and_drop_failure_have_the_expected_lifecycle_logs() {
    let (mut power, api) = owner(&[
        Call::Clear(RequestKind::Display),
        Call::Clear(RequestKind::System),
    ]);
    power.enable().unwrap();
    let capture = bevy_widgetry_test_utils::LogCapture::default();
    capture.run(|| {
        let mut app = App::new();
        app.add_plugins(WidgetryPowerInhibitPlugin);
        assert!(
            app.world()
                .resource::<PowerInhibit>()
                .owner
                .request
                .is_none()
        );
        app.update();
        assert!(
            app.world()
                .resource::<PowerInhibit>()
                .owner
                .request
                .is_none()
        );
        drop(power);
    });
    assert_eq!(api.calls().last(), Some(&Call::Close));
    assert_eq!(
        api.calls()
            .iter()
            .filter(|call| **call == Call::Close)
            .count(),
        1
    );
    let records = capture.records();
    assert_eq!(records.len(), 5);
    assert_eq!(records[0].level, bevy::log::Level::INFO);
    assert!(records[0].fields["message"].contains("WidgetryPowerInhibitPlugin 注册完成"));
    use bevy_widgetry_test_utils::DiagnosticPhase;
    let sources: Vec<_> = records
        .iter()
        .filter(|record| record.phase() == Some(DiagnosticPhase::Source))
        .collect();
    assert_eq!(sources.len(), 2);
    for source in sources {
        assert!(!source.fields.contains_key("backtrace"));
        let handled = records
            .iter()
            .find(|record| {
                record.phase() == Some(DiagnosticPhase::Handled)
                    && record.fields["error_id"] == source.fields["error_id"]
            })
            .unwrap();
        assert_eq!(handled.level, bevy::log::Level::DEBUG);
        assert!(handled.fields["outcome"].contains("最终关闭"));
    }
}
