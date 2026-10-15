#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod lifecycle;
mod scene;

use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_internal_log::widgetry_info;
use bevy_widgetry_window::WidgetryWindowPlugin;
pub use scene::{
    WidgetryMessageBox, WidgetryMessageBoxButtons, WidgetryMessageBoxResult,
    WidgetryMessageBoxResultEvent, widgetry_message_box,
};

pub struct WidgetryMessageBoxPlugin;

impl Plugin for WidgetryMessageBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryWindowPlugin>() {
            app.add_plugins(WidgetryWindowPlugin);
        }
        if !app.is_plugin_added::<WidgetryButtonPlugin>() {
            app.add_plugins(WidgetryButtonPlugin);
        }
        app.add_systems(
            PostUpdate,
            style::establish_owners.in_set(bevy_widgetry_core::ui::WidgetryUiSystems::StyleOwners),
        );
        app.add_systems(
            PostUpdate,
            style::update_colors.in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
        app.add_observer(style::own_window)
            .add_observer(style::own_action);
        app.add_observer(style::refresh_theme)
            .add_observer(lifecycle::finish_closing)
            .add_observer(lifecycle::escape)
            .add_systems(
                Last,
                lifecycle::close_requests.before(bevy::window::close_when_requested),
            );
        widgetry_info!("WidgetryMessageBoxPlugin 注册完成");
    }
}

mod colors;
pub use colors::*;
mod style;
pub mod internal {
    pub use crate::style::apply_owned_message_box_colors;
}
