use crate::manager::lock;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub(crate) type Callback = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

#[derive(Default)]
pub(crate) struct WakeState {
    revision: AtomicUsize,
    pending: AtomicBool,
    callback: Mutex<CallbackState>,
}

#[derive(Default)]
struct CallbackState {
    callback: Option<Callback>,
    faulted: bool,
    error: Option<String>,
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
            Err(error) if !state.faulted => {
                widgetry_error!(%error, "Job 主循环唤醒失败");
                state.error = Some(error);
                state.faulted = true;
            }
            Ok(()) if state.faulted => {
                widgetry_info!("Job 主循环唤醒已恢复");
                state.faulted = false;
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

    pub fn take_error(&self) -> Option<String> {
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
        let state = WakeState::default();
        state.connect(Some(Arc::new(|| Err("event loop closed".into()))));
        state.notify();
        assert_eq!(state.take_error().as_deref(), Some("event loop closed"));
        state.finish_update(state.revision());
        state.notify();
        assert!(state.take_error().is_none());
        state.connect(Some(Arc::new(|| Ok(()))));
        state.finish_update(state.revision());
        state.connect(Some(Arc::new(|| Err("new failure".into()))));
        state.notify();
        assert_eq!(state.take_error().as_deref(), Some("new failure"));
    }
}
