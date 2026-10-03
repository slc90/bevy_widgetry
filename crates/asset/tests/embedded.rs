//! State：embedded source 未注册/已注册、字体与 icon 未加载/已加载。
//! Stimuli：注册 WidgetryAssetPlugin，按 BuiltinFont/BuiltinIcon 标识读取内存 source。
//! Transitions：注册后每个标识取得对应 payload，字体可解析，SVG 路径互不混淆。
//! Invariants：读取不访问磁盘，所有内建标识均指向有效且对应的 asset。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::asset::io::AssetSourceId;
use bevy::prelude::*;
use bevy::tasks::block_on;
use bevy_widgetry_asset::{BuiltinFont, BuiltinIcon, WidgetryAssetPlugin};
use std::collections::HashSet;

#[test]
fn builtin_font_resolves_to_valid_embedded_font() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WidgetryAssetPlugin));
    let server = app.world().resource::<AssetServer>();
    let path = BuiltinFont::Default.path();
    let source = server.get_source(path.source()).unwrap();
    let bytes = block_on(async {
        let mut reader = source.reader().read(path.path()).await.unwrap();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.unwrap();
        bytes
    });
    assert!(bytes.starts_with(&[0, 1, 0, 0]));
    let font = Font::from_bytes(bytes);
    assert!(!font.data.is_empty());
}

#[test]
fn builtin_icons_resolve_to_registered_embedded_assets() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WidgetryAssetPlugin));
    let server = app.world().resource::<AssetServer>();
    let mut paths = HashSet::new();
    let mut contents = HashSet::new();
    for icon in [
        BuiltinIcon::CheckboxCheck,
        BuiltinIcon::CheckboxIndeterminate,
        BuiltinIcon::WindowClose,
        BuiltinIcon::WindowMaximize,
        BuiltinIcon::WindowMinimize,
        BuiltinIcon::WindowRestore,
        BuiltinIcon::ChevronDown,
        BuiltinIcon::ChevronUp,
        BuiltinIcon::TreeExpand,
        BuiltinIcon::TreeCollapse,
    ] {
        let path = icon.path();
        assert_eq!(path.source(), &AssetSourceId::from("embedded"));
        let source = server.get_source(path.source()).unwrap();
        let bytes = block_on(async {
            let mut reader = source.reader().read(path.path()).await.unwrap();
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        assert!(!bytes.is_empty(), "{icon:?} 的嵌入内容不能为空");
        assert!(paths.insert(path), "{icon:?} 的路径不能与其他图标重复");
        assert!(contents.insert(bytes), "{icon:?} 必须对应独立图标");
    }
}
