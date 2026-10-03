//! State：normal/hover/pressed/disabled；Add、Remove、Changed 与 ThemeChanged 驱动完整配色。
//! Guards：disabled 拒绝 pointer activation；重新启用恢复同一 root 的输入。
//! Invariants：disabled > pressed > hover > normal，style 不修改调用方 Node patch 或 children。
//! Coverage Map：background/priority 负责转换输出；theme 负责立即刷新；content 负责 Text/Icon 同帧传播；
//! pointer smoke 负责公开 Scene 到官方 Button observer 的 Activate 桥接，局部优先级归 style.rs。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    app::{App, Propagate},
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    prelude::*,
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Pressed},
    ui_widgets::{Activate, Button, ButtonPlugin},
};
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin};
use bevy_widgetry_core::{DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeMode};
use bevy_widgetry_test_utils::{
    advance_until, press, primary_click, release, scene_app, switch_theme,
};
use rstest::fixture;
use std::time::Duration;

#[fixture]
fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    app
}

mod background {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn spawned_button_is_default(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn hover_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_hovered,
            &node,
            &children,
        );
    }

    #[rstest]
    fn clearing_hover_restores_default(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_hovered,
            &node,
            &children,
        );

        app.world_mut().entity_mut(entity).insert(Hovered(false));

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn pressing_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background,
            &node,
            &children,
        );

        app.world_mut().entity_mut(entity).insert(Pressed);

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_pressed,
            &node,
            &children,
        );
    }

    #[rstest]
    fn removing_pressed_restores_default(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut().entity_mut(entity).insert(Pressed);

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_pressed,
            &node,
            &children,
        );

        app.world_mut().entity_mut(entity).remove::<Pressed>();

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn disabling_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut()
            .entity_mut(entity)
            .insert(InteractionDisabled);

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_disabled,
            &node,
            &children,
        );

        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background,
            &node,
            &children,
        );
    }
}

mod background_priority {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn removing_pressed_falls_back_to_hover(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut()
            .entity_mut(entity)
            .insert(Hovered(true))
            .insert(Pressed);

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_pressed,
            &node,
            &children,
        );

        app.world_mut().entity_mut(entity).remove::<Pressed>();

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_hovered,
            &node,
            &children,
        );
    }

    #[rstest]
    fn removing_disabled_falls_back_to_pressed(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut()
            .entity_mut(entity)
            .insert(Pressed)
            .insert(InteractionDisabled);

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_disabled,
            &node,
            &children,
        );

        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_pressed,
            &node,
            &children,
        );
    }

    #[rstest]
    fn removing_disabled_falls_back_to_hover(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned")] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut()
            .entity_mut(entity)
            .insert(Hovered(true))
            .insert(InteractionDisabled);

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_disabled,
            &node,
            &children,
        );

        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();

        app.update();

        assert_transition_style(
            &app,
            entity,
            DARK_THEME.control_background_hovered,
            &node,
            &children,
        );
    }
}

#[test]
fn widgetry_button_sets_default_foreground() {
    let mut app = app();
    let button = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(button)
            .unwrap()
            .0
            .0,
        DARK_THEME.foreground
    );
}

fn assert_style(
    app: &App,
    entity: bevy::ecs::entity::Entity,
    background: bevy::color::Color,
    border: bevy::color::Color,
    foreground: bevy::color::Color,
) {
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        background
    );
    assert_eq!(
        *app.world().get::<BorderColor>(entity).unwrap(),
        BorderColor::all(border)
    );
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(entity)
            .unwrap()
            .0
            .0,
        foreground
    );
}

