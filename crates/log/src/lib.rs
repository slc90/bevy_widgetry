//! Widgetry 内部统一日志宏；不配置 subscriber 或输出，不由 facade 导出。

/// 宏展开使用的卫生路径，不要求调用方以特定名称导入 Bevy。
#[doc(hidden)]
pub use bevy::log as __log;

/// 记录少量生命周期与已记录异常的恢复，固定 Widgetry target 并转发结构化字段。
#[macro_export]
macro_rules! widgetry_info {
    ($($arg:tt)*) => { $crate::__log::info!(target: "bevy_widgetry", $($arg)*) };
}

/// 记录内部吸收的非预期外部失败或实际能力降级。
#[macro_export]
macro_rules! widgetry_warn {
    ($($arg:tt)*) => { $crate::__log::warn!(target: "bevy_widgetry", $($arg)*) };
}

/// 记录 Widgetry 自身内部不变量被破坏等严重错误。
#[macro_export]
macro_rules! widgetry_error {
    ($($arg:tt)*) => { $crate::__log::error!(target: "bevy_widgetry", $($arg)*) };
}
