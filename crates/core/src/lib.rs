#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

pub mod color;
pub mod diagnostics;
pub mod disabled;
mod font;
pub mod foreground;
pub mod icon;
pub mod pointer;
pub mod scene;
pub mod text;
pub mod ui;
pub mod z_index;

pub use font::{WidgetryAppExt, WidgetryFontPlugin};
