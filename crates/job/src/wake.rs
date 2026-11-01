use crate::JobFailure;
use crate::manager::lock;
use bevy_widgetry_internal_log::widgetry_info;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub(crate) type Callback =
    Arc<dyn Fn() -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;

#[derive(Default)]
pub(crate) struct WakeState {
    revision: AtomicUsize,
    pending: AtomicBool,
    callback: Mutex<CallbackState>,
}

#[derive(Default)]
struct CallbackState {
    callback: Option<Callback>,
    faulted: Option<JobFailure>,
    error: Option<JobFailure>,
}

impl WakeState {
    pub fn notify(&self) {
        self.revision.fetch_add(1, Ordering::SeqCst);
        self.request();
    }

    fn request(&self) {
        if !self.pending.swap(true, Ordering::SeqCst) {
            self.post();
        }
    }

    fn post(&self) {
        let callback = lock(&self.callback).callback.clone();
        let Some(callback) = callback else {
            return;
        };
        let result = callback();
        let mut state = lock(&self.callback);
        match result {
            Err(error) if state.faulted.is_none() => {
                let failure = JobFailure::external_shared(
                    Arc::from(error),
                    bevy_widgetry_diagnostics::ErrorFrame::new("job.wake"),
                );
                state.error = Some(failure.clone());
                state.faulted = Some(failure);
            }
            Ok(()) if state.faulted.is_some() => {
                widgetry_info!("Job 主循环唤醒已恢复");
                if let Some(error) = state.faulted.take() {
                    state.error = None;
                    error.record_handled("重试并成功通知宿主主循环", "主循环唤醒已恢复");
                }
            }
            _ => {}
        }
    }

    pub fn connect(&self, callback: Option<Callback>) {
        lock(&self.callback).callback = callback;
        if self.pending.load(Ordering::SeqCst) {
            self.post();
        }
    }

    pub fn revision(&self) -> usize {
        self.revision.load(Ordering::SeqCst)
    }

    pub fn finish_update(&self, seen: usize) {
        self.pending.store(false, Ordering::SeqCst);
        if self.revision() != seen {
            self.request();
        }
    }

    pub fn take_error(&self) -> Option<JobFailure> {
        lock(&self.callback).error.take()
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn dense_notifications_merge_and_headless_pending_is_delivered_on_connection() {
        let state = WakeState::default();
        let calls = Arc::new(AtomicUsize::new(0));
        for _ in 0..500 {
            state.notify();
        }
        let count = calls.clone();
        state.connect(Some(Arc::new(move || {
            count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        for _ in 0..500 {
            state.notify();
        }
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        state.finish_update(state.revision());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        state.notify();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn notification_during_consumption_is_reposted_after_clearing_pending() {
        let state = Arc::new(WakeState::default());
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        state.connect(Some(Arc::new(move || {
            count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })));
        state.notify();
        let seen = state.revision();
        let producer = state.clone();
        std::thread::spawn(move || producer.notify())
            .join()
            .unwrap();
        state.finish_update(seen);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        state.finish_update(state.revision());
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        state.notify();
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn callback_failures_are_reported_once_until_recovery() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        let logs = LogCapture::default();
        logs.run(|| {
            let state = WakeState::default();
            state.connect(Some(Arc::new(|| {
                Err(Box::new(std::io::Error::from_raw_os_error(5)))
            })));
            state.notify();
            let first = state.take_error().unwrap();
            let origin = first.diagnostics().next().unwrap().origin().clone();
            state.finish_update(state.revision());
            state.notify();
            assert!(state.take_error().is_none());
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
            state.connect(Some(Arc::new(|| Ok(()))));
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .len(),
                1
            );
            state.finish_update(state.revision());
            state.connect(Some(Arc::new(|| {
                Err(Box::new(std::io::Error::from_raw_os_error(6)))
            })));
            state.notify();
            let next = state.take_error().unwrap();
            let next_origin = next.diagnostics().next().unwrap().origin();
            assert_ne!(origin.id(), next_origin.id());
            assert_eq!(
                logs.diagnostic_records(next_origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        });
    }
}
