#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod group;
mod input;
mod option;
mod style;

use bevy::input_focus::{InputFocusSystems, tab_navigation::TabNavigationPlugin};
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy_widgetry_core::ui::WidgetryUiPlugin;
use bevy_widgetry_internal_log::widgetry_info;
pub use colors::*;
pub use group::WidgetrySegmented;
pub use option::{WidgetrySegmentedOption, WidgetrySegmentedOptionProps};

#[derive(Default, Clone, Debug)]
pub struct WidgetrySegmentedProps {
    pub orientation: SegmentedOrientation,
    pub equal_width: bool,
    pub colors: WidgetrySegmentedColorOverrides,
}

pub struct WidgetrySegmentedPlugin;

impl Plugin for WidgetrySegmentedPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<bevy_widgetry_theme::WidgetryThemePlugin>() {
            app.add_plugins(bevy_widgetry_theme::WidgetryThemePlugin);
        }
        group::install(app);
        style::install(app);
        app.add_observer(input::on_click)
            .add_observer(input::on_key);
        app.add_systems(
            PreUpdate,
            group::initialize
                .before(InputFocusSystems::Dispatch)
                .before(PickingSystems::Hover),
        );
        widgetry_info!("WidgetrySegmentedPlugin 注册完成");
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentedOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Component, Default)]
pub(crate) struct OrientationState(pub(crate) SegmentedOrientation);
