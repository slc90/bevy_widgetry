use crate::text_field::{Parts, ReadOnly, TextFieldBase, WidgetryTextField};
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
    &'static Parts,
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
        _entity,
        overrides,
        read_only,
        hovered,
        disabled,
        parts,
    ): <TextFieldStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    frames: &mut Query<(&mut BackgroundColor, &mut BorderColor)>,
    editors: &mut Query<(&mut TextColor, &mut TextCursorStyle)>,
) -> Result {
    let focused = focused_entity == Some(parts.editor);

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
    let (background, border) = frames
        .get_mut(parts.frame)
        .map_err(|_| invalid_target(parts.frame))?;
    let (text, cursor) = editors
        .get_mut(parts.editor)
        .map_err(|_| invalid_target(parts.editor))?;
    apply_outputs(state, (background, border, text, cursor));
    Ok(())
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
    mut frames: Query<(&mut BackgroundColor, &mut BorderColor)>,
    mut editors: Query<(&mut TextColor, &mut TextCursorStyle)>,
) -> Result {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item, &mut frames, &mut editors)?;
    }
    Ok(())
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
    mut frames: Query<(&mut BackgroundColor, &mut BorderColor)>,
    mut editors: Query<(&mut TextColor, &mut TextCursorStyle)>,
) -> Result {
    if !input_focus.is_changed() {
        return Ok(());
    }

    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item, &mut frames, &mut editors)?;
    }
    Ok(())
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
    mut frames: Query<(&mut BackgroundColor, &mut BorderColor)>,
    mut editors: Query<(&mut TextColor, &mut TextCursorStyle)>,
) -> Result {
    let focused = input_focus.get();

    for entity in removed_disabled.read() {
        if let Ok(item) = query.get_mut(entity) {
            apply_text_field_style(mode.colors(), focused, item, &mut frames, &mut editors)?;
        }
    }
    Ok(())
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
    let parts = world
        .get::<Parts>(entity)
        .ok_or_else(|| invalid_target(entity))?;
    let (frame, editor) = (parts.frame, parts.editor);
    let focused = world.get_resource::<InputFocus>().and_then(InputFocus::get) == Some(editor);
    let state = if world.get::<InteractionDisabled>(entity).is_some() {
        colors.disabled
    } else if focused {
        colors.focused
    } else if world.get::<Hovered>(entity).is_some_and(|h| h.0) {
        colors.hovered
    } else {
        colors.normal
    };
    let mut frame_query = world.query::<(&mut BackgroundColor, &mut BorderColor)>();
    let (mut background, mut border) = frame_query
        .get_mut(world, frame)
        .map_err(|_| invalid_target(entity))?;
    background.set_if_neq(BackgroundColor(state.background));
    border.set_if_neq(BorderColor::all(state.border));
    let mut editor_query = world.query::<(&mut TextColor, &mut TextCursorStyle)>();
    let (mut text, mut cursor) = editor_query
        .get_mut(world, editor)
        .map_err(|_| invalid_target(entity))?;
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
    Ok(())
}

fn invalid_target(entity: Entity) -> BevyError {
    bevy_widgetry_internal_log::widgetry_error!(?entity, "TextField 缺失托管颜色输出");
    BevyError::error("TextField 缺失托管颜色输出")
}
