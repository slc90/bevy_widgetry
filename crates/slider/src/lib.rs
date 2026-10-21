#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod gesture;
mod input;
mod lifecycle;
mod slider;
mod style;
mod value;

pub use colors::{WidgetrySliderColorOverrides, WidgetrySliderStateColorOverrides};
pub use lifecycle::{SliderDragEnded, SliderDragOutcome, SliderDragStarted};
pub use slider::{WidgetrySlider, WidgetrySliderOrientation, WidgetrySliderProps};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetrySliderPlugin;

impl Plugin for WidgetrySliderPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<bevy::input_focus::tab_navigation::TabNavigationPlugin>() {
            app.add_plugins(bevy::input_focus::tab_navigation::TabNavigationPlugin);
        }
        input::install(app);
        style::install(app);
        widgetry_info!("WidgetrySliderPlugin 注册完成");
    }
}
