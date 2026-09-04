use bevy::{
    app::App,
    color::Color,
    picking::hover::Hovered,
    ui::{BackgroundColor, InteractionDisabled, Pressed},
};
use bevy_widgetry::styled_button::{StyledButton, StyledButtonPlugin};

const EXPECTED_DEFAULT_BG: Color = Color::srgb(0.30, 0.30, 0.30);
const EXPECTED_HOVERED_BG: Color = Color::srgb(0.20, 0.65, 0.95);
const EXPECTED_PRESSED_BG: Color = Color::srgb(0.85, 0.12, 0.12);
const EXPECTED_DISABLED_BG: Color = Color::srgb(0.15, 0.15, 0.15);

#[test]
fn hovered_updates_background() {
    let mut app = App::new();
    app.add_plugins(StyledButtonPlugin);

    let entity = app.world_mut().spawn(StyledButton).id();

    app.world_mut().entity_mut(entity).insert(Hovered(true));

    app.update();

    let background = app.world().get::<BackgroundColor>(entity).unwrap();

    assert_eq!(background.0, EXPECTED_HOVERED_BG);
}

#[test]
fn adding_pressed_updates_background() {
    let mut app = App::new();
    app.add_plugins(StyledButtonPlugin);

    let entity = app.world_mut().spawn(StyledButton).id();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        EXPECTED_DEFAULT_BG,
    );

    app.world_mut().entity_mut(entity).insert(Pressed);

    app.update();

    let background = app.world().get::<BackgroundColor>(entity).unwrap();

    assert_eq!(background.0, EXPECTED_PRESSED_BG);
}

#[test]
fn removing_pressed_restores_background() {
    let mut app = App::new();
    app.add_plugins(StyledButtonPlugin);

    let entity = app.world_mut().spawn(StyledButton).id();

    app.world_mut().entity_mut(entity).insert(Pressed);

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        EXPECTED_PRESSED_BG,
    );

    app.world_mut().entity_mut(entity).remove::<Pressed>();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        EXPECTED_DEFAULT_BG,
    );
}

#[test]
fn interaction_disabled_updates_background() {
    let mut app = App::new();
    app.add_plugins(StyledButtonPlugin);

    let entity = app.world_mut().spawn(StyledButton).id();

    app.world_mut()
        .entity_mut(entity)
        .insert(InteractionDisabled);

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        EXPECTED_DISABLED_BG,
    );

    app.world_mut()
        .entity_mut(entity)
        .remove::<InteractionDisabled>();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        EXPECTED_DEFAULT_BG,
    );
}
