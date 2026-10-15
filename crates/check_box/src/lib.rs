use bevy_widgetry_theme::WidgetryThemePlugin;
#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod checkbox;
mod indicator;
mod style;
mod tri_state;

use bevy::prelude::*;
use bevy::ui_widgets::CheckboxPlugin;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_core::ui::WidgetryUiPlugin;
use bevy_widgetry_internal_log::widgetry_info;
pub use checkbox::WidgetryCheckBox;
pub use tri_state::{WidgetryCheckState, WidgetryTriStateCheckbox};

pub struct WidgetryCheckBoxPlugin;

impl Plugin for WidgetryCheckBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<CheckboxPlugin>() {
            app.add_plugins(CheckboxPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }
        if !app.is_plugin_added::<WidgetryAssetPlugin>() {
            app.add_plugins(WidgetryAssetPlugin);
        }
        if !app.is_plugin_added::<WidgetryIconPlugin>() {
            app.add_plugins(WidgetryIconPlugin);
        }
        app.add_observer(tri_state::on_press)
            .add_observer(indicator::own_mark)
            .add_observer(style::refresh_theme)
            .add_observer(tri_state::on_click)
            .add_observer(tri_state::on_release)
            .add_observer(tri_state::on_drag_end)
            .add_observer(tri_state::on_cancel)
            .add_observer(tri_state::on_key)
            .add_systems(
                PostUpdate,
                style::update_mark_geometry
                    .after(bevy_widgetry_core::ui::WidgetryUiSystems::Materialize)
                    .before(bevy_widgetry_core::icon::WidgetryIconMaterialize),
            )
            .add_systems(
                PostUpdate,
                (
                    tri_state::sync_accessibility,
                    style::update_changed,
                    style::update_removed,
                )
                    .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
            );
        widgetry_info!("WidgetryCheckBoxPlugin 注册完成");
    }
}

mod colors;
pub use colors::*;

#[derive(Default, Clone, Debug)]
pub struct WidgetryCheckBoxProps {
    pub colors: WidgetryCheckBoxColorOverrides,
}

pub mod internal {
    pub use crate::style::apply_owned_checkbox_colors;
}
