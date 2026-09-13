use bevy::prelude::*;
use bevy_widgetry_core::WidgetryAppExt;

/// 为跨控件 BSN 生命周期测试提供无桌面的资产、字体策略和指针资源。
/// 调用方继续装配被测控件插件，不创建原生窗口或渲染设备。
pub fn scene_app() -> App {
    let mut app = App::new();
    app.set_default_font(bevy::text::FontSource::Monospace);
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Image>();
    app.init_asset::<bevy::scene::ScenePatch>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.add_message::<bevy::window::WindowCloseRequested>();
    app
}
