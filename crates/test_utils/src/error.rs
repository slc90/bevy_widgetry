use bevy::ecs::error::{ErrorContext, ErrorHandler, match_severity};
use bevy::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

thread_local! {
    /// 只捕获当前测试 scope 的错误，避免跨测试污染或替换全局 subscriber。
    static ACTIVE_CAPTURE: RefCell<Option<ErrorCapture>> = const { RefCell::new(None) };
}

/// 捕获真实 system、observer 与 command 交给宿主 handler 的错误。
/// 对应 schedule 应使用 SingleThreadedExecutor，确保 handler 在捕获 thread 执行。
#[derive(Clone, Default)]
pub struct ErrorCapture(Arc<Mutex<Vec<BevyError>>>);

/// 在 scope 退出（包括测试断言失败）时恢复外层捕获。
struct CaptureGuard(Option<ErrorCapture>);

impl ErrorCapture {
    /// 将此 handler 安装到测试 App，再通过 run 限定捕获 scope。
    pub fn handler() -> ErrorHandler {
        capture_error
    }

    /// 捕获当前 thread 的 Bevy 错误，不把测试中的 panic 当成返回错误。
    pub fn run<R>(&self, action: impl FnOnce() -> R) -> R {
        let previous = ACTIVE_CAPTURE.with(|active| active.replace(Some(self.clone())));
        let _guard = CaptureGuard(previous);
        action()
    }

    /// 取出本 scope 捕获的错误，供测试验证 severity、内容与数量。
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

/// 没有显式捕获 scope 时保留 Bevy 的正常宿主行为。
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
