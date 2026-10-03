//! 提供基于 ListModel 的 ComboBox，支持选项展示、selection、Popup 和 theme style。

mod combo_box;
mod field;
mod popup;
mod registration;

use bevy::prelude::*;
use bevy::ui_widgets::popover::PopoverPlugin;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_list_view::WidgetryListViewPlugin;
use bevy_widgetry_log::widgetry_info;
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
            .add_observer(popup::refresh_theme);
        widgetry_info!("WidgetryComboBoxPlugin 注册完成");
    }
}
