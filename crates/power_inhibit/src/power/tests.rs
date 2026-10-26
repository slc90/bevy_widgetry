use super::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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
    fn call(&self, call: Call) -> Result<(), PowerError> {
        let mut state = self.0.lock().unwrap();
        state.calls.push(call);
        if state.failures.front() == Some(&call) {
            state.failures.pop_front();
            return Err(PowerError::error(format!("{call:?}: Win32 error 5")));
        }
        Ok(())
    }

    fn calls(&self) -> Vec<Call> {
        self.0.lock().unwrap().calls.clone()
    }
}

impl Api for FakeApi {
    type Handle = FakeHandle;

    fn create(&mut self) -> Result<Self::Handle, PowerError> {
        self.call(Call::Create)?;
        Ok(FakeHandle(self.0.clone()))
    }

    fn set(&mut self, _handle: &Self::Handle, kind: RequestKind) -> Result<(), PowerError> {
        self.call(Call::Set(kind))
    }

    fn clear(&mut self, _handle: &Self::Handle, kind: RequestKind) -> Result<(), PowerError> {
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
        assert!(error.to_string().contains("Win32 error 5"));
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
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
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("Set(System)"));
        assert_eq!(error.to_string().contains("rollback"), rollback_fails);
        if rollback_fails {
            assert!(error.to_string().contains("Clear(Display)"));
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
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains(&format!("Clear({kind:?})")));
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
    assert!(error.to_string().contains("Clear(Display)"));
    assert!(error.to_string().contains("Clear(System)"));
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
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].level, bevy::log::Level::INFO);
    assert!(records[0].fields["message"].contains("WidgetryPowerInhibitPlugin 注册完成"));
    assert_eq!(records[1].level, bevy::log::Level::WARN);
    let error = &records[1].fields["error"];
    assert!(error.contains("Clear(Display)"));
    assert!(error.contains("Clear(System)"));
}
