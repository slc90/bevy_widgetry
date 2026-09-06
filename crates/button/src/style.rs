use bevy::{
    app::{App, Plugin, Propagate, Update},
    color::Color,
    ecs::{
        component::Component,
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        system::{Query, Res},
    },
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Node, Pressed, UiRect, px},
    ui_widgets::Button,
    utils::default,
};

use bevy_widgetry_core::{
    ColorTheme, ForegroundColor, ForegroundColorPlugin, ThemeChanged, ThemeMode, ThemePlugin,
};

#[derive(Component, Default)]
#[require(
    Button,
    Hovered,
    TabIndex(-1),
    Node = styled_button_node(),
    BackgroundColor,
    BorderColor,
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

#[derive(Debug, PartialEq)]
struct ButtonStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

fn resolve_button_style(
    colors: &ColorTheme,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> ButtonStyle {
    let (background, border) = if disabled {
        (
            colors.control_background_disabled,
            colors.control_border_disabled,
        )
    } else if pressed {
        (
            colors.control_background_pressed,
            colors.control_border_pressed,
        )
    } else if hovered {
        (
            colors.control_background_hovered,
            colors.control_border_hovered,
        )
    } else {
        (colors.control_background, colors.control_border)
    };
    ButtonStyle {
        background,
        border,
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
    }
}

type ButtonStyleData = (
    &'static Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut Propagate<ForegroundColor>,
);

fn apply_button_style(
    colors: &ColorTheme,
    (hovered, pressed, disabled, mut background, mut border, mut foreground): <ButtonStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let style = resolve_button_style(colors, hovered.0, pressed, disabled);
    background.0 = style.background;
    *border = BorderColor::all(style.border);
    foreground.0 = ForegroundColor(style.foreground);
}

fn update_styled_button_style_changed(
    mode: Res<ThemeMode>,
    mut query: Query<
        ButtonStyleData,
        (
            With<StyledButton>,
            Or<(
                Added<StyledButton>,
                Changed<Hovered>,
                Added<Pressed>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
) {
    for item in &mut query {
        apply_button_style(mode.colors(), item);
    }
}

fn update_styled_button_style_removed(
    mode: Res<ThemeMode>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<ButtonStyleData, With<StyledButton>>,
) {
    for entity in removed_pressed.read().chain(removed_disabled.read()) {
        if let Ok(item) = query.get_mut(entity) {
            apply_button_style(mode.colors(), item);
        }
    }
}

fn refresh_button_theme(
    event: On<ThemeChanged>,
    mut query: Query<ButtonStyleData, With<StyledButton>>,
) {
    for item in &mut query {
        apply_button_style(event.mode.colors(), item);
    }
}

pub struct StyledButtonPlugin;

impl Plugin for StyledButtonPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }

        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_observer(refresh_button_theme);
        app.add_systems(
            Update,
            (
                update_styled_button_style_changed,
                update_styled_button_style_removed,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_THEME: ColorTheme = ColorTheme {
        foreground: Color::srgb_u8(1, 0, 0),
        foreground_disabled: Color::srgb_u8(2, 0, 0),
        control_background: Color::srgb_u8(3, 0, 0),
        control_background_hovered: Color::srgb_u8(4, 0, 0),
        control_background_pressed: Color::srgb_u8(5, 0, 0),
        control_background_active: Color::srgb_u8(6, 0, 0),
        control_background_disabled: Color::srgb_u8(7, 0, 0),
        control_border: Color::srgb_u8(8, 0, 0),
        control_border_hovered: Color::srgb_u8(9, 0, 0),
        control_border_pressed: Color::srgb_u8(10, 0, 0),
        control_border_active: Color::srgb_u8(11, 0, 0),
        control_border_disabled: Color::srgb_u8(12, 0, 0),
        popup_background: Color::srgb_u8(13, 0, 0),
        popup_border: Color::srgb_u8(14, 0, 0),
        item_background_hovered: Color::srgb_u8(15, 0, 0),
        item_background_selected: Color::srgb_u8(16, 0, 0),
    };

    #[test]
    fn resolves_complete_style_with_state_priority() {
        let c = &TEST_THEME;
        for (hovered, pressed, disabled, background, border, foreground) in [
            (
                false,
                false,
                false,
                c.control_background,
                c.control_border,
                c.foreground,
            ),
            (
                true,
                false,
                false,
                c.control_background_hovered,
                c.control_border_hovered,
                c.foreground,
            ),
            (
                true,
                true,
                false,
                c.control_background_pressed,
                c.control_border_pressed,
                c.foreground,
            ),
            (
                true,
                true,
                true,
                c.control_background_disabled,
                c.control_border_disabled,
                c.foreground_disabled,
            ),
        ] {
            assert_eq!(
                resolve_button_style(c, hovered, pressed, disabled),
                ButtonStyle {
                    background,
                    border,
                    foreground
                }
            );
        }
    }
}
