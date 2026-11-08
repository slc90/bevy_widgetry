#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

const ASYNC_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

use bevy::prelude::*;
use bevy::text::{EditableText, FontSource};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::{WidgetryAppExt, WidgetryFontPlugin};
use bevy_widgetry_test_utils::advance_until;
use std::time::Duration;

fn font_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::text::TextPlugin,
    ));
    app
}

fn with_emoji(app: &App, primary: FontSource) -> FontSource {
    let emoji = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Emoji.path());
    FontSource::list([primary, FontSource::Handle(emoji)])
}

#[test]
fn app_default_applies_without_widget_plugins() {
    let mut app = font_app();
    app.set_default_font(FontSource::monospace());
    let entity = app.world_mut().spawn(TextFont::from_font_size(23.0)).id();
    app.update();
    let font = app.world().get::<TextFont>(entity).unwrap();
    assert_eq!(font.font, with_emoji(&app, FontSource::monospace()));
    assert_eq!(font.font_size, bevy::text::FontSize::Px(23.0));
}

#[test]
fn explicit_font_sources_are_preserved() {
    let mut app = font_app();
    app.set_default_font(FontSource::sans_serif());
    let assets = Assets::<Font>::default();
    let handle = assets.reserve_handle();
    for source in [
        FontSource::monospace(),
        FontSource::Handle(handle),
        FontSource::list([FontSource::serif(), FontSource::sans_serif()]),
    ] {
        let entity = app.world_mut().spawn(TextFont::from(source.clone())).id();
        app.update();
        assert_eq!(app.world().get::<TextFont>(entity).unwrap().font, source);
    }
}

#[test]
fn fallback_only_processes_new_components() {
    let mut app = font_app();
    app.set_default_font(FontSource::monospace());
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
        with_emoji(&app, FontSource::monospace())
    );
}

#[test]
fn all_text_kinds_use_app_fallback() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        bevy::text::TextPlugin,
    ))
    .set_default_font(FontSource::monospace());
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
            with_emoji(&app, FontSource::monospace())
        );
    }
}

#[test]
fn initialization_order_preserves_user_configuration() {
    for configure_first in [true, false] {
        let mut app = font_app();
        if configure_first {
            app.set_default_font(FontSource::serif());
        } else {
            app.add_plugins(WidgetryFontPlugin);
        }
        app.set_default_font(FontSource::monospace());
        let entity = app.world_mut().spawn(TextFont::default()).id();
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(entity).unwrap().font,
            with_emoji(&app, FontSource::monospace())
        );
        assert_eq!(app.get_added_plugins::<WidgetryFontPlugin>().len(), 1);
    }
}

#[test]
fn builtin_default_loads_through_bevy_asset_server() {
    let mut app = font_app();
    app.add_plugins(WidgetryFontPlugin);
    let entity = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    let FontSource::List(sources) = app.world().get::<TextFont>(entity).unwrap().font.clone()
    else {
        panic!("默认字体必须包含主字体与 Emoji 回退");
    };
    assert_eq!(sources.len(), 2);
    for (source, builtin) in sources
        .iter()
        .zip([BuiltinFont::Default, BuiltinFont::Emoji])
    {
        let FontSource::Handle(handle) = source else {
            panic!("内建字体必须使用资源句柄")
        };
        assert_eq!(handle.path(), Some(&builtin.path()));
        advance_until(
            &mut app,
            ASYNC_WAIT_TIMEOUT,
            &format!("默认 Font {:?}", handle.id()),
            |world| world.resource::<Assets<Font>>().contains(handle.id()),
        )
        .expect("内建字体未能在期限内加载");
    }
    assert_eq!(app.get_added_plugins::<WidgetryAssetPlugin>().len(), 1);
    let another = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(another).unwrap().font,
        FontSource::List(sources)
    );
    assert_eq!(app.world().resource::<Assets<Font>>().len(), 2);
}

#[test]
fn replacing_primary_chain_preserves_existing_text_and_shared_emoji() {
    let mut app = font_app();
    app.set_default_font(FontSource::monospace());
    let first = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    let previous = app.world().get::<TextFont>(first).unwrap().font.clone();
    let primary = FontSource::list([FontSource::serif(), FontSource::sans_serif()]);
    app.set_default_font(primary.clone());
    let second = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    assert_eq!(app.world().get::<TextFont>(first).unwrap().font, previous);
    let expected = with_emoji(&app, primary);
    let actual = &app.world().get::<TextFont>(second).unwrap().font;
    assert_eq!(actual.flatten(), expected.flatten());
    let old_sources = previous.flatten();
    let new_sources = actual.flatten();
    assert_eq!(old_sources.last(), new_sources.last());
    assert_eq!(app.get_added_plugins::<WidgetryAssetPlugin>().len(), 1);
}
