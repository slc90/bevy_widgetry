#![cfg(test)]

use bevy::{
    app::App,
    color::Color,
    ecs::entity::Entity,
    input_focus::{FocusCause, InputFocus},
    picking::hover::Hovered,
    text::{TextColor, TextCursorStyle},
    ui::{BackgroundColor, BorderColor, InteractionDisabled},
};
use bevy_widgetry_core::{DARK_THEME, LIGHT_THEME, ThemeMode};
use bevy_widgetry_test_utils::switch_theme;
use bevy_widgetry_text_field::{StyledTextField, StyledTextFieldPlugin};
use rstest::fixture;

#[fixture]
fn app() -> App {
    let mut app = App::new();

    // Gallery 中由 DefaultPlugins 提供；
    // 测试里我们只需要这个 Resource。
    app.init_resource::<InputFocus>();

    app.add_plugins(StyledTextFieldPlugin);

    app
}

fn assert_style(app: &App, entity: Entity, background: Color, border: Color, foreground: Color) {
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        background
    );

    assert_eq!(
        *app.world().get::<BorderColor>(entity).unwrap(),
        BorderColor::all(border)
    );

    assert_eq!(app.world().get::<TextColor>(entity).unwrap().0, foreground);

    assert_eq!(
        app.world().get::<TextCursorStyle>(entity).unwrap().color,
        foreground
    );
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
}

fn clear_focus(app: &mut App) {
    app.world_mut().resource_mut::<InputFocus>().clear();
}

// 初始化没有焦点或悬停的输入框，验证完整默认颜色及光标样式。
#[test]
fn spawned_text_field_uses_normal_style() {
    let mut app = app();

    let entity = app.world_mut().spawn(StyledTextField).id();

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background,
        DARK_THEME.control_border,
        DARK_THEME.foreground,
    );
}

// 在同一输入框上切换悬停和焦点，验证焦点优先且失焦后正确回退。
#[test]
fn hover_and_focus_follow_expected_priority() {
    let mut app = app();

    let entity = app.world_mut().spawn(StyledTextField).id();

    app.update();

    app.world_mut().entity_mut(entity).insert(Hovered(true));

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_hovered,
        DARK_THEME.control_border_hovered,
        DARK_THEME.foreground,
    );

    focus(&mut app, entity);

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_active,
        DARK_THEME.control_border_active,
        DARK_THEME.foreground,
    );

    clear_focus(&mut app);

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_hovered,
        DARK_THEME.control_border_hovered,
        DARK_THEME.foreground,
    );
}

// 保留焦点时禁用再启用输入框，验证禁用覆盖后能够恢复焦点样式。
#[test]
fn disabled_has_priority_and_removal_restores_focus() {
    let mut app = app();

    let entity = app.world_mut().spawn((StyledTextField, Hovered(true))).id();

    focus(&mut app, entity);

    app.world_mut()
        .entity_mut(entity)
        .insert(InteractionDisabled);

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_disabled,
        DARK_THEME.control_border_disabled,
        DARK_THEME.foreground_disabled,
    );

    app.world_mut()
        .entity_mut(entity)
        .remove::<InteractionDisabled>();

    app.update();

    // Focus 仍然存在，因此恢复 Focused，而不是 Hovered。
    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_active,
        DARK_THEME.control_border_active,
        DARK_THEME.foreground,
    );
}

// 多种输入框状态下切换主题，验证配色变化不破坏文本和交互状态。
#[test]
fn theme_switch_preserves_current_widget_states() {
    let mut app = app();

    let focused = app.world_mut().spawn(StyledTextField).id();

    let disabled = app
        .world_mut()
        .spawn((StyledTextField, InteractionDisabled))
        .id();

    focus(&mut app, focused);

    app.update();

    switch_theme(&mut app, ThemeMode::Light);

    assert_style(
        &app,
        focused,
        LIGHT_THEME.control_background_active,
        LIGHT_THEME.control_border_active,
        LIGHT_THEME.foreground,
    );

    assert_style(
        &app,
        disabled,
        LIGHT_THEME.control_background_disabled,
        LIGHT_THEME.control_border_disabled,
        LIGHT_THEME.foreground_disabled,
    );

    // 状态本身不能因为换 Theme 被破坏。
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));

    assert!(app.world().get::<InteractionDisabled>(disabled).is_some());
}

// 切换主题并检查选区与失焦选区颜色，验证两种选区状态均更新。
#[test]
fn selection_colors_follow_theme() {
    let mut app = app();

    let entity = app.world_mut().spawn(StyledTextField).id();
    app.update();

    let cursor = app.world().get::<TextCursorStyle>(entity).unwrap();

    assert_eq!(cursor.selection_color, DARK_THEME.text_selection);
    assert_eq!(
        cursor.unfocused_selection_color,
        DARK_THEME.text_selection_unfocused
    );

    switch_theme(&mut app, ThemeMode::Light);

    let cursor = app.world().get::<TextCursorStyle>(entity).unwrap();

    assert_eq!(cursor.selection_color, LIGHT_THEME.text_selection);
    assert_eq!(
        cursor.unfocused_selection_color,
        LIGHT_THEME.text_selection_unfocused
    );
}
