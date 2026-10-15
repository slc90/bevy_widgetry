#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::text::FontSource;
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_test_utils::{advance_until, text_edit_app};
use std::time::Duration;

pub fn editing_app() -> App {
    let mut app = text_edit_app();
    app.add_plugins(WidgetryAssetPlugin);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    advance_until(
        &mut app,
        Duration::from_secs(10),
        "TextField 内建字体",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .expect("字体应在期限内就绪");
    app.set_default_font(FontSource::Handle(font));
    app
}
