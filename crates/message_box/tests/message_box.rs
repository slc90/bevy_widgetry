//! Coverage Map：本文件负责公开构造、Button→一次性结果、observer command/关闭、无结果结束和实际文本/layout。
//! scene.rs 负责私有 action 顺序与构造。
//! lifecycle.rs 保留同步 observer 在 Closing 前可读的边界。
//! State：未决议/已发出结果/已关闭。
//! stimuli 为真实输入、排队动作、callback cleanup、native/parent结束与theme。
//! Invariant：结果 source 为 dialog root、最多一次且 first accepted wins。
//! 结束不合成 Cancel，其他 dialog 归属不变。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::camera::{RenderTarget, visibility::VisibilitySystems};
use bevy::window::WindowRef;
use bevy::{
    prelude::*,
    ui_widgets::{Activate, ButtonPlugin},
    window::WindowClosed,
};
use bevy_widgetry_asset::BuiltinFont;
use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry_core::{WidgetryAppExt, foreground::ResolvedForeground};
use bevy_widgetry_message_box::{
    WidgetryMessageBox, WidgetryMessageBoxButtons, WidgetryMessageBoxPlugin,
    WidgetryMessageBoxResult, WidgetryMessageBoxResultEvent, widgetry_message_box,
};
use bevy_widgetry_test_utils::switch_theme;
use bevy_widgetry_test_utils::{
    add_ui_plugins, advance_until, press, primary_click, scene_app, spawn_ui_camera,
};
use bevy_widgetry_theme::WidgetryThemeMode;
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
    owned_widgetry_window,
};
use std::time::Duration;

#[derive(Resource, Default)]
struct Results(Vec<(Entity, WidgetryMessageBoxResult)>);

#[derive(Resource, Default)]
struct CommandReads(Vec<String>);

fn observed_app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryMessageBoxPlugin)
        .init_resource::<Results>();
    app.add_observer(
        |event: On<WidgetryMessageBoxResultEvent>, mut results: ResMut<Results>| {
            results.0.push((event.entity, event.result));
        },
    );
    app
}

fn parent(app: &mut App) -> (Entity, Entity) {
    let root = app.world_mut().commands().spawn_scene(bsn! {
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
    }).id();
    app.update();
    (root, native(app.world(), root))
}

fn native(world: &World, root: Entity) -> Entity {
    let camera = world.get::<UiTargetCamera>(root).unwrap().0;
    match world.get::<RenderTarget>(camera).unwrap() {
        RenderTarget::Window(WindowRef::Entity(entity)) => *entity,
        other => panic!("应为 native window target，实际为 {other:?}"),
    }
}

fn subtree(world: &World, root: Entity) -> Vec<Entity> {
    let mut result = vec![root];
    let mut at = 0;
    while at < result.len() {
        if let Some(children) = world.get::<Children>(result[at]) {
            result.extend(children.iter());
        }
        at += 1;
    }
    result
}

fn action(app: &App, root: Entity, label: &str) -> Entity {
    subtree(app.world(), root)
        .into_iter()
        .find(|entity| {
            app.world().get::<WidgetryButton>(*entity).is_some()
                && app
                    .world()
                    .get::<Children>(*entity)
                    .is_some_and(|children| {
                        children.iter().any(|child| {
                            app.world()
                                .get::<Text>(child)
                                .is_some_and(|text| text.0 == label)
                        })
                    })
        })
        .unwrap()
}

fn resources(app: &App, root: Entity) -> Vec<Entity> {
    let mut result = subtree(app.world(), root);
    result.extend([
        app.world().get::<UiTargetCamera>(root).unwrap().0,
        native(app.world(), root),
    ]);
    result
}

