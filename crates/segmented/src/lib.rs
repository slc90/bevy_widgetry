#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod group;
mod input;
mod option;

use bevy::input_focus::{InputFocusSystems, tab_navigation::TabNavigationPlugin};
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy_widgetry_core::ui::WidgetryUiPlugin;
use bevy_widgetry_internal_log::widgetry_info;
pub use group::WidgetrySegmented;
pub use option::WidgetrySegmentedOption;

pub struct WidgetrySegmentedPlugin;

impl Plugin for WidgetrySegmentedPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        group::install(app);
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
