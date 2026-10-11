//! State：embedded source 未注册/已注册、字体、icon 与 cursor 图集未加载/已加载。
//! Stimuli：注册 WidgetryAssetPlugin，按内建标识读取内存 source 并解码。
//! Transitions：取得对应 payload，字体可解析、SVG 独立、cursor 可解码为 RGBA。
//! Invariants：不访问磁盘；44 款静态图与12帧动画独立，Pointer 静态格等于动画首帧。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::asset::io::AssetSourceId;
use bevy::prelude::*;
use bevy::tasks::block_on;
use bevy_widgetry_asset::{BuiltinCursorImage, BuiltinFont, BuiltinIcon, WidgetryAssetPlugin};
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

#[test]
fn pointer_cursor_assets_are_rgba_and_static_matches_atlas_frame_zero() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WidgetryAssetPlugin));
    let server = app.world().resource::<AssetServer>();
    let images: Vec<_> = [
        BuiltinCursorImage::KamuriPointerAtlas,
        BuiltinCursorImage::KamuriStaticAtlas,
    ]
    .into_iter()
    .map(|id| {
        let path = id.path();
        let source = server.get_source(path.source()).unwrap();
        let bytes = block_on(async {
            let mut reader = source.reader().read(path.path()).await.unwrap();
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!(image.color(), image::ColorType::Rgba8);
        image.to_rgba8()
    })
    .collect();
    assert_eq!(images[0].dimensions(), (256, 192));
    let animation_layout = BuiltinCursorImage::KamuriPointerAtlas.atlas_layout();
    assert_eq!(animation_layout.textures.len(), 12);
    assert!(
        animation_layout
            .textures
            .iter()
            .all(|rect| rect.size() == UVec2::splat(64))
    );
    assert_eq!(images[1].dimensions(), (768, 576));
    let static_layout = BuiltinCursorImage::KamuriStaticAtlas.atlas_layout();
    assert_eq!(static_layout.textures.len(), 44);
    for (index, rect) in static_layout.textures.iter().enumerate() {
        assert_eq!(rect.size(), UVec2::splat(if index == 1 { 64 } else { 48 }));
        if index != 1 {
            let cell = image::imageops::crop_imm(
                &images[1],
                index as u32 % 8 * 96,
                index as u32 / 8 * 96,
                96,
                96,
            )
            .to_image();
            assert!(
                cell.enumerate_pixels()
                    .filter(|(x, y, _)| *x >= 48 || *y >= 48)
                    .all(|(_, _, pixel)| pixel[3] == 0)
            );
        }
    }
    assert_eq!(
        image::imageops::crop_imm(&images[0], 0, 0, 64, 64).to_image(),
        image::imageops::crop_imm(&images[1], 96, 0, 64, 64).to_image()
    );
    let frames: HashSet<_> = (0..12)
        .map(|i| {
            image::imageops::crop_imm(&images[0], i % 4 * 64, i / 4 * 64, 64, 64)
                .to_image()
                .into_raw()
        })
        .collect();
    assert_eq!(frames.len(), 12);
    assert_eq!(images[1].get_pixel(0, 0)[3], 0);
    let sprites: HashSet<_> = (0..44)
        .map(|i| {
            let sprite =
                image::imageops::crop_imm(&images[1], i % 8 * 96, i / 8 * 96, 96, 96).to_image();
            assert!(sprite.pixels().any(|pixel| pixel[3] == 0));
            assert!(sprite.pixels().any(|pixel| pixel[3] == 255));
            sprite.into_raw()
        })
        .collect();
    assert_eq!(sprites.len(), 44, "每个语义必须有独立的透明 sprite");
}
