#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod error;
mod headless;
mod model;
pub use headless::{
    WidgetryDatePickerHeadless, WidgetryDatePickerHeadlessPlugin, WidgetryDatePickerHeadlessProps,
};
mod today;
mod view;
pub use colors::WidgetryDatePickerColorOverrides;
pub use error::{DatePickerError, DatePickerFailure};

pub use model::*;
pub use time::{Date, Month, Weekday};
pub use view::WidgetryDatePickerLanguage;

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_debug;

pub struct WidgetryDatePickerPlugin;

impl Plugin for WidgetryDatePickerPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryDatePickerHeadlessPlugin>() {
            app.add_plugins(WidgetryDatePickerHeadlessPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
            app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        }
        if !app.is_plugin_added::<bevy_widgetry_asset::WidgetryAssetPlugin>() {
            app.add_plugins(bevy_widgetry_asset::WidgetryAssetPlugin);
        }
        if !app.is_plugin_added::<bevy_widgetry_core::icon::WidgetryIconPlugin>() {
            app.add_plugins(bevy_widgetry_core::icon::WidgetryIconPlugin);
        }
        view::install(app);
        widgetry_debug!("WidgetryDatePickerPlugin 注册完成");
    }
}
