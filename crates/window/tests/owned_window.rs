//! State：root/native lifecycle、owned/borrowed resources、parent modal child 数量。
//! stimuli 为公开 Scene、despawn 与重复 WindowClosed。
//! Invariant：资源归属只影响对应 root，唯一 blocker 随最后有效 child 释放。
//! 另一个 native parent 的完整 entity 集合保持。
//! 焦点模型覆盖 embedded/managed window：禁用保留焦点和路由，输入不激活，隐藏仍迁移焦点。
//! IME qualification 覆盖 TextInput/纯 EditableText、Editable/ReadOnly/Static 和禁用，mode 变化取消旧 composition。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    camera::RenderTarget,
    prelude::*,
    text::EditableText,
    window::{WindowClosed, WindowRef},
};
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::z_index;
use bevy_widgetry_test_utils::{add_ui_plugins, advance_until, scene_app};
use bevy_widgetry_window::{
    WidgetryModalWindow, WidgetryWindowBackground, WidgetryWindowControlsConfig,
    WidgetryWindowPlugin, owned_widgetry_window, prepare_native_window, widgetry_window,
};
use std::time::Duration;

fn ime_fixture() -> (App, Entity, Entity) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((WidgetryWindowPlugin, bevy::ui_widgets::TextInputPlugin));
    let root = app.world_mut().spawn_scene(bsn! {
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{Name("Editor") ~{EditableText::new("base")} Node {width:px(180),height:px(40)}})
    }).unwrap().id();
    app.update();
    let native = native(app.world(), root);
    let editor = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "Editor").then_some(entity))
        .unwrap();
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Pressed);
    (app, native, editor)
}

#[test]
fn managed_ime_uses_text_input_mode_and_disabled_qualification() {
    use bevy::text::TextReadWriteMode;
    for qualification in ["editable", "bare", "readonly", "static", "disabled"] {
        let (mut app, native, editor) = ime_fixture();
        if qualification != "bare" {
            app.world_mut()
                .entity_mut(editor)
                .insert(bevy::ui_widgets::TextInput);
        }
        match qualification {
            "readonly" => {
                app.world_mut()
                    .entity_mut(editor)
                    .insert(TextReadWriteMode::ReadOnly);
            }
            "static" => {
                app.world_mut()
                    .entity_mut(editor)
                    .insert(TextReadWriteMode::Static);
            }
            "disabled" => {
                app.world_mut()
                    .entity_mut(editor)
                    .insert(bevy::ui::InteractionDisabled);
            }
            _ => {}
        }
        app.world_mut().write_message(bevy::window::Ime::Commit {
            window: native,
            value: "中".into(),
        });
        app.update();
        let expected = if qualification == "editable" {
            "base中"
        } else {
            "base"
        };
        assert_eq!(
            app.world()
                .get::<EditableText>(editor)
                .unwrap()
                .value()
                .to_string(),
            expected,
            "{qualification}"
        );
        assert_eq!(
            app.world().get::<Window>(native).unwrap().ime_enabled,
            qualification == "editable",
            "{qualification}"
        );
    }
}

#[test]
fn managed_ime_mode_change_cancels_composition_without_committing_it() {
    use bevy::text::TextReadWriteMode;
    for mode in [TextReadWriteMode::ReadOnly, TextReadWriteMode::Static] {
        let (mut app, native, editor) = ime_fixture();
        app.world_mut()
            .entity_mut(editor)
            .insert(bevy::ui_widgets::TextInput);
        app.world_mut().write_message(bevy::window::Ime::Preedit {
            window: native,
            value: "中".into(),
            cursor: Some((0, 3)),
        });
        app.update();
        assert!(
            app.world()
                .get::<EditableText>(editor)
                .unwrap()
                .is_composing()
        );
        app.world_mut().entity_mut(editor).insert(mode);
        app.update();
        assert!(
            !app.world()
                .get::<EditableText>(editor)
                .unwrap()
                .is_composing()
        );
        assert_eq!(
            app.world()
                .get::<EditableText>(editor)
                .unwrap()
                .value()
                .to_string(),
            "base"
        );
        assert!(!app.world().get::<Window>(native).unwrap().ime_enabled);
    }
}

