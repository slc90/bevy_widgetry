#![cfg(test)]

use bevy::text::FontSource;
use bevy::{app::App, color::Color, ecs::entity::Entity, prelude::*};
use bevy_widgetry::button::{
    LongPressButton, LongPressEvent, LongPressPlugin, StyledButton, StyledButtonPlugin,
};
use bevy_widgetry::combo_box::{
    ComboBox, ComboBoxPlugin, SetComboBoxSelected, StyledComboBoxPlugin,
};
use bevy_widgetry::icon::{Icon, IconPlugin, IconProps};
use bevy_widgetry::message_box::{
    MessageBox, MessageBoxButtons, MessageBoxPlugin, MessageBoxResult, MessageBoxResultEvent,
    message_box,
};
use bevy_widgetry::style::WidgetryAppExt;
use bevy_widgetry::style::{
    ColorTheme, DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin,
};
use bevy_widgetry::text_field::StyledTextFieldPlugin;
use bevy_widgetry::window::{WindowControlsConfig, WindowPlugin, window};

// 单独使用任一样式或窗口插件时，普通 Bevy 文本也自动获得同一内建 fallback。
#[test]
fn each_ui_plugin_installs_app_font_fallback() {
    for plugin in 0..4 {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Font>()
            .init_asset::<Image>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<bevy::input_focus::InputFocus>();
        match plugin {
            0 => {
                app.add_plugins(StyledButtonPlugin);
            }
            1 => {
                app.add_plugins(StyledComboBoxPlugin);
            }
            2 => {
                app.add_plugins(StyledTextFieldPlugin);
            }
            _ => {
                app.add_plugins(WindowPlugin);
            }
        }
        let entity = app.world_mut().spawn(TextFont::default()).id();
        app.update();
        assert_ne!(
            app.world().get::<TextFont>(entity).unwrap().font,
            FontSource::default()
        );
        assert!(matches!(
            app.world().get::<TextFont>(entity).unwrap().font,
            FontSource::Handle(_)
        ));
    }
}

// 从 facade 导入消费者需要的类型，验证重构后公开入口仍可构造。
#[test]
fn facade_public_types_are_usable() {
    let _ = LongPressButton::default();
    let _ = LongPressEvent {
        entity: Entity::PLACEHOLDER,
    };
    let _ = LongPressPlugin;
    let _ = StyledButton;
    let _ = StyledButtonPlugin;
    let _ = ComboBox;
    let _ = ComboBoxPlugin;
    let _ = SetComboBoxSelected {
        entity: Entity::PLACEHOLDER,
        selected: 1,
    };
    let _ = ForegroundColor(Color::WHITE);
}

// 同时装配多个样式插件，验证共享主题设施不会重复注册且可使用外部主题。
#[test]
fn style_theme_api_and_plugins_work_together() {
    let _: &ColorTheme = &DARK_THEME;
    assert_eq!(ThemeMode::Light.colors(), &LIGHT_THEME);
    let mut app = App::new();
    app.set_default_font(FontSource::Monospace);
    app.insert_resource(ThemeMode::Light).add_plugins((
        ThemePlugin,
        StyledButtonPlugin,
        StyledComboBoxPlugin,
    ));
    app.world_mut().trigger(ThemeChanged {
        mode: ThemeMode::Light,
    });
    app.update();
    assert_eq!(*app.world().resource::<ThemeMode>(), ThemeMode::Light);
}

// 消费者仅通过 facade 与 BSN 创建图标，无需取得 AssetServer，运行期组件仍可用于查询。
#[test]
fn icon_scene_api_is_usable() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        IconPlugin,
    ));
    let _ = IconProps::default();
    let plain = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! { @Icon { @path: "some/icon.svg" } })
        .id();
    let configured = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! {
            @Icon {
                @path: "some/icon.svg",
                @max_size: { Some(UVec2::new(24, 24)) },
                @color: { Some(Color::WHITE) },
            }
        })
        .id();
    app.world_mut().flush();
    for entity in [plain, configured] {
        assert!(app.world().get::<Icon>(entity).is_some());
        assert!(app.world().get::<Node>(entity).is_some());
    }
}

// 从 facade 组合空标题栏与主体场景，验证新的 Window 公开入口可直接用于 BSN。
#[test]
fn window_scene_api_is_usable() {
    let _ = WindowPlugin;
    let config = WindowControlsConfig::default();
    assert!(config.minimize_visible && config.maximize_visible);
    let _ = bsn! {
        window(Entity::PLACEHOLDER, Entity::PLACEHOLDER, config, bsn_list![], bsn_list![(Text("Body"))])
    };
}

/// facade 提供 MessageBox 类型与 BSN 函数，消费者不需要直接依赖内部 crate。
#[test]
fn message_box_scene_api_is_usable() {
    let _ = MessageBox;
    let _ = MessageBoxPlugin;
    let _ = MessageBoxResultEvent {
        entity: Entity::PLACEHOLDER,
        result: MessageBoxResult::Ok,
    };
    let _ = bsn! {
        message_box(Entity::PLACEHOLDER, "Confirm", MessageBoxButtons::YesNoCancel, bsn_list![(Text("Save changes?"))])
    };
}
