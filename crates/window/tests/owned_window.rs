//! State：root/native lifecycle、owned/borrowed resources、parent modal child 数量。
//! stimuli 为公开 Scene、despawn 与重复 WindowClosed。
//! Invariant：资源归属只影响对应 root，唯一 blocker 随最后有效 child 释放。
//! 另一个 native parent 的完整 entity 集合保持。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{WindowClosed, WindowRef},
};
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::z_index;
use bevy_widgetry_test_utils::{advance_until, scene_app};
use bevy_widgetry_window::{
    WidgetryModalWindow, WidgetryWindowBackground, WidgetryWindowControlsConfig,
    WidgetryWindowPlugin, owned_widgetry_window, prepare_native_window, widgetry_window,
};
use std::time::Duration;

#[test]
fn owned_resources_follow_root_lifetime() {
    for native_first in [false, true] {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app.world_mut().commands().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
        }).id();
        app.update();
        let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
        let target = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .unwrap();
        assert!(
            matches!(app.world().get::<RenderTarget>(camera), Some(RenderTarget::Window(WindowRef::Entity(entity))) if *entity == target)
        );
        if native_first {
            app.world_mut().entity_mut(target).despawn();
            app.world_mut()
                .write_message(WindowClosed { window: target });
            app.update();
        } else {
            app.world_mut().entity_mut(root).despawn();
            app.world_mut().flush();
        }
        for entity in [root, target, camera] {
            assert!(app.world().get_entity(entity).is_err());
        }
    }
}

fn native(world: &World, root: Entity) -> Entity {
    let camera = world.get::<UiTargetCamera>(root).unwrap().0;
    match world.get::<RenderTarget>(camera).unwrap() {
        RenderTarget::Window(WindowRef::Entity(window)) => *window,
        target => panic!("应绑定 native window，实际为 {target:?}"),
    }
}

fn resources(world: &World, root: Entity) -> Vec<Entity> {
    let mut tree = vec![root];
    let mut index = 0;
    while index < tree.len() {
        if let Some(children) = world.get::<Children>(tree[index]) {
            tree.extend(children.iter());
        }
        index += 1;
    }
    tree.push(world.get::<UiTargetCamera>(root).unwrap().0);
    tree.push(native(world, root));
    tree
}

fn wait_for_title_icons(app: &mut App) {
    let icons = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryIcon>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    assert!(!icons.is_empty());
    advance_until(
        app,
        Duration::from_secs(5),
        "Window title icon image children",
        |world| {
            icons.iter().all(|icon| {
                world.get::<Children>(*icon).is_some_and(|children| {
                    children
                        .iter()
                        .any(|child| world.get::<ImageNode>(child).is_some())
                })
            })
        },
    )
    .expect("记录资源基线前 title icon 必须就绪");
}

fn blockers(world: &World, root: Entity) -> Vec<Entity> {
    world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .filter(|entity| {
            world.get::<GlobalZIndex>(*entity) == Some(&GlobalZIndex(z_index::MODAL))
                && world
                    .get::<Pickable>(*entity)
                    .is_some_and(|p| p.should_block_lower && !p.is_hoverable)
        })
        .collect()
}

#[test]
fn public_modal_children_share_only_their_own_parent_blocker() {
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    let first = app.world_mut().commands().spawn_scene(bsn! {
        owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![(Text("parent A"))])
    }).id();
    let borrowed_window = app
        .world_mut()
        .spawn(prepare_native_window(Window::default()))
        .id();
    let borrowed_camera = app.world_mut().spawn(Camera2d).id();
    let second = app.world_mut().commands().spawn_scene(bsn! {
        widgetry_window(borrowed_window, borrowed_camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![(Text("parent B"))])
    }).id();
    app.update();
    wait_for_title_icons(&mut app);
    let parent = native(app.world(), first);
    let other = resources(app.world(), second);
    assert!(blockers(app.world(), first).is_empty());
    assert!(blockers(app.world(), second).is_empty());
    let mut children = Vec::new();
    let mut blocker = None;
    for _ in 0..2 {
        let child = app.world_mut().commands().spawn_scene(bsn! {
            owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
            template(move |_| Ok(WidgetryModalWindow {parent}))
        }).id();
        app.update();
        let current = blockers(app.world(), first);
        assert_eq!(current.len(), 1);
        if let Some(previous) = blocker {
            assert_eq!(current[0], previous);
        }
        blocker = Some(current[0]);
        children.push((child, resources(app.world(), child)));
        assert!(blockers(app.world(), second).is_empty());
        assert_eq!(resources(app.world(), second), other);
    }
    for (position, (child, owned)) in children.into_iter().enumerate() {
        app.world_mut().despawn(child);
        app.world_mut().flush();
        for entity in owned {
            assert!(app.world().get_entity(entity).is_err());
        }
        let current = blockers(app.world(), first);
        assert_eq!(
            current,
            if position == 0 {
                vec![blocker.unwrap()]
            } else {
                vec![]
            }
        );
        assert_eq!(resources(app.world(), second), other);
        for &entity in &other {
            assert!(app.world().get_entity(entity).is_ok());
        }
    }
    assert!(app.world().get_entity(blocker.unwrap()).is_err());
    assert!(app.world().get_entity(parent).is_ok());
}

#[test]
fn repeated_lifecycle_signals_do_not_reclaim_other_roots() {
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    let roots: Vec<_> = (0..2).map(|_| app.world_mut().commands().spawn_scene(bsn! {
        owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![(Text("owned content"))])
    }).id()).collect();
    let borrowed_window = app
        .world_mut()
        .spawn(prepare_native_window(Window::default()))
        .id();
    let borrowed_camera = app.world_mut().spawn(Camera2d).id();
    let borrowed = app.world_mut().commands().spawn_scene(bsn! {
        widgetry_window(borrowed_window, borrowed_camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![(Text("borrowed content"))])
    }).id();
    app.update();
    wait_for_title_icons(&mut app);
    let retired = resources(app.world(), roots[0]);
    let closed_window = native(app.world(), roots[0]);
    let preserved = [
        resources(app.world(), roots[1]),
        resources(app.world(), borrowed),
    ];
    for _ in 0..2 {
        app.world_mut().commands().entity(roots[0]).try_despawn();
        app.world_mut().write_message(WindowClosed {
            window: closed_window,
        });
        app.world_mut().write_message(WindowClosed {
            window: closed_window,
        });
        app.update();
        for &entity in &retired {
            assert!(app.world().get_entity(entity).is_err());
        }
        for (root, saved) in [roots[1], borrowed].into_iter().zip(&preserved) {
            assert_eq!(resources(app.world(), root), *saved);
            for &entity in saved {
                assert!(app.world().get_entity(entity).is_ok());
            }
        }
    }
}
