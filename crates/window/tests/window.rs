use bevy::prelude::*;
use bevy_widgetry_core::{ThemeChanged, ThemeMode};
use bevy_widgetry_window::{WindowControlsConfig, WindowPlugin, window};

/// 动态组合两个独立窗口，验证显式相机绑定和两个内容插槽。
#[test]
fn scenes_bind_camera_and_place_content_in_distinct_slots() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WindowPlugin));
    app.init_asset::<Image>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_asset::<bevy::scene::ScenePatch>();
    for _ in 0..2 {
        let target = app.world_mut().spawn(Window::default()).id();
        let camera = app.world_mut().spawn(Camera2d).id();
        let root = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                window(target, camera, WindowControlsConfig::default(),
                    bsn_list![(Name("TitleSlotChild"))],
                    bsn_list![(Name("ContentSlotChild"))])
            })
            .id();
        app.update();
        assert_eq!(app.world().get::<UiTargetCamera>(root).unwrap().0, camera);
        assert!(!app.world().get::<Window>(target).unwrap().decorations);
        let children = app.world().get::<Children>(root).unwrap();
        assert_eq!(children.len(), 3);
        let content = children[1];
        let title_bar = children[0];
        let title = app.world().get::<Children>(title_bar).unwrap()[1];
        for (slot, expected) in [(title, "TitleSlotChild"), (content, "ContentSlotChild")] {
            let child = app.world().get::<Children>(slot).unwrap()[0];
            assert_eq!(app.world().get::<Name>(child).unwrap().as_str(), expected);
        }
    }
}

/// 重复绑定必须仅回收新树，关闭消息仅清理对应 UI 且保留相机。
#[test]
fn duplicate_and_closed_windows_preserve_other_owners() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WindowPlugin));
    app.init_asset::<Image>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_asset::<bevy::scene::ScenePatch>();
    let target = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn(Camera2d).id();
    let mut roots = Vec::new();
    for _ in 0..2 {
        roots.push(app.world_mut().commands().spawn_scene(bsn! {
            window(target, camera, WindowControlsConfig::default(), bsn_list![], bsn_list![(Text("Body"))])
        }).id());
    }
    app.update();
    assert!(app.world().get_entity(roots[0]).is_ok());
    assert!(app.world().get_entity(roots[1]).is_err());
    let descendants: Vec<_> = app
        .world()
        .get::<Children>(roots[0])
        .unwrap()
        .iter()
        .collect();
    app.world_mut()
        .write_message(bevy::window::WindowClosed { window: target });
    app.update();
    assert!(app.world().get_entity(roots[0]).is_err());
    assert!(
        descendants
            .iter()
            .all(|&entity| app.world().get_entity(entity).is_err())
    );
    assert!(app.world().get_entity(camera).is_ok());
}

/// 首次创建读取预设主题，主题事件同时更新外框、背景和标题栏分隔线。
#[test]
fn theme_colors_initialize_and_refresh_together() {
    let mut app = App::new();
    app.insert_resource(ThemeMode::Light);
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WindowPlugin));
    app.init_asset::<Image>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_asset::<bevy::scene::ScenePatch>();
    let target = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn(Camera2d).id();
    let root = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! {
            window(target, camera, WindowControlsConfig::default(), bsn_list![], bsn_list![])
        })
        .id();
    app.update();
    let title = app.world().get::<Children>(root).unwrap()[0];
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        if mode == ThemeMode::Dark {
            *app.world_mut().resource_mut::<ThemeMode>() = mode;
            app.world_mut().trigger(ThemeChanged { mode });
        }
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            mode.colors().window_background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(mode.colors().window_border)
        );
        assert_eq!(
            *app.world().get::<BorderColor>(title).unwrap(),
            BorderColor::all(mode.colors().title_bar_border)
        );
    }
}

/// 无效或缺失组件的绑定必须完整回收 UI，不能留下插槽内容或影响有效原生窗口。
#[test]
fn invalid_bindings_remove_the_entire_scene() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WindowPlugin));
    app.init_asset::<Image>();
    app.init_asset::<bevy::scene::ScenePatch>();
    app.init_resource::<ButtonInput<MouseButton>>();
    let target = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn(Camera2d).id();
    let empty = app.world_mut().spawn_empty().id();
    for (target_window, target_camera) in [
        (Entity::PLACEHOLDER, camera),
        (empty, camera),
        (target, Entity::PLACEHOLDER),
        (target, empty),
    ] {
        let root = app.world_mut().commands().spawn_scene(bsn! {
            window(target_window, target_camera, WindowControlsConfig::default(), bsn_list![(Name("InvalidTitle"))], bsn_list![(Name("InvalidContent"))])
        }).id();
        app.world_mut().flush();
        let mut descendants = Vec::new();
        let mut pending = vec![root];
        while let Some(entity) = pending.pop() {
            if let Some(children) = app.world().get::<Children>(entity) {
                for child in children.iter() {
                    descendants.push(child);
                    pending.push(child);
                }
            }
        }
        app.update();
        assert!(app.world().get_entity(root).is_err());
        assert!(!descendants.is_empty());
        assert!(
            descendants
                .iter()
                .all(|&entity| app.world().get_entity(entity).is_err())
        );
        assert!(app.world().get::<Window>(target).unwrap().decorations);
    }
}
