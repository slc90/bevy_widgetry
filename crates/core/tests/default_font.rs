//! State：App fallback、TextFont 默认/显式与字体未加载/已加载。
//! Stimuli：set_default_font、Plugin 注册、新 TextFont 和首次 update。
//! Guards：只有新增且等于 Bevy 默认 sentinel 的 TextFont 使用 fallback。
//! Transitions：默认 font 取得配置或内建 fallback，显式 font 保持。
//! Invariants：调用顺序不覆盖已有配置，内建字体可加载，运行期配置不重写既有文本。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::text::{EditableText, FontSource};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::{WidgetryAppExt, WidgetryFontPlugin};
use bevy_widgetry_test_utils::advance_until;
use std::time::Duration;

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
    advance_until(
        &mut app,
        Duration::from_secs(10),
        &format!("默认 Font {:?}", handle.id()),
        |world| world.resource::<Assets<Font>>().contains(handle.id()),
    )
    .expect("内建字体未能在期限内加载");
    assert_eq!(app.get_added_plugins::<WidgetryAssetPlugin>().len(), 1);
}
