use crate::text_field::{ReadOnly, TextFieldBase, WidgetryTextField};
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{TextColor, TextCursorStyle};
use bevy::ui::InteractionDisabled;
use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged, WidgetryThemeMode};

type TextFieldStyleData = (
    Entity,
    &'static crate::colors::ColorState,
    Has<ReadOnly>,
    &'static Hovered,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut TextColor,
    &'static mut TextCursorStyle,
);

type ChangedTextFieldStyleQuery<'w, 's> = Query<
    'w,
    's,
    TextFieldStyleData,
    (
        With<TextFieldBase>,
        Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTextField>>,
        Or<(
            Added<TextFieldBase>,
            Changed<Hovered>,
            Changed<crate::colors::ColorState>,
            Added<InteractionDisabled>,
        )>,
    ),
>;

fn apply_text_field_style(
    colors: &WidgetryTheme,
    focused_entity: Option<Entity>,
    (
        entity,
        overrides,
        read_only,
        hovered,
        disabled,
        background,
        border,
        text,
        cursor,
    ): <TextFieldStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let focused = focused_entity == Some(entity);

    let colors = overrides.0.resolve(&colors.text_field);
    let colors = if read_only {
        colors.read_only
    } else {
        colors.editable
    };
    let state = if disabled {
        colors.disabled
    } else if focused {
        colors.focused
    } else if hovered.0 {
        colors.hovered
    } else {
        colors.normal
    };
    apply_outputs(state, (background, border, text, cursor));
}

fn apply_outputs(
    state: bevy_widgetry_theme::WidgetryTextFieldStateColors,
    (mut background, mut border, mut text, mut cursor): (
        bevy::prelude::Mut<BackgroundColor>,
        bevy::prelude::Mut<BorderColor>,
        bevy::prelude::Mut<TextColor>,
        bevy::prelude::Mut<TextCursorStyle>,
    ),
) {
    background.set_if_neq(BackgroundColor(state.background));
    border.set_if_neq(BorderColor::all(state.border));
    text.set_if_neq(TextColor(state.foreground));
    if cursor.color != state.caret
        || cursor.selection_color != state.selection_background
        || cursor.unfocused_selection_color != state.unfocused_selection_background
        || cursor.selected_text_color != Some(state.selection_foreground)
    {
        cursor.color = state.caret;
        cursor.selection_color = state.selection_background;
        cursor.unfocused_selection_color = state.unfocused_selection_background;
        cursor.selected_text_color = Some(state.selection_foreground);
    }
}

pub(super) fn update_widgetry_text_field_style_changed(
    mode: Res<WidgetryThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: ChangedTextFieldStyleQuery<'_, '_>,
) {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item);
    }
}

pub(super) fn update_widgetry_text_field_style_focus_changed(
    mode: Res<WidgetryThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: Query<
        TextFieldStyleData,
        (
            With<TextFieldBase>,
            Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTextField>>,
        ),
    >,
) {
    if !input_focus.is_changed() {
        return;
    }

    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item);
    }
}

pub(super) fn update_widgetry_text_field_style_removed(
    mode: Res<WidgetryThemeMode>,
    input_focus: Res<InputFocus>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<
        TextFieldStyleData,
        (
            With<TextFieldBase>,
            Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTextField>>,
        ),
    >,
) {
    let focused = input_focus.get();

    for entity in removed_disabled.read() {
        if let Ok(item) = query.get_mut(entity) {
            apply_text_field_style(mode.colors(), focused, item);
        }
    }
}

pub(super) fn refresh_text_field_theme(
    _event: On<WidgetryThemeChanged>,
    mut commands: bevy::prelude::Commands,
) {
    commands.queue(
        |world: &mut bevy::prelude::World| -> Result<(), bevy::prelude::BevyError> {
            let theme = world.resource::<WidgetryThemeMode>().colors().text_field;
            let roots = world
                .query_filtered::<Entity, (
                    With<TextFieldBase>,
                    Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTextField>>,
                )>()
                .iter(world)
                .collect::<Vec<_>>();
            for root in roots {
                let colors =
                    crate::WidgetryTextFieldColorOverrides::get(world, root)?.resolve(&theme);
                apply_owned_text_field_colors(world, root, &colors)?;
            }
            Ok(())
        },
    );
}

pub fn apply_owned_text_field_colors(
    world: &mut bevy::prelude::World,
    entity: Entity,
    colors: &bevy_widgetry_theme::WidgetryTextFieldColors,
) -> Result<(), bevy::prelude::BevyError> {
    use bevy::prelude::*;
    let colors = if world.get::<ReadOnly>(entity).is_some() {
        colors.read_only
    } else {
        colors.editable
    };
    let focused = world.get_resource::<InputFocus>().and_then(InputFocus::get) == Some(entity);
    let state = if world.get::<InteractionDisabled>(entity).is_some() {
        colors.disabled
    } else if focused {
        colors.focused
    } else if world.get::<Hovered>(entity).is_some_and(|h| h.0) {
        colors.hovered
    } else {
        colors.normal
    };
    let invalid = || {
        bevy_widgetry_internal_log::widgetry_error!(?entity, "TextField 缺失托管颜色输出");
        BevyError::error("TextField 缺失托管颜色输出")
    };
    let mut query = world.query::<(
        &mut BackgroundColor,
        &mut BorderColor,
        &mut TextColor,
        &mut TextCursorStyle,
    )>();
    let outputs = query.get_mut(world, entity).map_err(|_| invalid())?;
    apply_outputs(state, outputs);
    Ok(())
}
