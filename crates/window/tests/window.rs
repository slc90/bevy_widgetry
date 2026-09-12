use bevy::{
    camera::{RenderTarget, Viewport},
    prelude::*,
    window::{CompositeAlphaMode, WindowRef},
};
use bevy_widgetry_core::{ThemeChanged, ThemeMode};
use bevy_widgetry_window::{WindowControlsConfig, WindowPlugin, widgetry_window, window};

/// 任意创建期透明与装饰组合都归一化，同时保留调用方的标题和尺寸。
#[test]
fn widgetry_window_prepares_native_creation_properties() {
    for transparent in [false, true] {
        for decorations in [false, true] {
            let configured = widgetry_window(Window {
                transparent,
                decorations,
                title: "Custom window".into(),
                resolution: (640, 400).into(),
                ..default()
            });
            assert!(configured.transparent);
            assert!(!configured.decorations);
            assert_eq!(
                configured.composite_alpha_mode,
                CompositeAlphaMode::PreMultiplied
            );
            assert_eq!(configured.title, "Custom window");
            assert_eq!(configured.resolution.width(), 640.0);
            assert_eq!(configured.resolution.height(), 400.0);
        }
    }
}

/// 动态组合两个独立窗口，验证显式相机绑定和两个内容插槽。
#[test]
fn scenes_bind_camera_and_place_content_in_distinct_slots() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WindowPlugin));
    app.init_asset::<Image>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_asset::<bevy::scene::ScenePatch>();
    for _ in 0..2 {
        let target = app
            .world_mut()
            .spawn(widgetry_window(Window::default()))
            .id();
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
    let target = app
        .world_mut()
        .spawn(widgetry_window(Window::default()))
        .id();
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

/// 两个不同原生窗口各绑定独立 UI 树；关闭 A 必须递归清理 A、完整保留 B，且不回收任一相机。
#[test]
fn closing_one_window_preserves_the_other_tree_and_both_cameras() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WindowPlugin));
    app.init_asset::<Image>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_asset::<bevy::scene::ScenePatch>();
    let mut windows = Vec::new();
    for _ in 0..2 {
        let target = app
            .world_mut()
            .spawn(widgetry_window(Window::default()))
            .id();
        let camera = app.world_mut().spawn(Camera2d).id();
        let root = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                window(target, camera, WindowControlsConfig::default(),
                    bsn_list![(Name("TitleSlotChild"))],
                    bsn_list![(Name("ContentSlotChild") Children [(Name("NestedContent"))])])
            })
            .id();
        windows.push((target, camera, root));
    }
    app.update();
    let trees: Vec<Vec<Entity>> = windows
        .iter()
        .map(|&(_, camera, root)| {
            assert_eq!(app.world().get::<UiTargetCamera>(root).unwrap().0, camera);
            let mut tree = vec![root];
            let mut pending = vec![root];
            while let Some(entity) = pending.pop() {
                if let Some(children) = app.world().get::<Children>(entity) {
                    for child in children.iter() {
                        tree.push(child);
                        pending.push(child);
                    }
                }
            }
            assert!(tree.len() > app.world().get::<Children>(root).unwrap().len() + 1);
            assert!(
                tree.iter()
                    .all(|&entity| app.world().get_entity(entity).is_ok())
            );
            tree
        })
        .collect();

    app.world_mut().write_message(bevy::window::WindowClosed {
        window: windows[0].0,
    });
    app.update();

    for &entity in &trees[0] {
        assert!(
            app.world().get_entity(entity).is_err(),
            "A 的实体 {entity:?} 未清理"
        );
    }
    for &entity in &trees[1] {
        assert!(
            app.world().get_entity(entity).is_ok(),
            "B 的实体 {entity:?} 被误清理"
        );
    }
    for &(_, camera, _) in &windows {
        assert!(app.world().get::<Camera>(camera).is_some());
    }
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
    let target = app
        .world_mut()
        .spawn(widgetry_window(Window::default()))
        .id();
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
    let target = app
        .world_mut()
        .spawn(widgetry_window(Window::default()))
        .id();
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
        assert!(!app.world().get::<Window>(target).unwrap().decorations);
    }
}

