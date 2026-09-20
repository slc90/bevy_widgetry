#![cfg(test)]

use bevy::text::FontSource;
use bevy::{app::App, color::Color, ecs::entity::Entity, prelude::*};
use bevy_widgetry::button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry::combo_box::{
    WidgetryComboBox, WidgetryComboBoxOptionFactory, WidgetryComboBoxPlugin, WidgetryComboBoxProps,
};
use bevy_widgetry::icon::{WidgetryIcon, WidgetryIconPlugin, WidgetryIconProps};
use bevy_widgetry::message_box::{
    WidgetryMessageBox, WidgetryMessageBoxButtons, WidgetryMessageBoxPlugin,
    WidgetryMessageBoxResult, WidgetryMessageBoxResultEvent, widgetry_message_box,
};
use bevy_widgetry::radio_group::{
    WidgetryRadioGroup, WidgetryRadioGroupPlugin, WidgetryRadioOption,
};
use bevy_widgetry::style::WidgetryAppExt;
use bevy_widgetry::style::WidgetryFocusPlugin;
use bevy_widgetry::style::{
    ColorTheme, DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin,
};
use bevy_widgetry::text_field::{WidgetryTextField, WidgetryTextFieldPlugin};
use bevy_widgetry::window::{WidgetryWindowControlsConfig, WidgetryWindowPlugin, widgetry_window};

// facade 暴露完整 RadioGroup BSN 与静默选择 API，消费者无需直接引用功能 crate。
#[test]
fn radio_group_scene_api_is_usable() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryRadioGroup
        Children [(@WidgetryRadioOption Children [Text("Low")]), (@WidgetryRadioOption Children [Text("High")])]
    }).unwrap().id();
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    let children = app.world().get::<Children>(root).unwrap();
    assert!(app.world().get::<WidgetryRadioGroup>(root).is_some());
    assert!(
        app.world()
            .get::<WidgetryRadioOption>(children[1])
            .is_some()
    );
    assert!(app.world().get::<bevy::ui::Checked>(children[1]).is_some());
    assert!(app.world().get::<bevy::ui::Checked>(children[0]).is_none());
}

// Window plugin 继续为普通 Bevy 文本自动安装内建 fallback。
#[test]
fn window_plugin_installs_app_font_fallback() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<ButtonInput<MouseButton>>();
    app.add_plugins(WidgetryWindowPlugin);
    let entity = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    assert!(matches!(
        app.world().get::<TextFont>(entity).unwrap().font,
        FontSource::Handle(_)
    ));
}

// facade 的 TextField 可通过 BSN 构造；显式装配共享 focus plugin 不应重复注册或改变字体策略。
#[test]
fn text_field_scene_preserves_app_font_policy() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ));
    app.add_plugins((WidgetryFocusPlugin, WidgetryTextFieldPlugin));
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    app.update();
    assert!(app.world().get::<WidgetryTextField>(entity).is_some());
    assert!(
        app.world()
            .get::<bevy::text::EditableText>(entity)
            .is_some()
    );
    assert_eq!(
        app.world().get::<TextFont>(entity).unwrap().font,
        FontSource::default()
    );
}

// Button 不创建文本，独立注册时不需要 asset 设施，也不应改写调用方文本的默认字体。
#[test]
fn button_plugin_leaves_default_font_unchanged() {
    let mut app = App::new();
    app.add_plugins(WidgetryButtonPlugin);
    let entity = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(entity).unwrap().font,
        FontSource::default()
    );
}

// 从 facade 导入消费者需要的 type，验证重构后公开入口仍可构造。
#[test]
fn facade_public_types_are_usable() {
    let _ = WidgetryComboBox;
    let _ = WidgetryComboBoxPlugin;
    let props = WidgetryComboBoxProps::default();
    assert!(props.options.is_empty());
    let _ = WidgetryComboBoxOptionFactory::new(|| bsn_list![Text("Option")]);
    let _ = ForegroundColor(Color::WHITE);
}

