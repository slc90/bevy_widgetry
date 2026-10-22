#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod input;
mod selection;
mod tabs;

pub use tabs::{TabsOrientation, WidgetryTab, WidgetryTabProps, WidgetryTabs, WidgetryTabsProps};

use bevy::input_focus::InputFocusSystems;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryTabsPlugin;

impl Plugin for WidgetryTabsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<bevy::input_focus::tab_navigation::TabNavigationPlugin>() {
            app.add_plugins(bevy::input_focus::tab_navigation::TabNavigationPlugin);
        }
        selection::install(app);
        app.add_observer(input::on_click)
            .add_observer(input::on_key)
            .add_systems(
                PreUpdate,
                selection::initialize
                    .before(InputFocusSystems::Dispatch)
                    .before(PickingSystems::Hover),
            )
            .add_systems(
                PostUpdate,
                selection::initialize.in_set(WidgetryUiSystems::Build),
            );
        widgetry_info!("WidgetryTabsPlugin 注册完成");
    }
}
