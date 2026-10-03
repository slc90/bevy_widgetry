//! State：背景为 Theme/Image，入口为 borrowed/owned，theme 为 Dark/Light。
//! Stimuli：公开 Scene 构造与 ThemeChanged。
//! Invariants：Theme 与 Image 互斥，Image 只改变 alpha，背景直接挂在 root。
//! Couplings：Image 不跟随 theme，但 window/title bar border 继续更新；borrowed/owned 共用语义。

// 测试断言必须在 contract 不满足时失败，因此只在本测试文件允许生产 panic lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::VisualBox;
use bevy_widgetry_core::{ThemeChanged, ThemeMode};
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowImageBackground,
    WidgetryWindowImageMode, WidgetryWindowPlugin, owned_widgetry_window, prepare_native_window,
    widgetry_window,
};

fn window(app: &mut App, owned: bool, background: WidgetryWindowBackground) -> Entity {
    if owned {
        app.world_mut().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), background, bsn_list![], bsn_list![])
        }).unwrap().id()
    } else {
        let target = app
            .world_mut()
            .spawn(prepare_native_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        app.world_mut().spawn_scene(bsn! {
            widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), background, bsn_list![], bsn_list![])
        }).unwrap().id()
    }
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
                .resource::<ThemeMode>()
                .colors()
                .window_background
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
        let mode = ThemeMode::Light;
        app.world_mut().trigger(ThemeChanged { mode });
        assert_eq!(
            app.world().get::<BackgroundColor>(theme).unwrap().0,
            mode.colors().window_background
        );
        assert!(app.world().get::<BackgroundColor>(image).is_none());
        assert_eq!(
            app.world().get::<ImageNode>(image).unwrap().color,
            Color::srgba(1.0, 1.0, 1.0, 0.5)
        );
        for root in [theme, image] {
            assert_eq!(
                *app.world().get::<BorderColor>(root).unwrap(),
                BorderColor::all(mode.colors().window_border)
            );
            let bar = app.world().get::<Children>(root).unwrap()[0];
            assert_eq!(
                *app.world().get::<BorderColor>(bar).unwrap(),
                BorderColor::all(mode.colors().title_bar_border)
            );
        }
    }
}
