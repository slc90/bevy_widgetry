// 测试及其 helper 使用断言和 expect 验证 contract；生产代码仍禁止主动 panic。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

//! TextField 消费场景共享已加载字体的 fixture；UI/input/camera 装配仍由 test_utils 负责。

use bevy::prelude::*;
use bevy::text::FontSource;
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_test_utils::{advance_until, text_edit_app};
use std::time::Duration;

/// 在创建 EditableText 之前预加载内建字体，selection 不依赖系统 generic font 的可用性。
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
