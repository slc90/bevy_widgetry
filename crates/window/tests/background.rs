//! State：背景为 Theme/Image，入口为 borrowed/owned，theme 为 Dark/Light，asset 为等待/就绪，layout 为有效/无效尺寸，opacity 为 0/0.5/1。
//! Stimuli：公开 Scene 构造、WidgetryThemeChanged、asset 就绪、ComputedNode 变化与实际 UiPlugin layout。
//! Guards：Cover 仅在图片就绪且尺寸有效时更新。
//! Invariants：Theme 与 Image 互斥，Image 只改变 alpha，背景直接挂在 root。
//! 空闲帧不改 ImageNode，Cover 在当前帧 layout 后按中心裁剪，尺寸恢复后重新同步。
//! Couplings：Image 不跟随 theme，但 window/title bar border 继续更新。
//! borrowed/owned 共用语义，asset 首次就绪可在没有新 layout 变化时完成裁剪。
//! 内容 host 的局部 Disabled 独立选择 frame 前景，覆盖、Theme 与恢复均遵守该状态。

// 测试断言必须在 contract 不满足时失败，因此只在本测试文件允许生产 panic lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::VisualBox;
use bevy_widgetry_theme::WidgetryThemeMode;

use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowImageBackground,
    WidgetryWindowImageMode, WidgetryWindowPlugin, owned_widgetry_window, prepare_native_window,
    widgetry_window,
};

fn window(app: &mut App, owned: bool, background: WidgetryWindowBackground) -> Entity {
    if owned {
        app.world_mut().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), background, Default::default(),  bsn_list![], bsn_list![])
        }).unwrap().id()
    } else {
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), background, Default::default(),  bsn_list![], bsn_list![])
        }).unwrap().id()
    }
}

#[test]
fn locally_disabled_content_uses_disabled_frame_foreground_and_recovers() {
    use bevy::ui::InteractionDisabled;
    use bevy_widgetry_core::text::WidgetryText;
    use bevy_widgetry_window::WidgetryWindowColorOverrides;
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    let mut colors = WidgetryWindowColorOverrides::default();
    colors.frame.normal.foreground = Some(Color::linear_rgb(0.9, 0.1, 0.2));
    colors.frame.disabled.foreground = Some(Color::linear_rgb(0.1, 0.8, 0.3));
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(),
                WidgetryWindowBackground::Theme, colors.clone(), bsn_list![],
                bsn_list![(Text("body") WidgetryText)])
        })
        .unwrap()
        .id();
    app.update();
    let text = app
        .world_mut()
        .query_filtered::<Entity, With<Text>>()
        .single(app.world())
        .unwrap();
    let content = app.world().get::<ChildOf>(text).unwrap().parent();
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.frame.normal.foreground.unwrap()
    );
    app.world_mut()
        .entity_mut(content)
        .insert(InteractionDisabled);
    app.update();
    assert!(app.world().get::<InteractionDisabled>(root).is_none());
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.frame.disabled.foreground.unwrap()
    );
    WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.frame.disabled.foreground.unwrap()
    );
    WidgetryWindowColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        WidgetryThemeMode::Light
            .colors()
            .window
            .frame
            .disabled
            .foreground
    );
    app.world_mut()
        .entity_mut(content)
        .remove::<InteractionDisabled>();
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        WidgetryThemeMode::Light
            .colors()
            .window
            .frame
            .normal
            .foreground
    );
}

#[test]
fn image_tint_multiplies_configured_opacity_without_compounding() {
    use bevy_widgetry_window::WidgetryWindowColorOverrides;
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    let handle = app.world().resource::<Assets<Image>>().reserve_handle();
    let root = window(
        &mut app,
        true,
        WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
            image: handle.clone(),
            mode: WidgetryWindowImageMode::Stretch,
            opacity: 0.5,
        }),
    );
    let colors = WidgetryWindowColorOverrides {
        image_tint: Some(Color::srgba(0.2, 0.4, 0.6, 0.4)),
        ..default()
    };
    WidgetryWindowColorOverrides::set_in_world(app.world_mut(), root, colors).unwrap();
    for mode in [
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::Light,
        WidgetryThemeMode::Dark,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        app.update();
        app.update();
        let image = app.world().get::<ImageNode>(root).unwrap();
        assert_eq!(image.image, handle);
        assert_eq!(image.color, Color::srgba(0.2, 0.4, 0.6, 0.2));
        assert!(app.world().get::<BackgroundColor>(root).is_none());
    }
    WidgetryWindowColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(root).unwrap().color.alpha(),
        0.5
    );
}

