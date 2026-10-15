use bevy::ecs::error::{ErrorContext, ErrorHandler, match_severity};
use bevy::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

thread_local! {
    static ACTIVE_CAPTURE: RefCell<Option<ErrorCapture>> = const { RefCell::new(None) };
}

// The error capture scope is thread-local.
// A multithreaded schedule can run the handler outside that scope, so use SingleThreadedExecutor.
#[derive(Clone, Default)]
pub struct ErrorCapture(Arc<Mutex<Vec<BevyError>>>);

struct CaptureGuard(Option<ErrorCapture>);

impl ErrorCapture {
    pub fn handler() -> ErrorHandler {
        capture_error
    }

    pub fn run<R>(&self, action: impl FnOnce() -> R) -> R {
        let previous = ACTIVE_CAPTURE.with(|active| active.replace(Some(self.clone())));
        let _guard = CaptureGuard(previous);
        action()
    }

    pub fn take(&self) -> Vec<BevyError> {
        std::mem::take(
            &mut *self
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }
}

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        ACTIVE_CAPTURE.with(|active| active.replace(self.0.take()));
    }
}

fn capture_error(error: BevyError, context: ErrorContext) {
    let capture = ACTIVE_CAPTURE.with(|active| active.borrow().clone());
    if let Some(capture) = capture {
        capture
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(error);
    } else {
        match_severity(error, context);
    }
}
