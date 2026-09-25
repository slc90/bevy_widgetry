//! 通过 BSN 组合任意 option 内容的 theme ComboBox。

mod combo_box;
mod field;
mod option;
mod popup;

use bevy::prelude::*;
use bevy::ui_widgets::{ListBoxPlugin, popover::PopoverPlugin};
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_log::widgetry_info;
pub use combo_box::{WidgetryComboBox, WidgetryComboBoxOptionFactory, WidgetryComboBoxProps};

/// 注册完整 ComboBox 行为、Button、icon、theme 和内建 asset；须在 Bevy AssetPlugin 之后添加。
/// BSN 构造依赖 Bevy Scene 设施；option 中的文本字体由调用方配置。
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
        if !app.is_plugin_added::<ListBoxPlugin>() {
            app.add_plugins(ListBoxPlugin);
        }
        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }
        app.add_observer(combo_box::handle_value_change)
            .add_observer(popup::handle_field_activate)
            .add_observer(popup::handle_outside_click)
            .add_observer(popup::handle_reselect)
            .add_observer(popup::refresh_theme)
            .add_observer(option::refresh_theme)
            .add_systems(
                PreUpdate,
                (
                    field::mirror_disabled_added,
                    field::mirror_disabled_removed,
                    field::initialize_disabled,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    field::sync_content,
                    field::sync_icon,
                    option::update_changed,
                    option::update_removed,
                    option::update_disabled,
                ),
            );
        widgetry_info!("WidgetryComboBoxPlugin 注册完成");
    }
}
