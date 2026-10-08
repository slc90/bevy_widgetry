//! Icon Coverage Map：本文件负责公开 API、真实 loader、首帧 UI 准备、颜色传播与共享 image。
//! icon.rs 局部测试负责保留 handle 控制的等待/乱序就绪/取消/销毁/零尺寸和失败诊断。
//! icon/svg.rs 局部测试负责缩放、ceil 尺寸、像素 buffer 与 Image 转换。
//!
//! State：尚无图/已有图、当前显示/请求中资源、显式色/继承色/白色 fallback、可显示/零尺寸。
//! Stimuli：Scene 构造、asset 就绪或失败、set_svg、set_color、clear_color、foreground propagation、despawn。
//! Guards：只有当前 SVG 就绪且尺寸非零才创建或替换图像。
//! 冷加载等待与同帧更新分别观察。
//! Invariants：最多一个受 Icon 管理的 image child，等待不清空旧图，显式色优先，image child 不拦截 picking。
//! Couplings：共享像素不共享颜色或 entity lifecycle。
//! 同帧检查在单次 update 后，不用条件等待替代。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::app::Propagate;
use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::ui::{ComputedStackIndex, UiSystems};
use bevy::window::RequestRedraw;
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_core::ForegroundColor;
use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin};
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_ui_plugins, advance_until, scene_app, spawn_ui_camera,
};
use std::time::{Duration, Instant};

#[test]
fn entity_api_commits_inputs_and_distinguishes_same_values() {
    let mut app = scene_app();
    app.add_plugins((WidgetryAssetPlugin, WidgetryIconPlugin));
    let entity = app.world_mut().spawn_scene(bsn! {
        @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @max_size: {Some(UVec2::splat(16))} }
    }).unwrap().id();
    assert!(WidgetryIcon::set_color_in_world(app.world_mut(), entity, Color::BLACK).unwrap());
    assert!(!WidgetryIcon::set_color_in_world(app.world_mut(), entity, Color::BLACK).unwrap());
    let icon = app.world().get::<WidgetryIcon>(entity).unwrap();
    assert_eq!(icon.color_override(), Some(Color::BLACK));
    assert_eq!(icon.max_size(), Some(UVec2::splat(16)));
    assert!(WidgetryIcon::clear_color_in_world(app.world_mut(), entity).unwrap());
    assert!(!WidgetryIcon::clear_color_in_world(app.world_mut(), entity).unwrap());
    assert!(
        !WidgetryIcon::set_svg_in_world(app.world_mut(), entity, BuiltinIcon::WindowClose.path())
            .unwrap()
    );
    assert!(
        WidgetryIcon::set_svg_in_world(app.world_mut(), entity, BuiltinIcon::WindowRestore.path())
            .unwrap()
    );
    assert_eq!(
        app.world().get::<WidgetryIcon>(entity).unwrap().path(),
        Some(&BuiltinIcon::WindowRestore.path())
    );
    assert_eq!(
        app.world()
            .get::<WidgetryIcon>(entity)
            .unwrap()
            .color_override(),
        None
    );
    assert!(app.world().get::<Children>(entity).is_none());
}

#[test]
fn entity_api_rejects_invalid_targets_without_side_effects() {
    let logs = LogCapture::default();
    logs.run(|| {
        let mut app = scene_app();
        app.add_plugins(WidgetryIconPlugin);
        let ordinary = app.world_mut().spawn_empty().id();
        let stale = app.world_mut().spawn_empty().id();
        app.world_mut().despawn(stale);
        for entity in [ordinary, stale] {
            for error in [
                WidgetryIcon::set_color_in_world(app.world_mut(), entity, Color::BLACK)
                    .unwrap_err(),
                WidgetryIcon::clear_color_in_world(app.world_mut(), entity).unwrap_err(),
                WidgetryIcon::set_svg_in_world(app.world_mut(), entity, "new.svg").unwrap_err(),
            ] {
                assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
                assert!(error.to_string().contains("Icon"));
            }
        }
        assert!(app.world().get::<WidgetryIcon>(ordinary).is_none());
        assert!(app.world().get_entity(stale).is_err());
    });
    let errors: Vec<_> = logs
        .records()
        .into_iter()
        .filter(|record| record.level == bevy::log::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 6);
    assert!(
        errors
            .iter()
            .all(|record| record.fields.contains_key("entity"))
    );
}

