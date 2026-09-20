#![cfg(test)]

use bevy::{
    app::{App, Propagate},
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    prelude::*,
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Pressed},
    ui_widgets::{Button, ButtonPlugin},
};
use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::{DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeMode};
use bevy_widgetry_test_utils::{scene_app, switch_theme};
use rstest::fixture;

/// 复用 headless Scene 环境并装配被测 Button。
#[fixture]
fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    app
}

mod background {
    use super::*;
    use rstest::rstest;

    // 新 Button 尚无 interaction state，首次更新应使用默认背景。
    #[rstest]
    fn spawned_button_is_default(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.update();

        let background = app.world().get::<BackgroundColor>(entity).unwrap();

        assert_eq!(background.0, DARK_THEME.control_background);
    }

    // 已有 Button 进入 hover，验证 change detection 会应用 hover 配色。
    #[rstest]
    fn hover_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        let background = app.world().get::<BackgroundColor>(entity).unwrap();

        assert_eq!(background.0, DARK_THEME.control_background_hovered);
    }

    // 同一 Button 先 hover 再离开，验证清除 state 不会残留旧背景。
    #[rstest]
    fn clearing_hover_restores_default(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_hovered,
        );

        app.world_mut().entity_mut(entity).insert(Hovered(false));

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background,
        );
    }

    // 为已有 Button 添加 Pressed，验证 pressed 配色覆盖默认配色。
    #[rstest]
    fn pressing_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background,
        );

        app.world_mut().entity_mut(entity).insert(Pressed);

        app.update();

        let background = app.world().get::<BackgroundColor>(entity).unwrap();

        assert_eq!(background.0, DARK_THEME.control_background_pressed);
    }

    // pressed state 被移除且没有 hover，验证 Remove event 恢复默认 style。
    #[rstest]
    fn removing_pressed_restores_default(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.world_mut().entity_mut(entity).insert(Pressed);

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_pressed,
        );

        app.world_mut().entity_mut(entity).remove::<Pressed>();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background,
        );
    }

    // 禁用已有 Button 并恢复，验证两次 state transition 都更新背景。
    #[rstest]
    fn disabling_updates_background(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.world_mut()
            .entity_mut(entity)
            .insert(InteractionDisabled);

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_disabled,
        );

        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background,
        );
    }
}

mod background_priority {
    use super::*;
    use rstest::rstest;

    // pressed 和 hover 并存时移除 pressed，验证低优先级 hover 仍然有效。
    #[rstest]
    fn removing_pressed_falls_back_to_hover(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.world_mut()
            .entity_mut(entity)
            .insert(Hovered(true))
            .insert(Pressed);

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_pressed,
        );

        app.world_mut().entity_mut(entity).remove::<Pressed>();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_hovered,
        );
    }

    // disabled 与 pressed 并存时重新启用，验证现存 pressed state 没有丢失。
    #[rstest]
    fn removing_disabled_falls_back_to_pressed(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.world_mut()
            .entity_mut(entity)
            .insert(Pressed)
            .insert(InteractionDisabled);

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_disabled,
        );

        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_pressed,
        );
    }

    // disabled 与 hover 并存时重新启用，验证无需重新进入即可恢复 hover 颜色。
    #[rstest]
    fn removing_disabled_falls_back_to_hover(mut app: App) {
        let entity = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();

        app.world_mut()
            .entity_mut(entity)
            .insert(Hovered(true))
            .insert(InteractionDisabled);

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_disabled,
        );

        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            DARK_THEME.control_background_hovered,
        );
    }
}

// 创建带文本的 Button，验证 style 初始化提供可传播的默认 foreground color。
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

// 在切换 theme 后创建 Button，验证 Scene 初始化读取当前 theme 和附加 state。
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

// 多种 interaction state 下切换 theme，验证颜色立即改变而 state component 不变。
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

// BSN 展开提供完整默认外壳，不限定消费者的尺寸和内容排布。
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

// 单独注册 style plugin 或预先注册官方行为 plugin，都只保留一份官方 Button 行为。
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

// 局部几何 patch 保留未覆盖的外壳默认值，theme 更新也不改变 layout。
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

// child 文本继承 Button foreground color，disabled、恢复和 theme 切换均沿真实 hierarchy 传播。
#[test]
fn foreground_propagates_to_children() {
    let mut app = app();
    let button = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryButton Children [Text("Button")]
        })
        .unwrap()
        .id();
    let child = app.world().get::<Children>(button).unwrap()[0];
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
        }
    }
}
