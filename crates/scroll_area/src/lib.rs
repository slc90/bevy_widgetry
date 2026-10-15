#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod disabled;
mod headless;
mod layout;
mod pointer;
mod style;

pub use headless::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollAreaContent,
    WidgetryScrollAreaPlugin, WidgetryScrollAreaViewport, WidgetryScrollIntoView,
};
pub use style::{WidgetryScrollArea, WidgetryScrollAreaProps};

mod colors;
pub use colors::*;
pub mod internal {
    pub use crate::style::apply_owned_scroll_colors;
}
