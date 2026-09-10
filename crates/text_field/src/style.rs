use crate::{TextField, TextFieldPlugin};
use bevy::{
    app::{App, Plugin, Update},
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        system::{Query, Res},
    },
    input_focus::InputFocus,
    picking::hover::Hovered,
    text::{TextColor, TextCursorStyle},
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Node, UiRect, px},
    utils::default,
};
use bevy_widgetry_core::{ColorTheme, ThemeChanged, ThemeMode, ThemePlugin};

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

type TextFieldStyleData = (
    Entity,
    &'static Hovered,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut TextColor,
    &'static mut TextCursorStyle,
);

fn apply_text_field_style(
    colors: &ColorTheme,
    focused_entity: Option<Entity>,
    (
        entity,
        hovered,
        disabled,
        mut background,
        mut border,
        mut text,
        mut cursor,
    ): <TextFieldStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let focused = focused_entity == Some(entity);

    let style = resolve_text_field_style(colors, hovered.0, focused, disabled);

    background.0 = style.background;
    *border = BorderColor::all(style.border);
    text.0 = style.foreground;
    cursor.color = style.foreground;
    cursor.selection_color = colors.text_selection;
    cursor.unfocused_selection_color = colors.text_selection_unfocused;
    cursor.selected_text_color = None;
}

fn update_styled_text_field_style_changed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: Query<
        TextFieldStyleData,
        (
            With<StyledTextField>,
            Or<(
                Added<StyledTextField>,
                Changed<Hovered>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
) {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item);
    }
}

fn update_styled_text_field_style_focus_changed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: Query<TextFieldStyleData, With<StyledTextField>>,
) {
    if !input_focus.is_changed() {
        return;
    }

    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item);
    }
}

fn update_styled_text_field_style_removed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<TextFieldStyleData, With<StyledTextField>>,
) {
    let focused = input_focus.get();

    for entity in removed_disabled.read() {
        if let Ok(item) = query.get_mut(entity) {
            apply_text_field_style(mode.colors(), focused, item);
        }
    }
}

fn refresh_text_field_theme(
    event: On<ThemeChanged>,
    input_focus: Res<InputFocus>,
    mut query: Query<TextFieldStyleData, With<StyledTextField>>,
) {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(event.mode.colors(), focused, item);
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

        app.add_observer(refresh_text_field_theme);

        app.add_systems(
            Update,
            (
                update_styled_text_field_style_changed,
                update_styled_text_field_style_focus_changed,
                update_styled_text_field_style_removed,
            ),
        );
    }
}