#[test]
fn missing_asset_server_rejects_svg_requests_without_changing_inputs() {
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    errors.run(|| {
        logs.run(|| {
            let mut app = scene_app();
            app.set_error_handler(ErrorCapture::handler());
            app.add_plugins((WidgetryAssetPlugin, WidgetryIconPlugin));
            let entity = app.world_mut().spawn_scene(bsn! {
            @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @color: {Some(Color::BLACK)} }
        }).unwrap().id();
            app.world_mut().remove_resource::<AssetServer>();
            let error = WidgetryIcon::set_svg_in_world(
                app.world_mut(),
                entity,
                BuiltinIcon::WindowRestore.path(),
            )
            .unwrap_err();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            assert!(error.to_string().contains("AssetServer"));
            WidgetryIcon::set_svg(
                &mut app.world_mut().commands(),
                entity,
                BuiltinIcon::WindowRestore.path(),
            );
            app.world_mut().flush();
            let icon = app.world().get::<WidgetryIcon>(entity).unwrap();
            assert_eq!(icon.path(), Some(&BuiltinIcon::WindowClose.path()));
            assert_eq!(icon.color_override(), Some(Color::BLACK));
        })
    });
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        2
    );
}

#[test]
fn queued_inputs_merge_at_execution_and_report_stale_targets() {
    let errors = ErrorCapture::default();
    errors.run(|| {
        let mut app = scene_app();
        app.set_error_handler(ErrorCapture::handler());
        app.add_plugins((WidgetryAssetPlugin, WidgetryIconPlugin));
        let entity = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()} }
            })
            .unwrap()
            .id();
        {
            let mut commands = app.world_mut().commands();
            WidgetryIcon::set_color(&mut commands, entity, Color::BLACK);
            WidgetryIcon::set_svg(&mut commands, entity, BuiltinIcon::WindowRestore.path());
            WidgetryIcon::clear_color(&mut commands, entity);
            WidgetryIcon::set_color(&mut commands, entity, Color::WHITE);
        }
        assert_eq!(
            app.world()
                .get::<WidgetryIcon>(entity)
                .unwrap()
                .color_override(),
            None
        );
        app.world_mut().flush();
        let icon = app.world().get::<WidgetryIcon>(entity).unwrap();
        assert_eq!(icon.path(), Some(&BuiltinIcon::WindowRestore.path()));
        assert_eq!(icon.color_override(), Some(Color::WHITE));
        {
            let mut commands = app.world_mut().commands();
            WidgetryIcon::set_color(&mut commands, entity, Color::BLACK);
            WidgetryIcon::set_svg(&mut commands, entity, BuiltinIcon::WindowClose.path());
            WidgetryIcon::clear_color(&mut commands, entity);
        }
        app.world_mut().despawn(entity);
        app.world_mut().flush();
        assert!(app.world().get_entity(entity).is_err());
    });
    let failures = errors.take();
    assert_eq!(failures.len(), 3);
    assert!(
        failures
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
}

