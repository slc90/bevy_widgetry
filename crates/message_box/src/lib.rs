mod lifecycle;
mod scene;

use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_log::widgetry_info;
use bevy_widgetry_window::WidgetryWindowPlugin;
pub use scene::{
    WidgetryMessageBox, WidgetryMessageBoxButtons, WidgetryMessageBoxResult,
    WidgetryMessageBoxResultEvent, widgetry_message_box,
};

/// 注册 WidgetryMessageBox 及其 Window、Button 依赖；须在 Bevy asset、Scene 和文本 plugin 之后添加。
pub struct WidgetryMessageBoxPlugin;

impl Plugin for WidgetryMessageBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryWindowPlugin>() {
            app.add_plugins(WidgetryWindowPlugin);
        }
        if !app.is_plugin_added::<WidgetryButtonPlugin>() {
            app.add_plugins(WidgetryButtonPlugin);
        }
        app.add_observer(scene::refresh_theme)
            .add_observer(lifecycle::begin_closing)
            .add_observer(lifecycle::finish_closing);
        widgetry_info!("WidgetryMessageBoxPlugin 注册完成");
    }
}