/// 专用相机覆盖错误目标和局部视口，透明清屏且保留业务排序与启用状态。
#[test]
fn binding_configures_dedicated_camera() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WindowPlugin));
    app.init_asset::<Image>();
    app.init_asset::<bevy::scene::ScenePatch>();
    app.init_resource::<ButtonInput<MouseButton>>();
    for is_active in [false, true] {
        let target = app
            .world_mut()
            .spawn(widgetry_window(Window::default()))
            .id();
        let camera = app
            .world_mut()
            .spawn((
                Camera2d,
                Camera {
                    viewport: Some(Viewport::default()),
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                    order: 7,
                    is_active,
                    ..default()
                },
                RenderTarget::Window(WindowRef::Entity(Entity::PLACEHOLDER)),
            ))
            .id();
        let root = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                window(target, camera, WindowControlsConfig::default(), bsn_list![], bsn_list![])
            })
            .id();
        app.update();
        assert!(app.world().get_entity(root).is_ok());
        assert!(matches!(app.world().get::<RenderTarget>(camera),
            Some(RenderTarget::Window(WindowRef::Entity(bound))) if *bound == target));
        let configured = app.world().get::<Camera>(camera).unwrap();
        assert!(configured.viewport.is_none());
        assert!(
            matches!(configured.clear_color, ClearColorConfig::Custom(color) if color == Color::NONE)
        );
        assert_eq!(configured.order, 7);
        assert_eq!(configured.is_active, is_active);
    }
}

/// 透明、装饰或合成模式违反约束时清理完整新树，并保留原生属性及相机配置。
#[test]
fn invalid_native_properties_reject_binding_without_mutating_owners() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WindowPlugin));
    app.init_asset::<Image>();
    app.init_asset::<bevy::scene::ScenePatch>();
    app.init_resource::<ButtonInput<MouseButton>>();
    for (transparent, decorations, composite_alpha_mode) in [
        (false, false, CompositeAlphaMode::PreMultiplied),
        (true, true, CompositeAlphaMode::PreMultiplied),
        (false, true, CompositeAlphaMode::PreMultiplied),
        (true, false, CompositeAlphaMode::Auto),
        (true, false, CompositeAlphaMode::Opaque),
        (true, false, CompositeAlphaMode::Inherit),
        (true, false, CompositeAlphaMode::PostMultiplied),
    ] {
        let target = app
            .world_mut()
            .spawn(Window {
                transparent,
                decorations,
                composite_alpha_mode,
                ..default()
            })
            .id();
        let camera = app
            .world_mut()
            .spawn((
                Camera2d,
                Camera {
                    viewport: Some(Viewport::default()),
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                    ..default()
                },
            ))
            .id();
        let root = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                window(target, camera, WindowControlsConfig::default(), bsn_list![],
                    bsn_list![(Name("RejectedContent"))])
            })
            .id();
        app.world_mut().flush();
        let mut tree = vec![root];
        let mut index = 0;
        while index < tree.len() {
            if let Some(children) = app.world().get::<Children>(tree[index]) {
                tree.extend(children.iter());
            }
            index += 1;
        }
        assert!(tree.len() > 1);
        app.update();
        assert!(
            tree.iter()
                .all(|&entity| app.world().get_entity(entity).is_err())
        );
        let native = app.world().get::<Window>(target).unwrap();
        assert_eq!(native.transparent, transparent);
        assert_eq!(native.decorations, decorations);
        assert_eq!(native.composite_alpha_mode, composite_alpha_mode);
        let configured = app.world().get::<Camera>(camera).unwrap();
        assert!(configured.viewport.is_some());
        assert!(
            matches!(configured.clear_color, ClearColorConfig::Custom(color) if color == Color::BLACK)
        );
        assert!(matches!(
            app.world().get::<RenderTarget>(camera),
            Some(RenderTarget::Window(WindowRef::Primary))
        ));
    }
}
