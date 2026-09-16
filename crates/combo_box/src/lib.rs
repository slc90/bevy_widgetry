//! 通过 BSN 组合任意选项内容的主题下拉选择控件。

mod combo_box;
mod field;
mod option;
mod popup;

use bevy::prelude::*;
use bevy::ui_widgets::ListBoxPlugin;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_core::icon::IconPlugin;
use bevy_widgetry_log::widgetry_info;
pub use combo_box::{WidgetryComboBox, WidgetryComboBoxOptionFactory, WidgetryComboBoxProps};

/// 注册完整 ComboBox 行为、按钮、图标、主题和内建资产；须在 Bevy AssetPlugin 之后添加。
/// BSN 构造依赖 Bevy Scene 设施；选项中的文本字体由调用方配置。
pub struct WidgetryComboBoxPlugin;

impl Plugin for WidgetryComboBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryButtonPlugin>() {
            app.add_plugins(WidgetryButtonPlugin);
        }
        if !app.is_plugin_added::<WidgetryAssetPlugin>() {
            app.add_plugins(WidgetryAssetPlugin);
        }
        if !app.is_plugin_added::<IconPlugin>() {
            app.add_plugins(IconPlugin);
        }
        if !app.is_plugin_added::<ListBoxPlugin>() {
            app.add_plugins(ListBoxPlugin);
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
