use bevy::prelude::*;
use bevy::text::{EditableText, FontSource};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::{WidgetryAppExt, WidgetryFontPlugin};
use std::time::{Duration, Instant};

// 不使用任何控件插件时，初始化 API 仍为普通文本提供默认字体且保留字号。
#[test]
fn app_default_applies_without_widget_plugins() {
    let mut app = App::new();
    app.set_default_font(FontSource::Monospace);
    let entity = app.world_mut().spawn(TextFont::from_font_size(23.0)).id();
    app.update();
    let font = app.world().get::<TextFont>(entity).unwrap();
    assert_eq!(font.font, FontSource::Monospace);
    assert_eq!(font.font_size, bevy::text::FontSize::Px(23.0));
}

// 显式指定的通用字体族和资产句柄均不被 fallback 覆盖。
#[test]
fn explicit_font_sources_are_preserved() {
    let mut app = App::new();
    app.set_default_font(FontSource::SansSerif);
    let assets = Assets::<Font>::default();
    let handle = assets.reserve_handle();
    for source in [FontSource::Monospace, FontSource::Handle(handle)] {
        let entity = app.world_mut().spawn(TextFont::from(source.clone())).id();
        app.update();
        assert_eq!(app.world().get::<TextFont>(entity).unwrap().font, source);
    }
}

// 新文本跨帧继续使用 fallback，已处理文本改回哨兵后不会被每帧重写。
#[test]
fn fallback_only_processes_new_components() {
    let mut app = App::new();
    app.set_default_font(FontSource::Monospace);
    let old = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    app.world_mut().get_mut::<TextFont>(old).unwrap().font = FontSource::default();
    let new = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(old).unwrap().font,
        FontSource::default()
    );
    assert_eq!(
        app.world().get::<TextFont>(new).unwrap().font,
        FontSource::Monospace
    );
}

// UI、span、二维文字和可编辑文字均通过自身的 TextFont 接入 App fallback。
#[test]
fn all_text_kinds_use_app_fallback() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .set_default_font(FontSource::Monospace);
    let ui = app
        .world_mut()
        .spawn_scene(bsn! { Text("中文 UI") })
        .unwrap()
        .id();
    let span = app
        .world_mut()
        .spawn_scene(bsn! { TextSpan("中文 span") })
        .unwrap()
        .id();
    let text2d = app
        .world_mut()
        .spawn_scene(bsn! { Text2d("中文 2d") })
        .unwrap()
        .id();
    let editable = app
        .world_mut()
        .spawn_scene(bsn! { EditableText::default() })
        .unwrap()
        .id();
    app.update();
    for entity in [ui, span, text2d, editable] {
        assert_eq!(
            app.world().get::<TextFont>(entity).unwrap().font,
            FontSource::Monospace
        );
    }
}

// 用户配置在共享插件之前或之后设置均生效，多次配置保留最终值且不重复注册。
#[test]
fn initialization_order_preserves_user_configuration() {
    for configure_first in [true, false] {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Font>();
        if configure_first {
            app.set_default_font(FontSource::Serif);
        } else {
            app.add_plugins(WidgetryFontPlugin);
        }
        app.set_default_font(FontSource::Monospace);
        let entity = app.world_mut().spawn(TextFont::default()).id();
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(entity).unwrap().font,
            FontSource::Monospace
        );
        assert_eq!(app.get_added_plugins::<WidgetryFontPlugin>().len(), 1);
    }
}

// 默认配置经真实 Bevy FontLoader 加载内建字体，且兼容资源插件已注册的顺序。
#[test]
fn builtin_default_loads_through_bevy_asset_server() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::text::TextPlugin,
        WidgetryAssetPlugin,
        WidgetryFontPlugin,
    ));
    let entity = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    let FontSource::Handle(handle) = app.world().get::<TextFont>(entity).unwrap().font.clone()
    else {
        panic!("内建默认字体必须使用字体资产句柄");
    };
    assert_eq!(handle.path(), Some(&BuiltinFont::Default.path()));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !app.world().resource::<Assets<Font>>().contains(handle.id()) {
        assert!(Instant::now() < deadline, "内建字体未能在期限内加载");
        std::thread::yield_now();
        app.update();
    }
    assert_eq!(app.get_added_plugins::<WidgetryAssetPlugin>().len(), 1);
}
