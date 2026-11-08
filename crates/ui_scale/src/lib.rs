#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod error;
mod scale;
mod windows;

pub use bevy_widgetry_core::window_scale::WindowScaleExcluded;
pub use error::{UiScaleError, UiScaleFailure};
pub use scale::WidgetryUiScale;
pub use windows::{WidgetryUiScalePlugin, WidgetryUiScaleSystems};
