use crate::text_field::{Parts, ReadOnly, TextFieldEditor};
use bevy::input_focus::{AcquireFocus, FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::InteractionDisabled;

pub(super) fn block_disabled_text_field_edits(
    mut query: Query<&mut EditableText, (With<TextFieldEditor>, With<InteractionDisabled>)>,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.clear();
        editable_text.pending_paste = None;
    }
}

pub(super) fn block_read_only_text_field_edits(
    mut query: Query<(&TextFieldEditor, &mut EditableText)>,
    read_only: Query<(), With<ReadOnly>>,
) {
    for (owner, mut editable_text) in &mut query {
        if !read_only.contains(owner.root) {
            continue;
        }
        editable_text.pending_edits.retain(|edit| {
            !matches!(
                edit,
                TextEdit::Cut
                    | TextEdit::Paste
                    | TextEdit::Insert(_)
                    | TextEdit::Backspace
                    | TextEdit::BackspaceWord
                    | TextEdit::Delete
                    | TextEdit::DeleteWord
                    | TextEdit::ImeSetCompose { .. }
                    | TextEdit::ImeCommit { .. }
            )
        });
        editable_text.pending_paste = None;
    }
}

// TextInput only handles caret/selection, so fields without TabIndex need pointer focus here.
pub(super) fn focus_text_field_on_press(
    event: On<bevy::picking::events::PointerPress>,
    text_fields: Query<(), (With<TextFieldEditor>, Without<InteractionDisabled>)>,
    mut focus: ResMut<InputFocus>,
) {
    if event.button == bevy::picking::pointer::PointerButton::Primary
        && text_fields.contains(event.entity)
        && focus.get() != Some(event.entity)
    {
        focus.set(event.entity, FocusCause::Pressed);
    }
}

// Without TabIndex, AcquireFocus would reach the window and clear pointer focus.
// Stop propagation only when this field already owns focus.
pub(super) fn retain_text_field_focus_on_acquire(
    mut event: On<AcquireFocus>,
    text_fields: Query<(), (With<TextFieldEditor>, Without<InteractionDisabled>)>,
    roots: Query<&Parts, Without<InteractionDisabled>>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(parts) = roots.get(event.focused_entity) {
        event.propagate(false);
        if focus.get() != Some(parts.editor) {
            focus.set(parts.editor, FocusCause::Navigated);
        }
        event.focused_entity = parts.editor;
        return;
    }
    if text_fields.contains(event.focused_entity) && focus.get() == Some(event.focused_entity) {
        event.propagate(false);
    }
}

pub(crate) fn contain_slot_focus(
    mut event: On<AcquireFocus>,
    slots: Query<
        (),
        Or<(
            With<crate::WidgetryTextFieldPrefix>,
            With<crate::WidgetryTextFieldSuffix>,
        )>,
    >,
) {
    if slots.contains(event.focused_entity) {
        event.propagate(false);
    }
}

pub(crate) fn redirect_root_focus(roots: Query<&Parts>, mut focus: ResMut<InputFocus>) {
    if let Some(current) = focus.get()
        && let Ok(parts) = roots.get(current)
    {
        focus.set(parts.editor, FocusCause::Navigated);
    }
}

pub(crate) fn redirect_gained_focus(
    event: On<bevy::input_focus::FocusGained>,
    roots: Query<&Parts>,
    mut focus: ResMut<InputFocus>,
) {
    if focus.get() == Some(event.entity)
        && let Ok(parts) = roots.get(event.entity)
    {
        focus.set(parts.editor, event.cause);
    }
}

pub(crate) fn navigate_from_editor(
    mut event: On<bevy::input_focus::FocusedInput<bevy::input::keyboard::KeyboardInput>>,
    editors: Query<(&TextFieldEditor, &EditableText)>,
    roots: Query<&Parts>,
    nav: bevy::input_focus::tab_navigation::TabNavigation,
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<bevy::input_focus::InputFocusVisible>,
) -> Result {
    if event.input.key_code != KeyCode::Tab || !event.input.state.is_pressed() || event.input.repeat
    {
        return Ok(());
    }
    let Ok((owner, edit)) = editors.get(event.original_event_target()) else {
        return Ok(());
    };
    // Handle at the composite root, after TextInput has decided whether to propagate the key.
    // A sibling observer on the editor can overwrite propagate(false) when it returns.
    if event.focused_entity != owner.root {
        return Ok(());
    }
    if edit.is_composing() {
        return Ok(());
    }
    let action = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        bevy::input_focus::tab_navigation::NavAction::Previous
    } else {
        bevy::input_focus::tab_navigation::NavAction::Next
    };
    let next = match nav.navigate(&InputFocus::from_entity(owner.root), action) {
        Ok(next) => next,
        Err(bevy::input_focus::tab_navigation::TabNavigationError::NoTabGroupForCurrentFocus {
            new_focus,
            ..
        }) => new_focus,
        Err(error) => {
            bevy_widgetry_internal_log::widgetry_warn!(%error, "TextField Tab 导航不可用");
            event.propagate(false);
            return Ok(());
        }
    };
    let target = roots.get(next).map(|parts| parts.editor).unwrap_or(next);
    focus.set(target, FocusCause::Navigated);
    visible.0 = true;
    event.propagate(false);
    Ok(())
}