#[test]
fn runtime_mutations_survive_scene_initialization() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((WidgetryAssetPlugin, WidgetryIconPlugin))
        .configure_sets(
            PostUpdate,
            (VisibilitySystems::VisibilityPropagate, UiSystems::Stack).before(UiSystems::Propagate),
        );
    spawn_ui_camera(&mut app, UVec2::splat(400), 2.0);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryIcon {
                @path: {BuiltinIcon::WindowClose.path()},
                @max_size: { Some(UVec2::new(16, 16)) },
                @color: { Some(Color::BLACK) },
            }
        })
        .unwrap()
        .id();
    advance_until(
        &mut app,
        Duration::from_secs(2),
        &format!("Icon {entity:?} 初始 image child"),
        |world| world.get::<Children>(entity).is_some(),
    )
    .expect("初始图标未生成图像");
    let child = app.world().get::<Children>(entity).unwrap()[0];
    assert!(app.world().get::<InheritedVisibility>(child).unwrap().get());
    assert!(
        app.world().get::<ComputedStackIndex>(child).unwrap().0
            > app.world().get::<ComputedStackIndex>(entity).unwrap().0
    );
    let initial_image = app.world().get::<ImageNode>(child).unwrap().image.clone();
    assert_image_ready(&app, entity);
    let image = app
        .world()
        .resource::<Assets<Image>>()
        .get(&initial_image)
        .unwrap();
    assert_eq!(image.size(), UVec2::splat(16));
    assert_eq!(
        app.world().get::<ComputedNode>(entity).unwrap().size(),
        Vec2::splat(32.0)
    );
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::BLACK
    );
    WidgetryIcon::set_color_in_world(app.world_mut(), entity, Color::srgb(1.0, 0.0, 0.0)).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::srgb(1.0, 0.0, 0.0)
    );
    WidgetryIcon::clear_color_in_world(app.world_mut(), entity).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::WHITE
    );
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().image,
        initial_image
    );
    WidgetryIcon::set_svg_in_world(app.world_mut(), entity, BuiltinIcon::WindowRestore.path())
        .unwrap();
    advance_until(
        &mut app,
        Duration::from_secs(2),
        &format!("Icon {entity:?} SVG 替换"),
        |world| world.get::<ImageNode>(child).unwrap().image != initial_image,
    )
    .expect("运行期 SVG 替换未完成");
    app.update();
    assert_eq!(app.world().get::<Children>(entity).unwrap()[0], child);
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::WHITE
    );
    let node = app.world().get::<Node>(entity).unwrap();
    assert_eq!(node.width, px(16));
    assert_eq!(node.height, px(16));
}

#[test]
fn asynchronous_icons_request_redraw_until_ready() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        WidgetryAssetPlugin,
        WidgetryIconPlugin,
    ))
    .init_asset::<Image>()
    .add_message::<RequestRedraw>();
    let icon = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()} }
        })
        .unwrap()
        .id();
    let mut previous = None;
    for path in [BuiltinIcon::WindowClose, BuiltinIcon::WindowRestore] {
        WidgetryIcon::set_svg_in_world(app.world_mut(), icon, path.path()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            app.world_mut()
                .resource_mut::<Messages<RequestRedraw>>()
                .clear();
            app.update();
            let requested = !app.world().resource::<Messages<RequestRedraw>>().is_empty();
            let image = app.world().get::<Children>(icon).and_then(|children| {
                app.world()
                    .get::<ImageNode>(children[0])
                    .map(|node| node.image.clone())
            });
            assert!(requested, "图标尚需加载或新图像尚需提交时必须请求刷新");
            if image.is_some() && image != previous {
                previous = image;
                break;
            }
            assert!(Instant::now() < deadline, "仅由刷新请求驱动时图标未能完成");
            std::thread::yield_now();
        }
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        app.update();
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }
}

fn ui_app() -> App {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((WidgetryAssetPlugin, WidgetryIconPlugin));
    spawn_ui_camera(&mut app, UVec2::splat(400), 2.0);
    app.configure_sets(
        PostUpdate,
        (VisibilitySystems::VisibilityPropagate, UiSystems::Stack).before(UiSystems::Propagate),
    );
    app
}

