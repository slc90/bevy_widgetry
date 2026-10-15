use bevy_widgetry_theme::WidgetryThemePlugin;
#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod group;
mod group_style;
mod option;
mod option_style;

use bevy::input_focus::InputFocusSystems;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy::ui_widgets::{RadioGroupPlugin, radio_self_update};
use bevy_widgetry_core::ui::WidgetryUiPlugin;
use bevy_widgetry_internal_log::widgetry_info;
pub use group::WidgetryRadioGroup;
pub use option::{WidgetryRadioOption, WidgetryRadioOptionProps};

pub struct WidgetryRadioGroupPlugin;

impl Plugin for WidgetryRadioGroupPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<RadioGroupPlugin>() {
            app.add_plugins(RadioGroupPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }
        group::install(app);
        app.add_observer(radio_self_update)
            .add_observer(group::handle_value_change)
            .add_observer(group_style::refresh_theme)
            .add_observer(option_style::refresh_theme);
        app.add_systems(
            PreUpdate,
            group::initialize
                .before(InputFocusSystems::Dispatch)
                .before(PickingSystems::Hover),
        );
        app.add_systems(
            PostUpdate,
            (
                group_style::update_changed,
                group_style::update_focus_and_removed,
                option_style::update_changed,
                option_style::update_removed,
            )
                .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
        widgetry_info!("WidgetryRadioGroupPlugin 注册完成");
    }
}

mod colors;
pub use colors::*;

#[derive(Default, Clone, Debug)]
pub struct WidgetryRadioGroupProps {
    pub colors: WidgetryRadioGroupColorOverrides,
}
