#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod error;
mod gesture;
mod input;
mod interaction;
mod lifecycle;
mod linear;
mod slider;
mod style;
mod value;

pub use colors::{WidgetrySliderColorOverrides, WidgetrySliderStateColorOverrides};
pub use error::{SliderError, SliderFailure, SliderNumericConfig, SliderValueRejection};
pub use interaction::{
    SliderDirection, SliderDragStrategy, SliderInteractionStart, SliderInteractionStrategy,
    SliderPointerInput, SliderProjection, WidgetrySliderInteraction,
};
pub use lifecycle::{SliderDragEnded, SliderDragOutcome, SliderDragStarted};
pub use linear::WidgetrySliderLinear;
pub use slider::{WidgetrySlider, WidgetrySliderOrientation, WidgetrySliderProps};
pub use slider::{WidgetrySliderHeadless, WidgetrySliderHeadlessProps};
pub use value::WidgetrySliderState;

pub struct WidgetrySliderHeadlessPlugin;
impl Plugin for WidgetrySliderHeadlessPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WidgetrySliderState>();
        widgetry_debug!("WidgetrySliderHeadlessPlugin 注册完成");
    }
}

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_debug;

pub struct WidgetrySliderPlugin;

pub struct WidgetrySliderInteractionPlugin;
impl Plugin for WidgetrySliderInteractionPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetrySliderHeadlessPlugin>() {
            app.add_plugins(WidgetrySliderHeadlessPlugin);
        }
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<bevy::input_focus::tab_navigation::TabNavigationPlugin>() {
            app.add_plugins(bevy::input_focus::tab_navigation::TabNavigationPlugin);
        }
        input::install(app);
        widgetry_debug!("WidgetrySliderInteractionPlugin 注册完成");
    }
}

impl Plugin for WidgetrySliderPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetrySliderInteractionPlugin>() {
            app.add_plugins(WidgetrySliderInteractionPlugin);
        }
        style::install(app);
        widgetry_debug!("WidgetrySliderPlugin 注册完成");
    }
}
