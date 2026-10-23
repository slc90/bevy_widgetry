#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod progress;

pub use progress::{WidgetryProgressBar, WidgetryProgressBarProps, WidgetryProgressState};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryProgressBarPlugin;

impl Plugin for WidgetryProgressBarPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<progress::ProgressState>()
            .register_type::<WidgetryProgressState>();
        widgetry_info!("WidgetryProgressBarPlugin 注册完成");
    }
}
