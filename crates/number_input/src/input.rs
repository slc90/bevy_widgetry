use crate::{
    NumberInputFailure,
    error::{missing, query, text_field},
};
use crate::{
    WidgetryNumber, WidgetryNumberInputEditor, WidgetryNumberInputHeadless,
    editing::{ReadOnly, apply_pending, can_edit, editor, ensure_full, field, is_disabled},
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

#[derive(Component, FromTemplate)]
#[component(immutable)]
pub struct WidgetryNumberInputStep {
    pub root: Entity,
    pub increase: bool,
}

#[derive(Component)]
pub(crate) struct StepTarget(
    pub(crate) fn(&mut World, Entity, bool) -> Result<bool, NumberInputFailure>,
);

pub(crate) fn step<T: WidgetryNumber>(
    world: &mut World,
    root: Entity,
    increase: bool,
) -> Result<bool, NumberInputFailure> {
    state::<T>(world, root)?;
    let bound = editor(world, root)?;
    let field = field(world, root)?;
    if !can_edit(world, root)
        || is_disabled(world, field)
        || is_disabled(world, bound)
        || world.get::<EditableText>(bound).is_some_and(ime_owns_input)
    {
        return Ok(false);
    }
    ensure_full::<T>(world, root)?;
    apply_pending(world, root)?;
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
    WidgetryNumberInputHeadless::<T>::set_value_in_world(world, root, value)
}

fn activate(
    event: On<Activate>,
    buttons: Query<&WidgetryNumberInputStep, Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    let (root, increase) = (button.root, button.increase);
    commands.queue(move |world: &mut World| -> Result {
        let result = (|| -> Result<bool, NumberInputFailure> {
            let action = world
                .get::<StepTarget>(root)
                .map(|target| target.0)
                .ok_or_else(|| {
                    missing(
                        root,
                        root,
                        "WidgetryNumberInputHeadless",
                        "number_input.step_target",
                    )
                })?;
            action(world, root, increase)
        })();
        result.map(|_| ()).map_err(|error| error.into_bevy_error())
    });
}

fn focus_button(
    mut event: On<AcquireFocus>,
    buttons: Query<&WidgetryNumberInputStep, Without<InteractionDisabled>>,
    parts: Query<&WidgetryNumberInputEditor, With<StepTarget>>,
    targets: Query<&WidgetryInputTarget>,
    editors: Query<(), With<EditableText>>,
    mut focus: ResMut<InputFocus>,
) -> Result {
    let Ok(button) = buttons.get(event.focused_entity) else {
        return Ok(());
    };
    let part = parts.get(button.root).map_err(|source| {
        query(
            button.root,
            button.root,
            "WidgetryNumberInputEditor",
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
    editors.get(target.target).map_err(|source| {
        query(
            button.root,
            target.target,
            "EditableText",
            source,
            "number_input.focus_button",
        )
        .into_bevy_error()
    })?;
    // A guarded step must still retain editor focus, otherwise native tab focus can submit its draft on blur.
    focus.set(target.target, FocusCause::Pressed);
    event.focused_entity = target.target;
    Ok(())
}

fn on_key<T: WidgetryNumber>(
    mut event: On<FocusedInput<KeyboardInput>>,
    roots: Query<
        (Entity, &WidgetryNumberInputEditor),
        (With<WidgetryNumberInputHeadless<T>>, Without<ReadOnly>),
    >,
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
            if world.get::<WidgetryNumberInputHeadless<T>>(root).is_none()
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
            step::<T>(world, root, increase).map(|_| ())
        })();
        result.map_err(|error| error.into_bevy_error())
    });
}

fn ime_owns_input(edit: &EditableText) -> bool {
    edit.is_composing()
        || edit.pending_edits.iter().any(
            |request| matches!(request, TextEdit::ImeSetCompose { value, .. } if !value.is_empty()),
        )
}

pub(crate) fn install<T: WidgetryNumber>(app: &mut App) {
    app.add_observer(on_key::<T>);
}

pub(crate) fn install_focus(app: &mut App) {
    app.add_observer(focus_button).add_observer(activate);
}
