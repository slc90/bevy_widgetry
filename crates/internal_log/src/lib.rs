//! 提供用于记录 Widgetry 运行信息和失败诊断的日志 macro。
//! 调用方可使用一致的 target 筛选这些日志，并保留具体调用位置与诊断字段。
//!
//! widgetry_info、widgetry_warn 和 widgetry_error 分别记录 INFO、WARN 和 ERROR 级别日志。
//! macro 支持 message 与 structured field，可附带 entity、source、错误内容等定位信息。
//! 所有 macro 使用 bevy_widgetry 作为日志 target。
//!
//! 日志的筛选、格式和输出位置由宿主配置的 subscriber 决定。
//! macro 使用当前日志环境，宿主负责安装和管理 subscriber。
//! 调用位置与 structured field 保留在记录中，便于将日志关联到具体业务操作。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

#[doc(hidden)]
pub use bevy::log as __log;

#[macro_export]
macro_rules! widgetry_info {
    ($($arg:tt)*) => { $crate::__log::info!(target: "bevy_widgetry", $($arg)*) };
}

#[macro_export]
macro_rules! widgetry_warn {
    ($($arg:tt)*) => { $crate::__log::warn!(target: "bevy_widgetry", $($arg)*) };
}

#[macro_export]
macro_rules! widgetry_error {
    ($($arg:tt)*) => { $crate::__log::error!(target: "bevy_widgetry", $($arg)*) };
}
