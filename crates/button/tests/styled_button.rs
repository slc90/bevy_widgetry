use bevy::{
    app::App,
    picking::hover::Hovered,
    ui::{BackgroundColor, InteractionDisabled, Pressed},
};

use bevy_widgetry_button::{StyledButton, StyledButtonPlugin};
use bevy_widgetry_core::{DARK_THEME, LIGHT_THEME, ThemeChanged, ThemeMode};

use rstest::fixture;

#[fixture]
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(StyledButtonPlugin);
    app
}

mod background {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn spawned_button_is_default(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

        app.update();

        let background = app.world().get::<BackgroundColor>(entity).unwrap();

        assert_eq!(background.0, DARK_THEME.control_background);
    }

    #[rstest]
    fn hover_updates_background(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        let background = app.world().get::<BackgroundColor>(entity).unwrap();

        assert_eq!(background.0, DARK_THEME.control_background_hovered);
    }

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
    use rstest::rstest;

    use super::*;

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

#[test]
fn styled_button_sets_default_foreground() {
    use bevy::app::Propagate;
    use bevy_widgetry_core::ForegroundColor;
    let mut app = App::new();
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

fn switch_theme(app: &mut App, mode: ThemeMode) {
    *app.world_mut().resource_mut::<ThemeMode>() = mode;
    app.world_mut().trigger(ThemeChanged { mode });
}

fn assert_style(
    app: &App,
    entity: bevy::ecs::entity::Entity,
    background: bevy::color::Color,
    border: bevy::color::Color,
    foreground: bevy::color::Color,
) {
    use bevy::{app::Propagate, ui::BorderColor};
    use bevy_widgetry_core::ForegroundColor;
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
    // Hovered already exists and is no longer Changed when StyledButton is added.
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
