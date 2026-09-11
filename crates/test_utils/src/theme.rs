use bevy::app::App;
use bevy_widgetry_core::{ThemeChanged, ThemeMode};

/// 在测试中先更新主题资源再发出通知，保留立即刷新语义而不推进一帧。
/// 调用前需由主题或样式插件初始化 ThemeMode。
pub fn switch_theme(app: &mut App, mode: ThemeMode) {
    *app.world_mut().resource_mut::<ThemeMode>() = mode;
    app.world_mut().trigger(ThemeChanged { mode });
}
