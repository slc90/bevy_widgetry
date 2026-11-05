#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod bridge;
mod error;
mod menu;
#[allow(unsafe_code)]
mod native;
mod plugin;

pub use bridge::{WidgetryTrayClick, WidgetryTrayMenuClick};
pub use error::{TrayError, TrayFailure};

pub use menu::{TrayMenuEntry, WidgetryTrayMenu};
pub use plugin::WidgetryTrayPlugin;
