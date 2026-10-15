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