#[test]
fn reentrant_result_and_queued_observer_read_resolve_once_before_cleanup() {
    let mut app = observed_app();
    app.init_resource::<CommandReads>();
    let (_, parent) = parent(&mut app);
    let root = app.world_mut().commands().spawn_scene(bsn! {
        @widgetry_message_box(parent, "Reenter", WidgetryMessageBoxButtons::YesNoCancel, Default::default(),  bsn_list!{Text("read before close") bevy_widgetry_core::text::WidgetryText Name("observer body")})
    }).id();
    app.update();
    let yes = action(&app, root, "Yes");
    let no = action(&app, root, "No");
    let body = subtree(app.world(), root)
        .into_iter()
        .find(|entity| {
            app.world()
                .get::<Name>(*entity)
                .is_some_and(|name| name.as_str() == "observer body")
        })
        .unwrap();
    let owned = resources(&app, root);
    app.add_observer(
        move |event: On<WidgetryMessageBoxResultEvent>, mut commands: Commands| {
            if event.entity != root {
                return;
            }
            commands.trigger(Activate { entity: no });
            commands.queue(move |world: &mut World| {
                assert!(world.get::<WidgetryMessageBox>(root).is_some());
                let value = world.get::<Text>(body).unwrap().0.clone();
                world.resource_mut::<CommandReads>().0.push(value);
            });
        },
    );
    press(&mut app, yes);
    app.world_mut().trigger(primary_click(yes));
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Results>().0,
        vec![(root, WidgetryMessageBoxResult::Yes)]
    );
    assert_eq!(
        app.world().resource::<CommandReads>().0,
        vec!["read before close"]
    );
    for entity in owned {
        assert!(app.world().get_entity(entity).is_err());
    }
    assert!(app.world().get_entity(parent).is_ok());
}

#[test]
fn ordered_different_actions_keep_the_first_accepted_result() {
    for (first, second, expected) in [
        ("Yes", "No", WidgetryMessageBoxResult::Yes),
        ("No", "Yes", WidgetryMessageBoxResult::No),
    ] {
        let mut app = observed_app();
        let (_, parent) = parent(&mut app);
        let root = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                @widgetry_message_box(parent, "Order", WidgetryMessageBoxButtons::YesNo, Default::default(),  bsn_list!{})
            })
            .id();
        app.update();
        let (first, second) = (action(&app, root, first), action(&app, root, second));
        app.world_mut()
            .commands()
            .trigger(Activate { entity: first });
        app.world_mut()
            .commands()
            .trigger(Activate { entity: second });
        app.world_mut().flush();
        assert_eq!(app.world().resource::<Results>().0, vec![(root, expected)]);
        assert!(app.world().get_entity(root).is_err());
    }
}

#[test]
fn callback_cleanup_preserves_unrelated_dialogs_and_releases_the_last_blocker() {
    for end_parent in [false, true] {
        let mut app = observed_app();
        let (parent_root, parent_native) = parent(&mut app);
        let (_, other_parent) = parent(&mut app);
        let baseline = app.world().get::<Children>(parent_root).unwrap().len();
        let mut roots = Vec::new();
        for target in [parent_native, parent_native, other_parent] {
            roots.push(app.world_mut().commands().spawn_scene(bsn! {
                @widgetry_message_box(target, "Callback", WidgetryMessageBoxButtons::Ok, Default::default(),  bsn_list!{Text("keep ownership") bevy_widgetry_core::text::WidgetryText})
            }).id());
        }
        app.update();
        let root = roots[0];
        let retired = resources(&app, root);
        let sibling = resources(&app, roots[1]);
        let unrelated = resources(&app, roots[2]);
        let button = action(&app, root, "OK");
        app.add_observer(
            move |event: On<WidgetryMessageBoxResultEvent>, mut commands: Commands| {
                if event.entity != root {
                    return;
                }
                commands
                    .entity(if end_parent { parent_native } else { root })
                    .try_despawn();
                if end_parent {
                    commands.write_message(WindowClosed {
                        window: parent_native,
                    });
                }
            },
        );
        press(&mut app, button);
        app.world_mut().trigger(primary_click(button));
        app.world_mut().flush();
        app.update();
        assert_eq!(
            app.world().resource::<Results>().0,
            vec![(root, WidgetryMessageBoxResult::Ok)]
        );
        for entity in retired {
            assert!(app.world().get_entity(entity).is_err());
        }
        for entity in sibling {
            assert_eq!(app.world().get_entity(entity).is_ok(), !end_parent);
        }
        assert_eq!(resources(&app, roots[2]), unrelated);
        for entity in unrelated {
            assert!(app.world().get_entity(entity).is_ok());
        }
        if !end_parent {
            assert_eq!(
                app.world().get::<Children>(parent_root).unwrap().len(),
                baseline + 1
            );
            app.world_mut().despawn(roots[1]);
            app.world_mut().flush();
            assert_eq!(
                app.world().get::<Children>(parent_root).unwrap().len(),
                baseline
            );
            assert_eq!(app.world().resource::<Results>().0.len(), 1);
        }
    }
}