#[test]
fn theme_and_image_backgrounds_are_exclusive_on_both_entry_points() {
    for owned in [false, true] {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let theme = window(&mut app, owned, WidgetryWindowBackground::Theme);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .reserve_handle();
        let image = window(
            &mut app,
            owned,
            WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
                image: handle.clone(),
                mode: WidgetryWindowImageMode::Stretch,
                opacity: 0.5,
            }),
        );
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(theme).unwrap().0,
            app.world()
                .resource::<WidgetryThemeMode>()
                .colors()
                .window
                .frame
                .normal
                .background
        );
        assert!(app.world().get::<ImageNode>(theme).is_none());
        assert!(app.world().get::<BackgroundColor>(image).is_none());
        let node = app.world().get::<ImageNode>(image).unwrap();
        assert_eq!(node.image, handle);
        assert_eq!(node.image_mode, NodeImageMode::Stretch);
        assert_eq!(node.visual_box, VisualBox::BorderBox);
        assert_eq!(node.rect, None);
        assert_eq!(node.color, Color::srgba(1.0, 1.0, 1.0, 0.5));
        assert_eq!(app.world().get::<Children>(theme).unwrap().len(), 3);
        assert_eq!(app.world().get::<Children>(image).unwrap().len(), 3);
        let mode = WidgetryThemeMode::Light;
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).expect("theme switch succeeds");
        assert_eq!(
            app.world().get::<BackgroundColor>(theme).unwrap().0,
            mode.colors().window.frame.normal.background
        );
        assert!(app.world().get::<BackgroundColor>(image).is_none());
        assert_eq!(
            app.world().get::<ImageNode>(image).unwrap().color,
            Color::srgba(1.0, 1.0, 1.0, 0.5)
        );
        for root in [theme, image] {
            assert_eq!(
                *app.world().get::<BorderColor>(root).unwrap(),
                BorderColor::all(mode.colors().window.frame.normal.border)
            );
            let bar = app.world().get::<Children>(root).unwrap()[0];
            assert_eq!(
                *app.world().get::<BorderColor>(bar).unwrap(),
                BorderColor::all(mode.colors().window.title_bar.normal.border)
            );
        }
    }
}

