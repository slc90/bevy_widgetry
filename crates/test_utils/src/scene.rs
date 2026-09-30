use bevy::prelude::*;
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput, NativeKey},
    },
    input_focus::InputFocusPlugin,
    picking::events::{Pointer, Release},
    ui_widgets::EditableTextInputPlugin,
    window::Ime,
};
use bevy_widgetry_core::WidgetryAppExt;

/// 为跨 Widget 的 BSN lifecycle 测试提供 headless 的 asset、字体策略与官方 InputFocusPlugin。
/// 调用方继续装配被测 Widget plugin，不创建 native window 或 render device。
pub fn scene_app() -> App {
    let mut app = App::new();
    app.set_default_font(bevy::text::FontSource::Monospace);
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), InputFocusPlugin));
    app.init_asset::<Image>();
    app.init_asset::<bevy::scene::ScenePatch>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.add_message::<bevy::window::WindowCloseRequested>();
    app
}

/// 在 headless Scene 环境中运行官方文本输入 observer 和 focus event dispatch，不装配 render pipeline。
pub fn text_input_app() -> App {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<Key>>()
        .init_resource::<UiScale>()
        .add_message::<Ime>()
        .add_message::<Pointer<Release>>()
        .add_plugins(EditableTextInputPlugin);
    app
}

/// 写入一次按键并推进官方 focused-input dispatch；调用方需装配 KeyboardInput message 与 dispatch system。
pub fn press_key(app: &mut App, window: Entity, key_code: KeyCode) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
}
