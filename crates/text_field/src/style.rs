use crate::{TextField, TextFieldPlugin};
use bevy::{
    app::{App, Plugin, Update},
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        query::{Has, With},
        system::{Query, Res},
    },
    input_focus::InputFocus,
    picking::hover::Hovered,
    text::{TextColor, TextCursorStyle},
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Node, UiRect, px},
    utils::default,
};
use bevy_widgetry_core::{ColorTheme, ThemeMode, ThemePlugin};

#[derive(Component, Default)]
#[require(
    TextField,
    Hovered,
    Node = styled_text_field_node(),
    BackgroundColor,
    BorderColor,
)]
pub struct StyledTextField;

fn styled_text_field_node() -> Node {
    Node {
        width: px(240),
        height: px(40),
        padding: UiRect::axes(px(10), px(6)),
        border: UiRect::all(px(1)),
        ..default()
    }
}

#[derive(Debug, PartialEq)]
struct TextFieldStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

fn resolve_text_field_style(
    colors: &ColorTheme,
    hovered: bool,
    focused: bool,
    disabled: bool,
) -> TextFieldStyle {
    let (background, border) = if disabled {
        (
            colors.control_background_disabled,
            colors.control_border_disabled,
        )
    } else if focused {
        (
            colors.control_background_active,
            colors.control_border_active,
        )
    } else if hovered {
        (
            colors.control_background_hovered,
            colors.control_border_hovered,
        )
    } else {
        (colors.control_background, colors.control_border)
    };

    TextFieldStyle {
        background,
        border,
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
    }
}

fn update_text_field_style(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: Query<
        (
            Entity,
            &Hovered,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut TextColor,
            &mut TextCursorStyle,
        ),
        With<StyledTextField>,
    >,
) {
    let colors = mode.colors();

    for (entity, hovered, disabled, mut background, mut border, mut text, mut cursor) in &mut query
    {
        let focused = input_focus.get() == Some(entity);

        let style = resolve_text_field_style(colors, hovered.0, focused, disabled);

        background.0 = style.background;
        *border = BorderColor::all(style.border);
        text.0 = style.foreground;

        cursor.color = style.foreground;
    }
}

pub struct StyledTextFieldPlugin;

impl Plugin for StyledTextFieldPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TextFieldPlugin>() {
            app.add_plugins(TextFieldPlugin);
        }

        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }

        app.add_systems(Update, update_text_field_style);
    }
}
