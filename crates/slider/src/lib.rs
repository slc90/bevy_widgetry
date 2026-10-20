#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod slider;
mod value;

pub use slider::{WidgetrySlider, WidgetrySliderOrientation, WidgetrySliderProps};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetrySliderPlugin;

impl Plugin for WidgetrySliderPlugin {
    fn build(&self, _app: &mut App) {
        widgetry_info!("WidgetrySliderPlugin 注册完成");
    }
}
