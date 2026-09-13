mod lifecycle;
mod scene;

use bevy::prelude::*;
use bevy_widgetry_button::StyledButtonPlugin;
use bevy_widgetry_log::widgetry_info;
use bevy_widgetry_window::WindowPlugin;
pub use scene::{
    MessageBox, MessageBoxButtons, MessageBoxResult, MessageBoxResultEvent, message_box,
};

/// 注册 MessageBox 及其窗口、按钮依赖；须在 Bevy 资产、场景和文本插件之后添加。
pub struct MessageBoxPlugin;

impl Plugin for MessageBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WindowPlugin>() {
            app.add_plugins(WindowPlugin);
        }
        if !app.is_plugin_added::<StyledButtonPlugin>() {
            app.add_plugins(StyledButtonPlugin);
        }
        app.add_observer(scene::refresh_theme)
            .add_observer(lifecycle::begin_closing)
            .add_observer(lifecycle::finish_closing);
        widgetry_info!("MessageBoxPlugin 注册完成");
    }
}
