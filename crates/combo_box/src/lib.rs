//! 通过 BSN 组合独立 ListModel 数据的 theme ComboBox。

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

/// 注册 ComboBox 公共基础设施、Button、icon、theme 和内建 asset；须在 Bevy AssetPlugin 之后添加。
/// BSN 构造依赖 Bevy Scene 设施；还需通过 WidgetryComboBoxAppExt 注册业务 type。
/// 应用需提供官方 InputFocusPlugin 和实际 input/picking 派发；文本字体由调用方配置。
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
