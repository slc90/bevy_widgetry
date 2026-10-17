#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod link;
mod link_colors;
mod style;
pub use colors::*;
pub use link::{WidgetryLinkButton, WidgetryLinkButtonProps};
pub use link_colors::WidgetryLinkButtonColorOverrides;

pub use style::{WidgetryButton, WidgetryButtonPlugin, WidgetryButtonProps};

pub mod internal {
    pub use crate::style::apply_owned_button_colors;
}
