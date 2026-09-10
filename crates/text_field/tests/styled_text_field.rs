use bevy::{
    app::App,
    color::Color,
    ecs::entity::Entity,
    input_focus::{FocusCause, InputFocus},
    picking::hover::Hovered,
    text::{TextColor, TextCursorStyle},
    ui::{BackgroundColor, BorderColor, InteractionDisabled},
};

use bevy_widgetry_core::{DARK_THEME, LIGHT_THEME, ThemeChanged, ThemeMode};
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

fn switch_theme(app: &mut App, mode: ThemeMode) {
    *app.world_mut().resource_mut::<ThemeMode>() = mode;

    app.world_mut().trigger(ThemeChanged { mode });
}

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

#[test]
fn hover_and_focus_follow_expected_priority() {
    let mut app = app();

    let entity = app.world_mut().spawn(StyledTextField).id();

    app.update();

    // Normal -> Hovered
    app.world_mut().entity_mut(entity).insert(Hovered(true));

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_hovered,
        DARK_THEME.control_border_hovered,
        DARK_THEME.foreground,
    );

    // Hovered -> Focused
    focus(&mut app, entity);

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_active,
        DARK_THEME.control_border_active,
        DARK_THEME.foreground,
    );

    // Focused -> Hovered
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

#[test]
fn disabled_has_priority_and_removal_restores_focus() {
    let mut app = app();

    let entity = app.world_mut().spawn((StyledTextField, Hovered(true))).id();

    focus(&mut app, entity);

    app.world_mut()
        .entity_mut(entity)
        .insert(InteractionDisabled);

    app.update();

    // Disabled > Focused > Hovered
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
