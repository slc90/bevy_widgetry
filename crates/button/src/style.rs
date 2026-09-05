use bevy::{
    app::{App, Plugin, Propagate, Update},
    color::Color,
    ecs::{
        component::Component,
        lifecycle::RemovedComponents,
        query::{Added, Changed, Has, Or, With},
        system::Query,
    },
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Node, Pressed, UiRect, px},
    ui_widgets::Button,
    utils::default,
};
use bevy_widgetry_core::{ForegroundColor, ForegroundColorPlugin};

const BUTTON_BG_DEFAULT: Color = Color::srgb(0.30, 0.30, 0.30);
const BUTTON_BG_HOVERED: Color = Color::srgb(0.20, 0.65, 0.95);
const BUTTON_BG_PRESSED: Color = Color::srgb(0.85, 0.12, 0.12);
const BUTTON_BG_DISABLED: Color = Color::srgb(0.15, 0.15, 0.15);

const BUTTON_BORDER: Color = Color::srgb(0.0, 0.8, 1.0);

#[derive(Component, Default)]
#[require(
    Button,
    Hovered,
    TabIndex(-1),
    Node = styled_button_node(),
    BackgroundColor = styled_button_background(),
    BorderColor = styled_button_border(),
    Propagate::<ForegroundColor> = Propagate(ForegroundColor::default()),
)]
pub struct StyledButton;

fn styled_button_node() -> Node {
    Node {
        padding: UiRect::axes(px(12), px(6)),
        border: UiRect::all(px(1)),
        ..default()
    }
}

fn styled_button_background() -> BackgroundColor {
    BackgroundColor(BUTTON_BG_DEFAULT)
}

fn styled_button_border() -> BorderColor {
    BorderColor::all(BUTTON_BORDER)
}

fn update_styled_button_background_changed(
    mut query: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
        ),
        (
            With<StyledButton>,
            Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
        ),
    >,
) {
    for (hovered, pressed, disabled, mut background) in &mut query {
        background.0 = resolve_button_background(hovered.0, pressed, disabled);
    }
}

fn update_styled_button_background_removed(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
        ),
        With<StyledButton>,
    >,
) {
    for entity in removed_pressed.read().chain(removed_disabled.read()) {
        if let Ok((hovered, pressed, disabled, mut background)) = query.get_mut(entity) {
            background.0 = resolve_button_background(hovered.0, pressed, disabled);
        }
    }
}

fn resolve_button_background(hovered: bool, pressed: bool, disabled: bool) -> Color {
    if disabled {
        BUTTON_BG_DISABLED
    } else if pressed {
        BUTTON_BG_PRESSED
    } else if hovered {
        BUTTON_BG_HOVERED
    } else {
        BUTTON_BG_DEFAULT
    }
}

pub struct StyledButtonPlugin;

impl Plugin for StyledButtonPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }

        app.add_systems(
            Update,
            (
                update_styled_button_background_changed,
                update_styled_button_background_removed,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case::default(false, false, false, BUTTON_BG_DEFAULT)]
    #[case::hovered(true, false, false, BUTTON_BG_HOVERED)]
    #[case::pressed(false, true, false, BUTTON_BG_PRESSED)]
    #[case::disabled(false, false, true, BUTTON_BG_DISABLED)]
    #[case::pressed(true, true, false, BUTTON_BG_PRESSED)]
    #[case::disabled(true, false, true, BUTTON_BG_DISABLED)]
    #[case::disabled(false, true, true, BUTTON_BG_DISABLED)]
    #[case::disabled(true, true, true, BUTTON_BG_DISABLED)]
    fn resolves_button_background(
        #[case] hovered: bool,
        #[case] pressed: bool,
        #[case] disabled: bool,
        #[case] expected: Color,
    ) {
        assert_eq!(
            resolve_button_background(hovered, pressed, disabled),
            expected
        );
    }
}