fn assert_image_ready(app: &App, icon: Entity) -> Entity {
    let children = app
        .world()
        .get::<Children>(icon)
        .expect("Icon 的真实 image child 应完成渲染准备");
    assert_eq!(children.len(), 1);
    let child = children[0];
    assert_eq!(
        app.world()
            .get::<ChildOf>(child)
            .expect("Icon 的真实 image child 应完成渲染准备")
            .parent(),
        icon
    );
    let node = app
        .world()
        .get::<ImageNode>(child)
        .expect("Icon 的真实 image child 应完成渲染准备");
    let image = app
        .world()
        .resource::<Assets<Image>>()
        .get(&node.image)
        .expect("Icon 的真实 image child 应完成渲染准备");
    assert!(image.width() > 0 && image.height() > 0);
    assert_eq!(
        image
            .data
            .as_ref()
            .expect("Icon 的真实 image child 应完成渲染准备")
            .len(),
        (image.width() * image.height() * 4) as usize
    );
    assert!(
        image
            .data
            .as_ref()
            .expect("Icon 的真实 image child 应完成渲染准备")
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] > 0)
    );
    assert!(
        app.world()
            .get::<InheritedVisibility>(child)
            .expect("Icon 的真实 image child 应完成渲染准备")
            .get()
    );
    assert!(
        app.world()
            .get::<ComputedStackIndex>(child)
            .expect("Icon 的真实 image child 应完成渲染准备")
            .0
            > app
                .world()
                .get::<ComputedStackIndex>(icon)
                .expect("Icon 的真实 image child 应完成渲染准备")
                .0
    );
    let size = app
        .world()
        .get::<ComputedNode>(child)
        .expect("Icon 的真实 image child 应完成渲染准备")
        .size();
    assert!(size.x > 0.0 && size.y > 0.0);
    let picking = app
        .world()
        .get::<Pickable>(child)
        .expect("Icon 的真实 image child 应完成渲染准备");
    assert!(!picking.should_block_lower && !picking.is_hoverable);
    child
}

fn wait_for_image(app: &mut App, icon: Entity) {
    advance_until(
        app,
        Duration::from_secs(2),
        &format!("Icon {icon:?} image child"),
        |world| world.get::<Children>(icon).is_some(),
    )
    .expect("真实 loader 应在期限内完成");
}

#[test]
fn preloaded_source_replacement_is_ready_in_same_frame() {
    for queued in [false, true] {
        let mut app = ui_app();
        let icon = app.world_mut().spawn_scene(bsn! {
            @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @max_size: {Some(UVec2::splat(16))} }
        }).unwrap().id();
        let preload = app.world_mut().spawn_scene(bsn! {
            @WidgetryIcon { @path: {BuiltinIcon::WindowRestore.path()}, @max_size: {Some(UVec2::splat(16))} }
        }).unwrap().id();
        wait_for_image(&mut app, icon);
        wait_for_image(&mut app, preload);
        let child = assert_image_ready(&app, icon);
        let initial = app.world().get::<ImageNode>(child).unwrap().image.clone();
        let preload_child = assert_image_ready(&app, preload);
        let expected = app
            .world()
            .get::<ImageNode>(preload_child)
            .unwrap()
            .image
            .clone();
        app.world_mut().despawn(preload);
        if queued {
            WidgetryIcon::set_svg(
                &mut app.world_mut().commands(),
                icon,
                BuiltinIcon::WindowRestore.path(),
            );
            WidgetryIcon::set_color(&mut app.world_mut().commands(), icon, Color::BLACK);
        } else {
            WidgetryIcon::set_svg_in_world(
                app.world_mut(),
                icon,
                BuiltinIcon::WindowRestore.path(),
            )
            .unwrap();
            WidgetryIcon::set_color_in_world(app.world_mut(), icon, Color::BLACK).unwrap();
        }
        app.update();
        assert_eq!(assert_image_ready(&app, icon), child);
        assert_eq!(app.world().get::<ImageNode>(child).unwrap().image, expected);
        assert_ne!(expected, initial);
        assert!(app.world().get_entity(preload_child).is_err());
        assert_eq!(
            app.world().get::<ImageNode>(child).unwrap().color,
            Color::BLACK
        );
    }
}

