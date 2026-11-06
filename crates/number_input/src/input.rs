use crate::{
    NumberInputFailure,
    error::{missing, query, text_field},
};
use crate::{
    WidgetryNumber, WidgetryNumberInput,
    editing::{Parts, ReadOnly, StepButton, apply_pending, can_edit, editor, ensure_full, field},
    value::state,
};
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{AcquireFocus, FocusCause, FocusedInput, InputFocus},
    prelude::*,
    text::{EditableText, TextEdit},
    ui::InteractionDisabled,
    ui_widgets::Activate,
};
use bevy_widgetry_core::input_target::WidgetryInputTarget;
use bevy_widgetry_text_field::WidgetryTextFieldHeadless;

pub(crate) fn step<T: WidgetryNumber>(
    world: &mut World,
    root: Entity,
    increase: bool,
) -> Result<(), NumberInputFailure> {
    if !can_edit(world, root) {
        return Ok(());
    }
    ensure_full::<T>(world, root)?;
    apply_pending(world, root)?;
    let field = field(world, root)?;
    let text = WidgetryTextFieldHeadless::text(world, field)
        .map_err(|error| text_field(error, root, field, "number_input.step.text"))?;
    let state = state::<T>(world, root)?;
    let start = T::parse_decimal(&text)
        .filter(|v| state.accepts(*v))
        .unwrap_or(state.value);
    let mut value = start.saturating_step(state.step, increase);
    if let Some(min) = state.min
        && value < min
    {
        value = min;
    }
    if let Some(max) = state.max
        && value > max
    {
        value = max;
    }
    WidgetryTextFieldHeadless::focus_in_world(world, field)
        .map_err(|error| text_field(error, root, field, "number_input.step.focus_in_world"))?;
    WidgetryNumberInput::<T>::set_value_in_world(world, root, value)?;
    Ok(())
}

fn activate<T: WidgetryNumber>(
    event: On<Activate>,
    buttons: Query<&StepButton, Without<InteractionDisabled>>,
    roots: Query<(), With<WidgetryNumberInput<T>>>,
    mut commands: Commands,
) {
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    if !roots.contains(button.root) {
        return;
    }
    let (root, increase) = (button.root, button.increase);
    commands.queue(move |world: &mut World| -> Result {
        let result = (|| -> Result<(), NumberInputFailure> {
            if world.get::<WidgetryNumberInput<T>>(root).is_none() {
                return Ok(());
            }
            step::<T>(world, root, increase)
        })();
        result.map_err(|error| error.into_bevy_error())
    });
}

fn focus_button(
    mut event: On<AcquireFocus>,
    buttons: Query<&StepButton, Without<InteractionDisabled>>,
    parts: Query<&Parts>,
    targets: Query<&WidgetryInputTarget>,
    mut focus: ResMut<InputFocus>,
) -> Result {
    let Ok(button) = buttons.get(event.focused_entity) else {
        return Ok(());
    };
    let part = parts.get(button.root).map_err(|source| {
        query(
            button.root,
            button.root,
            "Parts",
            source,
            "number_input.focus_button",
        )
        .into_bevy_error()
    })?;
    let target = targets.get(part.field).map_err(|source| {
        query(
            button.root,
            part.field,
            "WidgetryInputTarget",
            source,
            "number_input.focus_button",
        )
        .into_bevy_error()
    })?;
    event.propagate(false);
    focus.set(target.target, FocusCause::Pressed);
    Ok(())
}

fn on_key<T: WidgetryNumber>(
    mut event: On<FocusedInput<KeyboardInput>>,
    roots: Query<(Entity, &Parts), (With<WidgetryNumberInput<T>>, Without<ReadOnly>)>,
    targets: Query<&WidgetryInputTarget>,
    editors: Query<&EditableText, Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let target = event.original_event_target();
    if event.focused_entity != target
        || !event.input.state.is_pressed()
        || !editors.get(target).is_ok_and(|edit| !ime_owns_input(edit))
    {
        return;
    }
    let increase = match event.input.key_code {
        KeyCode::ArrowUp => true,
        KeyCode::ArrowDown => false,
        _ => return,
    };
    let Some(root) = roots.iter().find_map(|(root, parts)| {
        targets
            .get(parts.field)
            .is_ok_and(|t| t.target == target)
            .then_some(root)
    }) else {
        return;
    };
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result {
        let result = (|| -> Result<(), NumberInputFailure> {
            if world.get::<WidgetryNumberInput<T>>(root).is_none()
                || world.resource::<InputFocus>().get() != Some(target)
            {
                return Ok(());
            }
            let editor = editor(world, root)?;
            if world
                .get::<EditableText>(editor)
                .is_some_and(ime_owns_input)
            {
                return Ok(());
            }
            // The native editor also queues vertical caret movement for this key; NumberInput owns that action.
            world
                .get_mut::<EditableText>(editor)
                .ok_or_else(|| missing(root, editor, "EditableText", "number_input.key"))?
                .pending_edits
                .retain(|edit| !matches!(edit, TextEdit::Up(_) | TextEdit::Down(_)));
            step::<T>(world, root, increase)
        })();
        result.map_err(|error| error.into_bevy_error())
    });
}

fn ime_owns_input(edit: &EditableText) -> bool {
    edit.is_composing()
        || edit
            .pending_edits
            .iter()
            .any(|request| matches!(request, TextEdit::ImeSetCompose { .. }))
}

pub(crate) fn install<T: WidgetryNumber>(app: &mut App) {
    app.add_observer(activate::<T>).add_observer(on_key::<T>);
}

pub(crate) fn install_focus(app: &mut App) {
    app.add_observer(focus_button);
}