fn assert_transition_style(
    app: &App,
    entity: Entity,
    background: Color,
    expected_node: &Node,
    expected_children: &[Entity],
) {
    let c = &DARK_THEME;
    let (border, foreground) = if background == c.control_background_disabled {
        (c.control_border_disabled, c.foreground_disabled)
    } else if background == c.control_background_pressed {
        (c.control_border_pressed, c.foreground)
    } else if background == c.control_background_hovered {
        (c.control_border_hovered, c.foreground)
    } else {
        (c.control_border, c.foreground)
    };
    assert_style(app, entity, background, border, foreground);
    let node = app.world().get::<Node>(entity).unwrap();
    assert_eq!(node, expected_node);
    assert_eq!(node.width, px(137));
    assert_eq!(node.padding, UiRect::all(px(3)));
    let children = app.world().get::<Children>(entity).unwrap();
    assert_eq!(children.to_vec(), expected_children);
    assert_eq!(children.len(), 1);
    assert_eq!(app.world().get::<Text>(children[0]).unwrap().0, "owned");
    assert_eq!(
        app.world().get::<ChildOf>(children[0]).unwrap().parent(),
        entity
    );
}

#[test]
fn newly_widgetry_button_uses_current_theme() {
    let mut app = app();
    switch_theme(&mut app, ThemeMode::Light);
    let fresh = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    app.update();
    assert_style(
        &app,
        fresh,
        LIGHT_THEME.control_background,
        LIGHT_THEME.control_border,
        LIGHT_THEME.foreground,
    );
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryButton Hovered(true)
        })
        .unwrap()
        .id();
    app.update();
    assert_style(
        &app,
        entity,
        LIGHT_THEME.control_background_hovered,
        LIGHT_THEME.control_border_hovered,
        LIGHT_THEME.foreground,
    );
}

#[test]
fn theme_switch_immediately_preserves_button_states() {
    let mut app = app();
    let hovered = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton Hovered(true) })
        .unwrap()
        .id();
    let pressed = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton Pressed })
        .unwrap()
        .id();
    let disabled = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton InteractionDisabled })
        .unwrap()
        .id();
    app.update();
    assert_style(
        &app,
        hovered,
        DARK_THEME.control_background_hovered,
        DARK_THEME.control_border_hovered,
        DARK_THEME.foreground,
    );
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        switch_theme(&mut app, mode);
        let c = mode.colors();
        assert_style(
            &app,
            hovered,
            c.control_background_hovered,
            c.control_border_hovered,
            c.foreground,
        );
        assert_style(
            &app,
            pressed,
            c.control_background_pressed,
            c.control_border_pressed,
            c.foreground,
        );
        assert_style(
            &app,
            disabled,
            c.control_background_disabled,
            c.control_border_disabled,
            c.foreground_disabled,
        );
        assert!(app.world().get::<Hovered>(hovered).unwrap().0);
        assert!(app.world().get::<Pressed>(pressed).is_some());
        assert!(app.world().get::<InteractionDisabled>(disabled).is_some());
    }
}

