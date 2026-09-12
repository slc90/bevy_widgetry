#![cfg(test)]

use bevy::{
    app::{App, Propagate},
    picking::hover::Hovered,
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Pressed},
};
use bevy_widgetry_button::{StyledButton, StyledButtonPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::{DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeMode};
use bevy_widgetry_test_utils::switch_theme;
use rstest::fixture;

#[fixture]
fn app() -> App {
    let mut app = App::new();
    app.set_default_font(bevy::text::FontSource::Monospace);
    app.add_plugins(StyledButtonPlugin);
    app
}

mod background {
    use super::*;
    use rstest::rstest;

    // 新按钮尚无交互状态，首次更新应使用默认背景。
    #[rstest]
    fn spawned_button_is_default(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

        app.update();

        let background = app.world().get::<BackgroundColor>(entity).unwrap();

        assert_eq!(background.0, DARK_THEME.control_background);
    }

    // 已有按钮进入悬停，验证变更检测会应用悬停配色。
    #[rstest]
    fn hover_updates_background(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        let background = app.world().get::<BackgroundColor>(entity).unwrap();

        assert_eq!(background.0, DARK_THEME.control_background_hovered);
    }

    // 同一按钮先悬停再离开，验证清除状态不会残留旧背景。
    #[rstest]
    fn clearing_hover_restores_default(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

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

    // 为已有按钮添加 Pressed，验证按压配色覆盖默认配色。
    #[rstest]
    fn pressing_updates_background(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

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

    // 按下状态被移除且没有悬停，验证移除事件恢复默认样式。
    #[rstest]
    fn removing_pressed_restores_default(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

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

    // 禁用已有按钮并恢复，验证两次状态转换都更新背景。
    #[rstest]
    fn disabling_updates_background(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

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

    // 按下和悬停并存时移除按下，验证低优先级悬停仍然有效。
    #[rstest]
    fn removing_pressed_falls_back_to_hover(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

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

    // 禁用与按下并存时重新启用，验证现存按下状态没有丢失。
    #[rstest]
    fn removing_disabled_falls_back_to_pressed(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

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

    // 禁用与悬停并存时重新启用，验证无需重新进入即可恢复悬停色。
    #[rstest]
    fn removing_disabled_falls_back_to_hover(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

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

// 创建带文本的按钮，验证样式初始化提供可传播的默认前景色。
#[test]
fn styled_button_sets_default_foreground() {
    let mut app = App::new();
    app.set_default_font(bevy::text::FontSource::Monospace);
    app.add_plugins(StyledButtonPlugin);
    let button = app.world_mut().spawn(StyledButton).id();
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

// 在已有交互组件上后加样式，验证首次初始化读取当前主题和未变更的状态。
#[test]
fn newly_styled_button_uses_current_theme() {
    let mut app = app();
    switch_theme(&mut app, ThemeMode::Light);
    let fresh = app.world_mut().spawn(StyledButton).id();
    app.update();
    assert_style(
        &app,
        fresh,
        LIGHT_THEME.control_background,
        LIGHT_THEME.control_border,
        LIGHT_THEME.foreground,
    );
    // 添加 StyledButton 前已存在 Hovered，且其变更标记已被清除。
    let entity = app.world_mut().spawn(Hovered(true)).id();
    app.update();
    app.world_mut().entity_mut(entity).insert(StyledButton);
    app.update();
    assert_style(
        &app,
        entity,
        LIGHT_THEME.control_background_hovered,
        LIGHT_THEME.control_border_hovered,
        LIGHT_THEME.foreground,
    );
}

// 多种交互状态下切换主题，验证颜色立即改变而状态组件不变。
#[test]
fn theme_switch_immediately_preserves_button_states() {
    let mut app = app();
    let hovered = app.world_mut().spawn((StyledButton, Hovered(true))).id();
    let pressed = app.world_mut().spawn((StyledButton, Pressed)).id();
    let disabled = app
        .world_mut()
        .spawn((StyledButton, InteractionDisabled))
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
