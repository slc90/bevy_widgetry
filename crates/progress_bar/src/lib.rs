#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod progress;
mod style;

pub use colors::WidgetryProgressBarColorOverrides;
pub use progress::{WidgetryProgressBar, WidgetryProgressBarProps, WidgetryProgressState};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryProgressBarPlugin;

impl Plugin for WidgetryProgressBarPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        app.register_type::<progress::ProgressState>()
            .register_type::<WidgetryProgressState>();
        style::install(app);
        widgetry_info!("WidgetryProgressBarPlugin 注册完成");
    }
}