#[test]
fn invalid_or_ended_parent_cleanup_has_no_result() {
    for invalid in [false, true] {
        let mut app = observed_app();
        let (_, parent) = parent(&mut app);
        let target = if invalid { Entity::PLACEHOLDER } else { parent };
        let root = app.world_mut().commands().spawn_scene(bsn! {
            @widgetry_message_box(target, "Parent ends", WidgetryMessageBoxButtons::YesNoCancel, Default::default(),  bsn_list!{})
        }).id();
        app.update();
        if !invalid {
            app.world_mut().despawn(parent);
            app.world_mut()
                .write_message(WindowClosed { window: parent });
            app.update();
        }
        assert!(app.world().get_entity(root).is_err());
        assert!(app.world().resource::<Results>().0.is_empty());
    }
}

#[test]
fn real_button_clicks_return_all_results_and_release_last_blocker() {
    for (label, expected) in [
        ("OK", WidgetryMessageBoxResult::Ok),
        ("Yes", WidgetryMessageBoxResult::Yes),
        ("No", WidgetryMessageBoxResult::No),
        ("Cancel", WidgetryMessageBoxResult::Cancel),
    ] {
        let mut app = scene_app();
        app.add_plugins((ButtonPlugin, WidgetryMessageBoxPlugin))
            .init_resource::<Results>();
        app.add_observer(
            |event: On<WidgetryMessageBoxResultEvent>,
             roots: Query<(), With<WidgetryMessageBox>>,
             mut results: ResMut<Results>| {
                assert!(roots.contains(event.entity));
                results.0.push((event.entity, event.result));
            },
        );
        let parent_root = app.world_mut().commands().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
        }).id();
        app.update();
        let parent = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .unwrap();
        let baseline_children = app.world().get::<Children>(parent_root).unwrap().len();
        let buttons = if label == "OK" {
            WidgetryMessageBoxButtons::Ok
        } else {
            WidgetryMessageBoxButtons::YesNoCancel
        };
        let roots: Vec<_> = (0..2)
            .map(|_| {
                app.world_mut()
                    .commands()
                    .spawn_scene(bsn! {
                        @widgetry_message_box(parent, "Choose", buttons, Default::default(),  bsn_list!{Text("Body") bevy_widgetry_core::text::WidgetryText})
                    })
                    .id()
            })
            .collect();
        app.update();
        assert_eq!(
            app.world().get::<Children>(parent_root).unwrap().len(),
            baseline_children + 1
        );
        for (index, root) in roots.iter().copied().enumerate() {
            let button = app
                .world_mut()
                .query::<(Entity, &WidgetryButton)>()
                .iter(app.world())
                .map(|(entity, _)| entity)
                .find(|&entity| {
                    let mut current = entity;
                    while let Some(parent) = app.world().get::<ChildOf>(current) {
                        current = parent.parent();
                    }
                    current == root
                        && app.world().get::<Children>(entity).is_some_and(|children| {
                            children.iter().any(|child| {
                                app.world()
                                    .get::<Text>(child)
                                    .is_some_and(|text| text.0 == label)
                            })
                        })
                })
                .unwrap();
            press(&mut app, button);
            app.world_mut().trigger(primary_click(button));
            app.world_mut().flush();
            assert!(app.world().get_entity(root).is_err());
            assert_eq!(app.world().resource::<Results>().0[index], (root, expected));
            assert_eq!(
                app.world().get::<Children>(parent_root).unwrap().len(),
                baseline_children + usize::from(index == 0)
            );
        }
        assert_eq!(
            app.world_mut().query::<&Window>().iter(app.world()).count(),
            1
        );
        assert_eq!(
            app.world_mut().query::<&Camera>().iter(app.world()).count(),
            1
        );
    }
}

