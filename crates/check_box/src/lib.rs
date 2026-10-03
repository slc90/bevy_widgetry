//! 提供二态与三态 CheckBox、BSN Scene、theme style 和 selection 更新。

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
            .add_observer(style::refresh_theme)
            .add_observer(tri_state::on_click)
            .add_observer(tri_state::on_release)
            .add_observer(tri_state::on_drag_end)
            .add_observer(tri_state::on_cancel)
            .add_observer(tri_state::on_key)
            .add_systems(
                Update,
                (
                    tri_state::sync_accessibility,
                    style::update_changed,
                    style::update_removed,
                ),
            );
        widgetry_info!("WidgetryCheckBoxPlugin 注册完成");
    }
}
