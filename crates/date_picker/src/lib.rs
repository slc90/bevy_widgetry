#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod model;
mod view;
pub use colors::WidgetryDatePickerColorOverrides;

pub use model::*;
pub use time::{Date, Month, Weekday};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryDatePickerPlugin;

impl Plugin for WidgetryDatePickerPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
            app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        }
        view::install(app);
        widgetry_info!("WidgetryDatePickerPlugin 注册完成");
    }
}
