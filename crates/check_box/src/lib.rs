//! 二态与三态 CheckBox 的 BSN 入口和共享样式。

mod checkbox;
mod indicator;
mod style;
mod tri_state;

use bevy::prelude::*;
use bevy::ui_widgets::CheckboxPlugin;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_core::{ForegroundColorPlugin, ThemePlugin};
use bevy_widgetry_log::widgetry_info;
pub use checkbox::WidgetryCheckBox;
pub use tri_state::{WidgetryCheckState, WidgetryTriStateCheckbox};

/// 装配二态、三态 CheckBox、theme 与 icon；须在 AssetPlugin 之后注册，BSN 构造依赖 ScenePlugin。
/// 文本字体由调用方配置，本 plugin 不安装字体 fallback。
pub struct WidgetryCheckBoxPlugin;

impl Plugin for WidgetryCheckBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<CheckboxPlugin>() {
            app.add_plugins(CheckboxPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }
        if !app.is_plugin_added::<WidgetryAssetPlugin>() {
            app.add_plugins(WidgetryAssetPlugin);
        }
        if !app.is_plugin_added::<WidgetryIconPlugin>() {
            app.add_plugins(WidgetryIconPlugin);
        }
        app.add_observer(tri_state::on_press)
            .add_observer(tri_state::on_click)
            .add_observer(tri_state::on_release)
            .add_observer(tri_state::on_drag_end)
            .add_observer(tri_state::on_cancel)
            .add_observer(tri_state::on_key)
            .add_observer(tri_state::widgetry_tri_state_checkbox_self_update)
            .add_systems(Update, (tri_state::sync_accessibility, style::update_style));
        widgetry_info!("WidgetryCheckBoxPlugin 注册完成");
    }
}