#[test]
fn invalid_colors_do_not_allocate_owned_resources() {
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    let before = app.world().entities().count_spawned();
    let mut colors = bevy_widgetry_window::WidgetryWindowColorOverrides::default();
    colors.frame.normal.background = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
    let result = app.world_mut().spawn_scene(bsn! {
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, colors, bsn_list!{}, bsn_list!{})
    });
    assert!(result.is_err());
    app.world_mut().flush();
    assert_eq!(app.world().entities().count_spawned(), before);
    assert_eq!(
        app.world_mut().query::<&Window>().iter(app.world()).count(),
        0
    );
    assert_eq!(
        app.world_mut().query::<&Camera>().iter(app.world()).count(),
        0
    );
}

#[test]
fn owned_resources_follow_root_lifetime() {
    for native_first in [false, true] {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app.world_mut().commands().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
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
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{Text("parent A") bevy_widgetry_core::text::WidgetryText})
    }).id();
    let borrowed_window = app
        .world_mut()
        .spawn(prepare_native_window(Window::default()))
        .id();
    let borrowed_camera = app.world_mut().spawn(Camera2d).id();
    let second = app.world_mut().commands().spawn_scene(bsn! {
        @widgetry_window(borrowed_window, borrowed_camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{Text("parent B") bevy_widgetry_core::text::WidgetryText})
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
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
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
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{Text("owned content") bevy_widgetry_core::text::WidgetryText})
    }).id()).collect();
    let borrowed_window = app
        .world_mut()
        .spawn(prepare_native_window(Window::default()))
        .id();
    let borrowed_camera = app.world_mut().spawn(Camera2d).id();
    let borrowed = app.world_mut().commands().spawn_scene(bsn! {
        @widgetry_window(borrowed_window, borrowed_camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{Text("borrowed content") bevy_widgetry_core::text::WidgetryText})
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

#[derive(Resource, Default)]
struct Activations(usize);

#[test]
fn inherited_disabled_preserves_window_focus_and_keyboard_target() {
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy::ui::InteractionDisabled;
    use bevy::ui_widgets::{Activate, Button, ButtonPlugin};
    use bevy::window::PrimaryWindow;
    use bevy_widgetry_test_utils::press_key;

    for managed in [false, true] {
        let mut app = scene_app();
        app.add_plugins((WidgetryWindowPlugin, ButtonPlugin))
            .init_resource::<Activations>();
        app.add_observer(|_: On<Activate>, mut count: ResMut<Activations>| count.0 += 1);
        let root = if managed {
            app.world_mut().commands().spawn_scene(bsn! {
                @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
            }).id()
        } else {
            app.world_mut().spawn((Window::default(), PrimaryWindow));
            app.world_mut().spawn(Node::default()).id()
        };
        app.update();
        let native = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .unwrap();
        let parent = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
        let button = app
            .world_mut()
            .spawn((Node::default(), Button, ChildOf(parent)))
            .id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(button, FocusCause::Navigated);
        app.world_mut()
            .entity_mut(parent)
            .insert(InteractionDisabled);
        app.world_mut().flush();
        app.update();
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(button),
            "managed={managed}"
        );
        press_key(&mut app, native, KeyCode::Enter);
        assert_eq!(app.world().resource::<Activations>().0, 0);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
        app.world_mut()
            .entity_mut(parent)
            .remove::<InteractionDisabled>();
        app.world_mut().flush();
        press_key(&mut app, native, KeyCode::Enter);
        assert_eq!(app.world().resource::<Activations>().0, 1);
        if managed {
            app.world_mut().get_mut::<Node>(parent).unwrap().display = Display::None;
            app.update();
            assert_ne!(app.world().resource::<InputFocus>().get(), Some(button));
        }
    }
}
