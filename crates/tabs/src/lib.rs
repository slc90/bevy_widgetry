#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod input;
mod scrolling;
mod selection;
mod style;
mod tabs;

pub use colors::*;

pub use tabs::{TabsOrientation, WidgetryTab, WidgetryTabProps, WidgetryTabs, WidgetryTabsProps};

use bevy::input_focus::InputFocusSystems;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryTabsPlugin;

impl Plugin for WidgetryTabsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiScale>();
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<bevy::input_focus::tab_navigation::TabNavigationPlugin>() {
            app.add_plugins(bevy::input_focus::tab_navigation::TabNavigationPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
            app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        }
        if !app.is_plugin_added::<bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin>() {
            app.add_plugins(bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin);
        }
        selection::install(app);
        style::install(app);
        scrolling::install(app);
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
