#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod model;

pub use model::*;
pub use time::{Date, Month, Weekday};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryDatePickerPlugin;

impl Plugin for WidgetryDatePickerPlugin {
    fn build(&self, _app: &mut App) {
        widgetry_info!("WidgetryDatePickerPlugin 注册完成");
    }
}