#[test]
fn native_close_has_no_result() {
    let mut app = scene_app();
    app.add_plugins(WidgetryMessageBoxPlugin)
        .init_resource::<Results>();
    app.add_observer(
        |event: On<WidgetryMessageBoxResultEvent>, mut results: ResMut<Results>| {
            results.0.push((event.entity, event.result))
        },
    );
    app.world_mut().commands().spawn_scene(bsn! { @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{}) });
    app.update();
    let parent = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    let root = app
        .world_mut()
        .commands()
        .spawn_scene(
            bsn! { @widgetry_message_box(parent, "Close", WidgetryMessageBoxButtons::YesNoCancel, Default::default(),  bsn_list!{}) },
        )
        .id();
    app.update();
    let native = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .iter(app.world())
        .find(|&entity| entity != parent)
        .unwrap();
    app.world_mut().entity_mut(native).despawn();
    app.world_mut()
        .write_message(WindowClosed { window: native });
    app.update();
    assert!(app.world().get_entity(root).is_err());
    assert!(app.world().resource::<Results>().0.is_empty());
    assert_eq!(
        app.world_mut().query::<&Camera>().iter(app.world()).count(),
        1
    );
}

#[test]
fn plugin_ensures_dependencies_once() {
    for pre_registered in [false, true] {
        let mut app = scene_app();
        if pre_registered {
            app.add_plugins((WidgetryWindowPlugin, WidgetryButtonPlugin));
        }
        app.add_plugins(WidgetryMessageBoxPlugin);
        assert_eq!(app.get_added_plugins::<WidgetryWindowPlugin>().len(), 1);
        assert_eq!(app.get_added_plugins::<WidgetryButtonPlugin>().len(), 1);
    }
}

#[test]
fn disabled_action_does_not_resolve() {
    let mut app = observed_app();
    app.world_mut().commands().spawn_scene(bsn! { @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{}) });
    app.update();
    let parent = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    let root = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! { @widgetry_message_box(parent, "Disabled", WidgetryMessageBoxButtons::Ok, Default::default(),  bsn_list!{}) })
        .id();
    app.update();
    let button = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryButton>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(button)
        .insert(bevy::ui::InteractionDisabled);
    app.world_mut().trigger(Activate { entity: button });
    app.world_mut().flush();
    assert!(app.world().get_entity(root).is_ok());
    assert!(app.world().resource::<Results>().0.is_empty());
    app.world_mut()
        .entity_mut(button)
        .remove::<bevy::ui::InteractionDisabled>();
    press(&mut app, button);
    app.world_mut().trigger(primary_click(button));
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Results>().0,
        vec![(root, WidgetryMessageBoxResult::Ok)]
    );
    assert!(app.world().get_entity(root).is_err());
}

#[test]
fn message_box_text_tracks_theme() {
    let mut app = rendering_app();
    let (_, parent) = parent(&mut app);
    let root = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! {
            @widgetry_message_box(parent, "Theme", WidgetryMessageBoxButtons::Ok, Default::default(),
                bsn_list!{Node Children [Node Children [Text("Nested body") bevy_widgetry_core::text::WidgetryText]]})
        })
        .id();
    app.update();
    prepare_bound_camera(&mut app, root);
    app.update();
    for mode in [WidgetryThemeMode::Dark, WidgetryThemeMode::Light] {
        switch_theme(&mut app, mode);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            mode.colors().window.frame.normal.background
        );
        assert!(app.world().get::<ImageNode>(root).is_none());
        assert_eq!(
            app.world().get::<ResolvedForeground>(root).unwrap().0,
            mode.colors().message_box.body.normal.foreground
        );
        let texts: Vec<_> = subtree(app.world(), root)
            .into_iter()
            .filter(|entity| {
                app.world()
                    .get::<Text>(*entity)
                    .is_some_and(|text| ["Theme", "Nested body"].contains(&text.0.as_str()))
            })
            .collect();
        assert_eq!(texts.len(), 2);
        for text in texts {
            assert_eq!(
                app.world().get::<TextColor>(text).unwrap().0,
                mode.colors().message_box.body.normal.foreground
            );
            assert!(
                !app.world()
                    .get::<bevy::text::TextLayoutInfo>(text)
                    .unwrap()
                    .glyphs
                    .is_empty()
            );
            assert!(app.world().get::<InheritedVisibility>(text).unwrap().get());
            assert!(
                app.world()
                    .get::<ComputedNode>(text)
                    .unwrap()
                    .size()
                    .min_element()
                    > 0.0
            );
        }
    }
}

