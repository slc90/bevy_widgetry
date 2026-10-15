#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod combo_box;
mod field;
mod popup;
mod registration;

use bevy::prelude::*;
use bevy::ui_widgets::popover::PopoverPlugin;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_internal_log::widgetry_info;
use bevy_widgetry_list_view::WidgetryListViewPlugin;
pub use combo_box::{WidgetryComboBox, WidgetryComboBoxProps};
pub use registration::WidgetryComboBoxAppExt;

pub struct WidgetryComboBoxPlugin;

impl Plugin for WidgetryComboBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryButtonPlugin>() {
            app.add_plugins(WidgetryButtonPlugin);
        }
        if !app.is_plugin_added::<WidgetryAssetPlugin>() {
            app.add_plugins(WidgetryAssetPlugin);
        }
        if !app.is_plugin_added::<WidgetryIconPlugin>() {
            app.add_plugins(WidgetryIconPlugin);
        }
        if !app.is_plugin_added::<WidgetryListViewPlugin>() {
            app.add_plugins(WidgetryListViewPlugin);
        }
        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }
        app.add_observer(popup::handle_outside_click)
            .add_observer(field::own_dropdown_icon);
        widgetry_info!("WidgetryComboBoxPlugin 注册完成");
    }
}

mod colors;
pub use colors::*;

mod style;
pub mod internal {
    pub use crate::style::apply_owned_combo_colors;
}
