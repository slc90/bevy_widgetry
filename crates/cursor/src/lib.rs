#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod coordination;
mod lease;
mod presentation;
mod theme;

pub use coordination::WidgetryCursorPlugin;
pub use lease::CursorLease;
pub use theme::{WidgetryCursorHint, WidgetryCursorTheme};

pub mod internal {
    #[derive(bevy::prelude::Component)]
    pub struct LoadingCursor;
}
