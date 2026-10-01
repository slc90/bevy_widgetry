//! Workspace 内部复用的异常边界 bookkeeping；state 由负责行为的 Component 持有。

use bevy::prelude::BevyError;

/// 保存一个行为上次报告的失败；不初始化日志，不维护全局状态或每帧诊断 system。
#[derive(Default)]
pub struct FailureState(Option<String>);

impl FailureState {
    /// 始终保留 Result 错误通道，仅在异常变化或恢复时调用所属 module 的日志 closure。
    pub fn observe<T>(
        &mut self,
        result: Result<T, BevyError>,
        failed: impl FnOnce(&BevyError),
        recovered: impl FnOnce(),
    ) -> Result<T, BevyError> {
        match &result {
            Err(error) => {
                // BevyError 的 Display 附带当前调用栈；错误首行表示原因，调用栈不属于异常 identity。
                let display = error.to_string();
                let message = display.lines().next().unwrap_or_default().to_owned();
                if self.0.as_ref() != Some(&message) {
                    failed(error);
                    self.0 = Some(message);
                }
            }
            Ok(_) => {
                if self.0.take().is_some() {
                    recovered();
                }
            }
        }
        result
    }
}