// 同时装配多个 style plugin，验证共享 theme 设施不会重复注册且可使用外部 theme。
#[test]
fn style_theme_api_and_plugins_work_together() {
    let _: &ColorTheme = &DARK_THEME;
    assert_eq!(ThemeMode::Light.colors(), &LIGHT_THEME);
    let mut app = App::new();
    app.set_default_font(FontSource::Monospace);
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>();
    app.insert_resource(ThemeMode::Light).add_plugins((
        ThemePlugin,
        WidgetryButtonPlugin,
        WidgetryComboBoxPlugin,
    ));
    app.world_mut().trigger(ThemeChanged {
        mode: ThemeMode::Light,
    });
    app.update();
    assert_eq!(*app.world().resource::<ThemeMode>(), ThemeMode::Light);
}

// 消费者仅通过 facade 与 BSN 创建 icon，无需取得 AssetServer，运行期 component 仍可用于 query。
#[test]
fn icon_scene_api_is_usable() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        WidgetryIconPlugin,
    ));
    let _ = WidgetryIconProps::default();
    let plain = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! { @WidgetryIcon { @path: "some/icon.svg" } })
        .id();
    let configured = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! {
            @WidgetryIcon {
                @path: "some/icon.svg",
                @max_size: { Some(UVec2::new(24, 24)) },
                @color: { Some(Color::WHITE) },
            }
        })
        .id();
    app.world_mut().flush();
    for entity in [plain, configured] {
        assert!(app.world().get::<WidgetryIcon>(entity).is_some());
        assert!(app.world().get::<Node>(entity).is_some());
    }
}

// 从 facade 组合空 title bar 与主体 Scene，验证新的 Window 公开入口可直接用于 BSN。
#[test]
fn window_scene_api_is_usable() {
    let _ = WidgetryWindowPlugin;
    let config = WidgetryWindowControlsConfig::default();
    assert!(config.minimize_visible && config.maximize_visible);
    let _ = bsn! {
        widgetry_window(Entity::PLACEHOLDER, Entity::PLACEHOLDER, config, bsn_list![], bsn_list![(Text("Body"))])
    };
}

/// facade 提供 WidgetryMessageBox type 与 BSN function，消费者不需要直接依赖内部 crate。
#[test]
fn message_box_scene_api_is_usable() {
    let _ = WidgetryMessageBox;
    let _ = WidgetryMessageBoxPlugin;
    let _ = WidgetryMessageBoxResultEvent {
        entity: Entity::PLACEHOLDER,
        result: WidgetryMessageBoxResult::Ok,
    };
    let _ = bsn! {
        widgetry_message_box(Entity::PLACEHOLDER, "Confirm", WidgetryMessageBoxButtons::YesNoCancel, bsn_list![(Text("Save changes?"))])
    };
}

// 消费者仅通过 facade 创建完整 Button Scene，保留可 query 的身份与官方行为 component。
#[test]
fn button_scene_api_is_usable() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ));
    app.set_default_font(FontSource::Monospace);
    app.add_plugins(WidgetryButtonPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    app.update();
    assert!(app.world().get::<WidgetryButton>(entity).is_some());
    assert!(
        app.world()
            .get::<bevy::ui_widgets::Button>(entity)
            .is_some()
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        DARK_THEME.control_background
    );
}

// facade 提供完整 ComboBox BSN 入口与静默 selection API，plugin 不隐式改变调用方字体策略。
#[test]
fn combo_box_scene_api_is_usable_without_installing_font_fallback() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ));
    app.init_asset::<Image>()
        .add_plugins(WidgetryComboBoxPlugin);
    let font = app.world_mut().spawn(TextFont::default()).id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox { @options: {vec![
                WidgetryComboBoxOptionFactory::new(|| bsn_list![Node]),
                WidgetryComboBoxOptionFactory::new(|| bsn_list![Node]),
            ]} }
        })
        .unwrap()
        .id();
    WidgetryComboBox::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    assert!(app.world().get::<WidgetryComboBox>(root).is_some());
    assert_eq!(
        app.world().get::<TextFont>(font).unwrap().font,
        FontSource::default()
    );
}