fn rendering_app() -> App {
    let mut app = observed_app();
    add_ui_plugins(&mut app);
    app.configure_sets(
        PostUpdate,
        (
            VisibilitySystems::VisibilityPropagate,
            bevy::ui::UiSystems::Stack,
        )
            .before(bevy::ui::UiSystems::Propagate),
    );
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    advance_until(
        &mut app,
        Duration::from_secs(10),
        "MessageBox 前置字体",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app
}

fn prepare_bound_camera(app: &mut App, root: Entity) {
    let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
    let temporary = spawn_ui_camera(app, UVec2::new(460, 260), 1.0);
    let computed = app
        .world()
        .get::<Camera>(temporary)
        .unwrap()
        .computed
        .clone();
    app.world_mut().despawn(temporary);
    app.world_mut().get_mut::<Camera>(camera).unwrap().computed = computed;
}

#[test]
fn long_body_layout_keeps_the_fixed_result_row_visible() {
    let mut app = rendering_app();
    let (_, parent) = parent(&mut app);
    let body_text = "A long message that wraps in a fixed dialog. ".repeat(80);
    let root = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! {
            @widgetry_message_box(parent, "Long body", WidgetryMessageBoxButtons::YesNoCancel, Default::default(),
                bsn_list!{Text(body_text) bevy_widgetry_core::text::WidgetryText Name("long body")})
        })
        .id();
    app.update();
    prepare_bound_camera(&mut app, root);
    app.update();
    assert_eq!(
        app.world().get::<ComputedNode>(root).unwrap().size(),
        Vec2::new(460.0, 260.0)
    );
    let text = subtree(app.world(), root)
        .into_iter()
        .find(|entity| {
            app.world()
                .get::<Name>(*entity)
                .is_some_and(|name| name.as_str() == "long body")
        })
        .unwrap();
    assert!(
        !app.world()
            .get::<bevy::text::TextLayoutInfo>(text)
            .unwrap()
            .glyphs
            .is_empty()
    );
    let body = app.world().get::<ChildOf>(text).unwrap().parent();
    let body_size = app.world().get::<ComputedNode>(body).unwrap().size();
    assert!(body_size.y > 0.0 && body_size.y < 260.0);
    assert_eq!(
        app.world().get::<Node>(body).unwrap().overflow,
        Overflow::clip()
    );
    assert!(app.world().get::<ComputedNode>(text).unwrap().size().y > body_size.y);
    for label in ["Yes", "No", "Cancel"] {
        let button = action(&app, root, label);
        let size = app.world().get::<ComputedNode>(button).unwrap().size();
        assert!(size.x >= 84.0);
        assert_eq!(size.y, 36.0);
        assert!(
            app.world()
                .get::<InheritedVisibility>(button)
                .unwrap()
                .get()
        );
        let root_center = app
            .world()
            .get::<UiGlobalTransform>(root)
            .unwrap()
            .translation;
        let center = app
            .world()
            .get::<UiGlobalTransform>(button)
            .unwrap()
            .translation;
        assert!(center.y - size.y / 2.0 >= root_center.y - 130.0);
        assert!(center.y + size.y / 2.0 <= root_center.y + 130.0);
    }
    assert_eq!(
        app.world()
            .get::<Window>(native(app.world(), root))
            .unwrap()
            .resolution
            .width(),
        460.0
    );
}
