use crate::WidgetryTextFieldHeadless;
use crate::text_field::{ReadOnly, TextFieldEditor};
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

// TextInput handles caret and selection without claiming its editor as the pointer focus target.
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

// An unresolved editor focus request would reach the window and clear the focus claimed by its PointerPress.
// Stop propagation only when this field already owns focus.
pub(super) fn retain_text_field_focus_on_acquire(
    mut event: On<AcquireFocus>,
    text_fields: Query<(), (With<TextFieldEditor>, Without<InteractionDisabled>)>,
    roots: Query<&WidgetryTextFieldHeadless, Without<InteractionDisabled>>,
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

pub(crate) fn redirect_root_focus(
    roots: Query<&WidgetryTextFieldHeadless>,
    mut focus: ResMut<InputFocus>,
) {
    if let Some(current) = focus.get()
        && let Ok(parts) = roots.get(current)
    {
        focus.set(parts.editor, FocusCause::Navigated);
    }
}

pub(crate) fn redirect_gained_focus(
    event: On<bevy::input_focus::FocusGained>,
    roots: Query<&WidgetryTextFieldHeadless>,
    mut focus: ResMut<InputFocus>,
) {
    if focus.get() == Some(event.entity)
        && let Ok(parts) = roots.get(event.entity)
    {
        focus.set(parts.editor, event.cause);
    }
}
