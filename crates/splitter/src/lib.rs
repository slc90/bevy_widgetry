#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod layout;
mod ratio;
mod splitter;

pub use splitter::{WidgetrySplitter, WidgetrySplitterOrientation, WidgetrySplitterProps};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetrySplitterPlugin;

impl Plugin for WidgetrySplitterPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        app.register_type::<splitter::State>()
            .register_type::<WidgetrySplitterOrientation>();
        layout::install(app);
        widgetry_info!("WidgetrySplitterPlugin 注册完成");
    }
}
