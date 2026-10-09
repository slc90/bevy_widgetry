//! State：normal/hover/pressed/disabled。
//! Add、Remove、Changed 与 WidgetryThemeChanged 驱动完整配色。
//! Guards：disabled 拒绝 pointer activation。
//! 重新启用恢复同一 root 的输入。
//! Invariants：disabled > pressed > hover > normal，style 不修改调用方 Node patch 或 children。
//! Coverage Map：background/priority 负责转换输出。
//! theme 负责立即刷新。
//! content 负责 Text/Icon 同帧传播。
//! pointer smoke 负责公开 Scene 到官方 Button observer 的 Activate 桥接，局部优先级归 style.rs。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    app::App,
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    prelude::*,
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Pressed},
    ui_widgets::{Activate, Button, ButtonPlugin},
};
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin};
use bevy_widgetry_test_utils::{
    advance_until, press, primary_click, release, scene_app, switch_theme,
};
use bevy_widgetry_theme::{WIDGETRY_DARK_THEME, WIDGETRY_LIGHT_THEME, WidgetryThemeMode};
use rstest::fixture;
use std::time::Duration;

#[test]
fn per_state_overrides_clear_to_current_theme_and_do_not_write_idle_outputs() {
    use bevy_widgetry_button::WidgetryButtonColorOverrides;
    #[derive(Resource, Default)]
    struct Writes(usize);
    let mut app = app();
    app.init_resource::<Writes>().add_systems(
        PostUpdate,
        (|colors: Query<
            (),
            (
                With<WidgetryButton>,
                Or<(
                    Changed<BackgroundColor>,
                    Changed<BorderColor>,
                    Changed<ResolvedForeground>,
                )>,
            ),
        >,
          mut writes: ResMut<Writes>| {
            writes.0 = colors.iter().count();
        })
        .after(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
    );
    let mut overrides = WidgetryButtonColorOverrides::default();
    overrides.normal.background = Some(Color::NONE);
    overrides.hovered.foreground = Some(Color::BLACK);
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryButton { @colors: overrides } Children [(Node Children [(Text("deep") bevy_widgetry_core::text::WidgetryText)])] }).unwrap().id();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::NONE
    );
    app.world_mut().entity_mut(root).insert(Hovered(true));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        WIDGETRY_DARK_THEME.button.hovered.background
    );
    assert_eq!(
        app.world().get::<ResolvedForeground>(root).unwrap().0,
        Color::BLACK
    );
    let branch = app.world().get::<Children>(root).unwrap()[0];
    let label = app.world().get::<Children>(branch).unwrap()[0];
    assert_eq!(app.world().get::<TextColor>(label).unwrap().0, Color::BLACK);
    WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
    app.world_mut().entity_mut(root).insert(Pressed);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        WIDGETRY_LIGHT_THEME.button.pressed.background
    );
    WidgetryButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    app.world_mut()
        .entity_mut(root)
        .remove::<Pressed>()
        .remove::<Hovered>()
        .insert(Hovered(false));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        WIDGETRY_LIGHT_THEME.button.normal.background
    );
    app.update();
    assert_eq!(app.world().resource::<Writes>().0, 0);
    assert!(WidgetryButtonColorOverrides::set_in_world(app.world_mut(), label, default()).is_err());
}

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
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.normal.background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn hover_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.hovered.background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn clearing_hover_restores_default(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.hovered.background,
            &node,
            &children,
        );

        app.world_mut().entity_mut(entity).insert(Hovered(false));

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.normal.background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn pressing_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.normal.background,
            &node,
            &children,
        );

        app.world_mut().entity_mut(entity).insert(Pressed);

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.pressed.background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn removing_pressed_restores_default(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(entity).unwrap().to_vec();
        let node = app.world().get::<Node>(entity).unwrap().clone();

        app.world_mut().entity_mut(entity).insert(Pressed);

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.pressed.background,
            &node,
            &children,
        );

        app.world_mut().entity_mut(entity).remove::<Pressed>();

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.normal.background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn disabling_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
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
            WIDGETRY_DARK_THEME.button.disabled.background,
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
            WIDGETRY_DARK_THEME.button.normal.background,
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
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
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
            WIDGETRY_DARK_THEME.button.pressed.background,
            &node,
            &children,
        );

        app.world_mut().entity_mut(entity).remove::<Pressed>();

        app.update();

        assert_transition_style(
            &app,
            entity,
            WIDGETRY_DARK_THEME.button.hovered.background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn removing_disabled_falls_back_to_pressed(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
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
            WIDGETRY_DARK_THEME.button.disabled.background,
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
            WIDGETRY_DARK_THEME.button.pressed.background,
            &node,
            &children,
        );
    }

    #[rstest]
    fn removing_disabled_falls_back_to_hover(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton Node { width: px(137), padding: UiRect::all(px(3)) } Children [Text("owned") bevy_widgetry_core::text::WidgetryText] })
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
            WIDGETRY_DARK_THEME.button.disabled.background,
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
            WIDGETRY_DARK_THEME.button.hovered.background,
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
        app.world().get::<ResolvedForeground>(button).unwrap().0,
        WIDGETRY_DARK_THEME.button.normal.foreground
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
        app.world().get::<ResolvedForeground>(entity).unwrap().0,
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
    let c = &WIDGETRY_DARK_THEME;
    let (border, foreground) = if background == c.button.disabled.background {
        (c.button.disabled.border, c.button.disabled.foreground)
    } else if background == c.button.pressed.background {
        (c.button.pressed.border, c.button.normal.foreground)
    } else if background == c.button.hovered.background {
        (c.button.hovered.border, c.button.normal.foreground)
    } else {
        (c.button.normal.border, c.button.normal.foreground)
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
    switch_theme(&mut app, WidgetryThemeMode::Light);
    let fresh = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    app.update();
    assert_style(
        &app,
        fresh,
        WIDGETRY_LIGHT_THEME.button.normal.background,
        WIDGETRY_LIGHT_THEME.button.normal.border,
        WIDGETRY_LIGHT_THEME.button.normal.foreground,
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
        WIDGETRY_LIGHT_THEME.button.hovered.background,
        WIDGETRY_LIGHT_THEME.button.hovered.border,
        WIDGETRY_LIGHT_THEME.button.normal.foreground,
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
        WIDGETRY_DARK_THEME.button.hovered.background,
        WIDGETRY_DARK_THEME.button.hovered.border,
        WIDGETRY_DARK_THEME.button.normal.foreground,
    );
    for mode in [WidgetryThemeMode::Light, WidgetryThemeMode::Dark] {
        switch_theme(&mut app, mode);
        let c = mode.colors();
        assert_style(
            &app,
            hovered,
            c.button.hovered.background,
            c.button.hovered.border,
            c.button.normal.foreground,
        );
        assert_style(
            &app,
            pressed,
            c.button.pressed.background,
            c.button.pressed.border,
            c.button.normal.foreground,
        );
        assert_style(
            &app,
            disabled,
            c.button.disabled.background,
            c.button.disabled.border,
            c.button.disabled.foreground,
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
    assert!(root.contains::<ResolvedForeground>());
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
    switch_theme(&mut app, WidgetryThemeMode::Light);
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
                Text("Button") bevy_widgetry_core::text::WidgetryText,
                @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()} },
                @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()}, @colors: { bevy_widgetry_core::icon::WidgetryIconColorOverrides { normal: bevy_widgetry_core::icon::WidgetryIconStateColorOverrides { foreground: Some(Color::srgb(1.0, 0.0, 0.0)) }, disabled: bevy_widgetry_core::icon::WidgetryIconStateColorOverrides { foreground: Some(Color::srgb(1.0, 0.0, 0.0)) } } } },
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
    for mode in [WidgetryThemeMode::Dark, WidgetryThemeMode::Light] {
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
                mode.colors().button.disabled.foreground
            } else {
                mode.colors().button.normal.foreground
            };
            assert_eq!(
                app.world()
                    .get::<bevy_widgetry_core::foreground::InheritedForeground>(child)
                    .unwrap()
                    .color,
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
        .spawn_scene(bsn! { @WidgetryButton Children [Text("click") bevy_widgetry_core::text::WidgetryText] })
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
