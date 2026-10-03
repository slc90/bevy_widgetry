//! 提供统一的 Widgetry 日志 macro。

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