#[test]
fn inherited_and_explicit_colors_remain_independent_for_shared_images() {
    let mut app = ui_app();
    let red = Color::srgb(1.0, 0.0, 0.0);
    let blue = Color::srgb(0.0, 0.0, 1.0);
    let green = Color::srgb(0.0, 1.0, 0.0);
    let mut roots = Vec::new();
    for color in [red, blue] {
        let root = app.world_mut().spawn_scene(bsn! {
            Node Children [(
                @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @max_size: {Some(UVec2::splat(16))} }
            )]
        }).unwrap().id();
        app.world_mut()
            .entity_mut(root)
            .insert(Propagate(ForegroundColor(color)));
        roots.push(root);
    }
    let icons: Vec<_> = roots
        .iter()
        .map(|root| app.world().get::<Children>(*root).unwrap()[0])
        .collect();
    for icon in &icons {
        wait_for_image(&mut app, *icon);
    }
    let children: Vec<_> = icons
        .iter()
        .map(|icon| assert_image_ready(&app, *icon))
        .collect();
    let shared = app
        .world()
        .get::<ImageNode>(children[0])
        .unwrap()
        .image
        .clone();
    assert_eq!(
        app.world().get::<ImageNode>(children[1]).unwrap().image,
        shared
    );
    for (child, color) in children.iter().zip([red, blue]) {
        assert_eq!(app.world().get::<ImageNode>(*child).unwrap().color, color);
    }
    app.world_mut()
        .entity_mut(roots[0])
        .insert(Propagate(ForegroundColor(green)));
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(children[0]).unwrap().color,
        green
    );
    assert_eq!(
        app.world().get::<ImageNode>(children[1]).unwrap().color,
        blue
    );
    WidgetryIcon::set_color_in_world(app.world_mut(), icons[0], Color::BLACK).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(children[0]).unwrap().color,
        Color::BLACK
    );
    app.world_mut()
        .entity_mut(roots[0])
        .insert(Propagate(ForegroundColor(red)));
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(children[0]).unwrap().color,
        Color::BLACK
    );
    WidgetryIcon::clear_color_in_world(app.world_mut(), icons[0]).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(children[0]).unwrap().color,
        red
    );
    for (icon, child) in icons.iter().zip(&children) {
        assert_eq!(assert_image_ready(&app, *icon), *child);
        assert_eq!(app.world().get::<ImageNode>(*child).unwrap().image, shared);
    }
    app.world_mut().despawn(icons[0]);
    app.update();
    assert!(app.world().get_entity(children[0]).is_err());
    assert_eq!(assert_image_ready(&app, icons[1]), children[1]);
    assert_eq!(
        app.world().get::<ImageNode>(children[1]).unwrap().image,
        shared
    );
    assert_eq!(
        app.world().get::<ImageNode>(children[1]).unwrap().color,
        blue
    );
}

#[test]
fn all_builtin_icons_materialize_through_real_loader() {
    let mut app = ui_app();
    for builtin in [
        BuiltinIcon::CheckboxCheck,
        BuiltinIcon::CheckboxIndeterminate,
        BuiltinIcon::ChevronDown,
        BuiltinIcon::ChevronUp,
        BuiltinIcon::WindowClose,
        BuiltinIcon::WindowMaximize,
        BuiltinIcon::WindowMinimize,
        BuiltinIcon::WindowRestore,
    ] {
        let icon = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryIcon { @path: {builtin.path()}, @max_size: {Some(UVec2::splat(24))} }
            })
            .unwrap()
            .id();
        wait_for_image(&mut app, icon);
        let child = assert_image_ready(&app, icon);
        let handle = &app.world().get::<ImageNode>(child).unwrap().image;
        let image = app.world().resource::<Assets<Image>>().get(handle).unwrap();
        assert!(
            image.width() <= 24 && image.height() <= 24,
            "{builtin:?} 应遵守尺寸上限"
        );
    }
}