#[test]
fn image_opacity_preserves_geometry_and_theme_independence_on_both_entry_points() {
    for owned in [false, true] {
        for mode in [
            WidgetryWindowImageMode::Stretch,
            WidgetryWindowImageMode::Cover,
        ] {
            for opacity in [0.0, 0.5, 1.0] {
                let mut app = scene_app();
                app.add_plugins(WidgetryWindowPlugin);
                let image = app
                    .world_mut()
                    .resource_mut::<Assets<Image>>()
                    .add(Image::new_fill(
                        Extent3d {
                            width: 24,
                            height: 16,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        &[255; 4],
                        TextureFormat::Rgba8UnormSrgb,
                        RenderAssetUsages::default(),
                    ));
                let root = window(
                    &mut app,
                    owned,
                    WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
                        image: image.clone(),
                        mode,
                        opacity,
                    }),
                );
                app.update();
                assert_eq!(app.world().get::<ImageNode>(root).unwrap().rect, None);
                for (size, cover) in [
                    (Vec2::new(32.0, 16.0), Rect::new(0.0, 2.0, 24.0, 14.0)),
                    (Vec2::new(16.0, 32.0), Rect::new(8.0, 0.0, 16.0, 16.0)),
                ] {
                    app.world_mut().get_mut::<ComputedNode>(root).unwrap().size = size;
                    app.update();
                    let expected_rect = (mode == WidgetryWindowImageMode::Cover).then_some(cover);
                    for theme in [WidgetryThemeMode::Light, WidgetryThemeMode::Dark] {
                        WidgetryThemeMode::set_in_world(app.world_mut(), theme)
                            .expect("theme switch succeeds");
                        app.update();
                        let node = app.world().get::<ImageNode>(root).unwrap();
                        assert_eq!(node.image, image);
                        assert_eq!(node.image_mode, NodeImageMode::Stretch);
                        assert_eq!(node.visual_box, VisualBox::BorderBox);
                        assert_eq!(node.rect, expected_rect);
                        assert_eq!(node.color, Color::srgba(1.0, 1.0, 1.0, opacity));
                        assert!(app.world().get::<BackgroundColor>(root).is_none());
                        assert_eq!(
                            *app.world().get::<BorderColor>(root).unwrap(),
                            BorderColor::all(theme.colors().window.frame.normal.border)
                        );
                        let bar = app.world().get::<Children>(root).unwrap()[0];
                        assert_eq!(
                            *app.world().get::<BorderColor>(bar).unwrap(),
                            BorderColor::all(theme.colors().window.title_bar.normal.border)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cover_waits_for_asset_then_tracks_layout_without_rewriting_idle_images() {
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .reserve_handle();
    let root = window(
        &mut app,
        true,
        WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
            image: handle.clone(),
            mode: WidgetryWindowImageMode::Cover,
            opacity: 0.5,
        }),
    );
    app.update();
    app.world_mut().get_mut::<ComputedNode>(root).unwrap().size = Vec2::new(1920.0, 1080.0);
    app.update();
    assert_eq!(app.world().get::<ImageNode>(root).unwrap().rect, None);
    assert!(app.world().get::<BackgroundColor>(root).is_none());
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(
            &handle,
            Image::new_fill(
                Extent3d {
                    width: 1000,
                    height: 800,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[255; 4],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ),
        )
        .unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(root).unwrap().rect,
        Some(Rect::new(0.0, 118.75, 1000.0, 681.25))
    );
    let changed = app
        .world()
        .entity(root)
        .get_ref::<ImageNode>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(root)
            .get_ref::<ImageNode>()
            .unwrap()
            .last_changed(),
        changed
    );
    app.world_mut().get_mut::<ComputedNode>(root).unwrap().size = Vec2::new(400.0, 800.0);
    app.update();
    let node = app.world().get::<ImageNode>(root).unwrap();
    assert_eq!(node.rect, Some(Rect::new(300.0, 0.0, 700.0, 800.0)));
    assert_eq!(node.image, handle);
    assert_eq!(node.color, Color::srgba(1.0, 1.0, 1.0, 0.5));
    for size in [
        Vec2::ZERO,
        Vec2::new(0.0, 800.0),
        Vec2::new(400.0, 0.0),
        Vec2::new(f32::NAN, 800.0),
    ] {
        app.world_mut().get_mut::<ComputedNode>(root).unwrap().size = size;
        app.update();
        assert_eq!(
            app.world().get::<ImageNode>(root).unwrap().rect,
            Some(Rect::new(300.0, 0.0, 700.0, 800.0))
        );
    }
    app.world_mut().get_mut::<ComputedNode>(root).unwrap().size = Vec2::new(1000.0, 800.0);
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(root).unwrap().rect,
        Some(Rect::new(0.0, 0.0, 1000.0, 800.0))
    );
}

#[test]
fn cover_reads_the_current_frames_real_ui_layout() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins(WidgetryWindowPlugin);
    let target = app
        .world_mut()
        .spawn(prepare_native_window(Window::default()))
        .id();
    let camera = spawn_ui_camera(&mut app, UVec2::new(1920, 1080), 1.0);
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_fill(
            Extent3d {
                width: 1000,
                height: 800,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255; 4],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
    let background = WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
        image,
        mode: WidgetryWindowImageMode::Cover,
        opacity: 1.0,
    });
    let root = app.world_mut().spawn_scene(bsn! {
        widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), background, Default::default(),  bsn_list![], bsn_list![])
    }).unwrap().id();
    app.update();
    assert_eq!(
        app.world().get::<ComputedNode>(root).unwrap().size(),
        Vec2::new(1920.0, 1080.0)
    );
    assert_eq!(
        app.world().get::<ImageNode>(root).unwrap().rect,
        Some(Rect::new(0.0, 118.75, 1000.0, 681.25))
    );
    app.world_mut()
        .get_mut::<Camera>(camera)
        .unwrap()
        .computed
        .target_info
        .as_mut()
        .unwrap()
        .physical_size = UVec2::new(400, 800);
    app.update();
    assert_eq!(
        app.world().get::<ComputedNode>(root).unwrap().size(),
        Vec2::new(400.0, 800.0)
    );
    assert_eq!(
        app.world().get::<ImageNode>(root).unwrap().rect,
        Some(Rect::new(300.0, 0.0, 700.0, 800.0))
    );
}
