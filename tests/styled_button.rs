use bevy::{
    app::App,
    color::Color,
    picking::hover::Hovered,
    ui::{BackgroundColor, InteractionDisabled, Pressed},
};
use bevy_widgetry::styled_button::{StyledButton, StyledButtonPlugin};
use rstest::fixture;

const EXPECTED_DEFAULT_BG: Color = Color::srgb(0.30, 0.30, 0.30);
const EXPECTED_HOVERED_BG: Color = Color::srgb(0.20, 0.65, 0.95);
const EXPECTED_PRESSED_BG: Color = Color::srgb(0.85, 0.12, 0.12);
const EXPECTED_DISABLED_BG: Color = Color::srgb(0.15, 0.15, 0.15);

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

        assert_eq!(background.0, EXPECTED_DEFAULT_BG);
    }

    #[rstest]
    fn hover_updates_background(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        let background = app.world().get::<BackgroundColor>(entity).unwrap();

        assert_eq!(background.0, EXPECTED_HOVERED_BG);
    }

    #[rstest]
    fn clearing_hover_restores_default(mut app: App) {
        let entity = app.world_mut().spawn(StyledButton).id();

        app.world_mut().entity_mut(entity).insert(Hovered(true));

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            EXPECTED_HOVERED_BG,
        );

        app.world_mut().entity_mut(entity).insert(Hovered(false));

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            EXPECTED_DEFAULT_BG,
        );
    }

    #[rstest]
    fn pressing_updates_background(mut app: App) {
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

    #[rstest]
    fn removing_pressed_restores_default(mut app: App) {
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

    #[rstest]
    fn disabling_updates_background(mut app: App) {
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

    #[rstest]
    fn enabling_restores_default(mut app: App) {
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
            EXPECTED_PRESSED_BG,
        );

        app.world_mut().entity_mut(entity).remove::<Pressed>();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            EXPECTED_HOVERED_BG,
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
            EXPECTED_DISABLED_BG,
        );

        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            EXPECTED_PRESSED_BG,
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
            EXPECTED_DISABLED_BG,
        );

        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();

        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            EXPECTED_HOVERED_BG,
        );
    }
}

mod foreground {
    use bevy::{app::Propagate, ecs::hierarchy::ChildOf, text::TextColor};
    use bevy_widgetry::styled_button::ForegroundColor;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn foreground_color_propagates_to_child(mut app: App) {
        let button = app.world_mut().spawn(StyledButton).id();

        let child = app.world_mut().spawn(ChildOf(button)).id();

        app.world_mut()
            .entity_mut(button)
            .insert(Propagate(ForegroundColor(Color::WHITE)));

        app.update();

        let foreground = app.world().get::<ForegroundColor>(child).unwrap();

        assert_eq!(foreground.0, Color::WHITE);
    }

    #[rstest]
    fn foreground_color_updates_text_color(mut app: App) {
        let button = app.world_mut().spawn(StyledButton).id();

        let child = app
            .world_mut()
            .spawn((ChildOf(button), TextColor(Color::BLACK)))
            .id();

        app.world_mut()
            .entity_mut(button)
            .insert(Propagate(ForegroundColor(Color::WHITE)));

        app.update();

        let text_color = app.world().get::<TextColor>(child).unwrap();

        assert_eq!(text_color.0, Color::WHITE);
    }

    #[rstest]
    fn foreground_color_change_updates_text_color(mut app: App) {
        let button = app.world_mut().spawn(StyledButton).id();

        let child = app
            .world_mut()
            .spawn((ChildOf(button), TextColor(Color::BLACK)))
            .id();

        app.world_mut()
            .entity_mut(button)
            .insert(Propagate(ForegroundColor(Color::WHITE)));

        app.update();

        assert_eq!(app.world().get::<TextColor>(child).unwrap().0, Color::WHITE,);

        app.world_mut()
            .entity_mut(button)
            .insert(Propagate(ForegroundColor(Color::BLACK)));

        app.update();

        assert_eq!(app.world().get::<TextColor>(child).unwrap().0, Color::BLACK,);
    }
}
