#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod collapsible;
mod disabled;
mod input;
mod state;

pub use collapsible::{WidgetryCollapsible, WidgetryCollapsibleProps};
pub use state::WidgetryCollapsibleChanged;

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryCollapsiblePlugin;

impl Plugin for WidgetryCollapsiblePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_button::WidgetryButtonPlugin>() {
            app.add_plugins(bevy_widgetry_button::WidgetryButtonPlugin);
        }
        state::install(app);
        if !app.is_plugin_added::<bevy::input_focus::tab_navigation::TabNavigationPlugin>() {
            app.add_plugins(bevy::input_focus::tab_navigation::TabNavigationPlugin);
        }
        widgetry_info!("WidgetryCollapsiblePlugin 注册完成");
    }
}