#[test]
fn scene_provides_default_shell() {
    let mut app = app();
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    let root = app.world().entity(entity);
    assert!(root.contains::<WidgetryButton>());
    assert!(root.contains::<Button>());
    assert!(!root.get::<Hovered>().unwrap().0);
    assert_eq!(root.get::<TabIndex>().unwrap().0, -1);
    assert_eq!(
        *root.get::<Node>().unwrap(),
        Node {
            min_height: px(32),
            padding: UiRect::axes(px(12), px(6)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        }
    );
    assert!(root.contains::<BackgroundColor>());
    assert!(root.contains::<BorderColor>());
    assert!(root.contains::<Propagate<ForegroundColor>>());
}

#[test]
fn plugin_ensures_official_button_behavior() {
    for preinstalled in [false, true] {
        let mut app = App::new();
        app.set_default_font(bevy::text::FontSource::Monospace);
        if preinstalled {
            app.add_plugins(ButtonPlugin);
        }
        app.add_plugins(WidgetryButtonPlugin);
        assert_eq!(app.get_added_plugins::<ButtonPlugin>().len(), 1);
    }
}

#[test]
fn scene_layout_patch_survives_style_updates() {
    let mut app = app();
    let entity = app.world_mut().spawn_scene(bsn! {
        @WidgetryButton
        Node { width: px(100), height: px(40), padding: UiRect::all(px(2)), column_gap: px(6), justify_content: JustifyContent::Center }
    }).unwrap().id();
    app.update();
    let expected = app.world().get::<Node>(entity).unwrap().clone();
    assert_eq!(expected.width, px(100));
    assert_eq!(expected.height, px(40));
    assert_eq!(expected.padding, UiRect::all(px(2)));
    assert_eq!(expected.column_gap, px(6));
    assert_eq!(expected.justify_content, JustifyContent::Center);
    assert_eq!(expected.min_height, px(32));
    assert_eq!(expected.border_radius, BorderRadius::all(px(4)));
    app.world_mut()
        .entity_mut(entity)
        .insert(InteractionDisabled);
    app.update();
    switch_theme(&mut app, ThemeMode::Light);
    assert_eq!(*app.world().get::<Node>(entity).unwrap(), expected);
}

#[test]
fn foreground_propagates_to_children() {
    let mut app = app();
    app.add_plugins((WidgetryAssetPlugin, WidgetryIconPlugin));
    let button = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryButton Children [
                Text("Button"),
                @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()} },
                @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @color: {Some(Color::srgb(1.0, 0.0, 0.0))} },
            ]
        })
        .unwrap()
        .id();
    let children = app.world().get::<Children>(button).unwrap().to_vec();
    let child = children[0];
    advance_until(
        &mut app,
        Duration::from_secs(2),
        "Button icon image",
        |world| {
            children[1..]
                .iter()
                .all(|icon| world.get::<Children>(*icon).is_some())
        },
    )
    .unwrap();
    let image = app.world().get::<Children>(children[1]).unwrap()[0];
    let explicit_image = app.world().get::<Children>(children[2]).unwrap()[0];
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        switch_theme(&mut app, mode);
        for disabled in [false, true, false] {
            if disabled {
                app.world_mut()
                    .entity_mut(button)
                    .insert(InteractionDisabled);
            } else {
                app.world_mut()
                    .entity_mut(button)
                    .remove::<InteractionDisabled>();
            }
            app.update();
            let expected = if disabled {
                mode.colors().foreground_disabled
            } else {
                mode.colors().foreground
            };
            assert_eq!(
                app.world().get::<ForegroundColor>(child).unwrap().0,
                expected
            );
            assert_eq!(app.world().get::<TextColor>(child).unwrap().0, expected);
            assert_eq!(app.world().get::<ImageNode>(image).unwrap().color, expected);
            assert_eq!(
                app.world().get::<ImageNode>(explicit_image).unwrap().color,
                Color::srgb(1.0, 0.0, 0.0)
            );
            assert_eq!(
                app.world().get::<Children>(button).unwrap().to_vec(),
                children
            );
        }
    }
}

#[derive(Resource, Default)]
struct Activations(Vec<Entity>);

#[test]
fn pointer_activation_resumes_after_disabled() {
    let mut app = app();
    app.init_resource::<Activations>().add_observer(
        |event: On<Activate>, mut events: ResMut<Activations>| events.0.push(event.entity),
    );
    let button = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton Children [Text("click")] })
        .unwrap()
        .id();
    let other = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    app.update();
    for (disabled, expected) in [(false, 1), (true, 1), (false, 2)] {
        if disabled {
            app.world_mut()
                .entity_mut(button)
                .insert(InteractionDisabled);
        } else {
            app.world_mut()
                .entity_mut(button)
                .remove::<InteractionDisabled>();
        }
        app.update();
        press(&mut app, button);
        app.world_mut().trigger(primary_click(button));
        app.world_mut().flush();
        release(&mut app, button);
        assert_eq!(
            app.world().resource::<Activations>().0,
            vec![button; expected]
        );
        assert!(!app.world().resource::<Activations>().0.contains(&other));
        assert!(app.world().get::<Pressed>(button).is_none());
    }
}
