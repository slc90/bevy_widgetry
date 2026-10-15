use crate::text_field::{ReadOnly, TextFieldBase};
use bevy::input_focus::{AcquireFocus, FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::InteractionDisabled;

pub(super) fn block_disabled_text_field_edits(
    mut query: Query<&mut EditableText, (With<TextFieldBase>, With<InteractionDisabled>)>,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.clear();
        editable_text.pending_paste = None;
    }
}

pub(super) fn block_read_only_text_field_edits(
    mut query: Query<&mut EditableText, (With<TextFieldBase>, With<ReadOnly>)>,
) {
    for mut editable_text in &mut query {
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
    text_fields: Query<(), (With<TextFieldBase>, Without<InteractionDisabled>)>,
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
    text_fields: Query<(), (With<TextFieldBase>, Without<InteractionDisabled>)>,
    focus: Res<InputFocus>,
) {
    if text_fields.contains(event.focused_entity) && focus.get() == Some(event.focused_entity) {
        event.propagate(false);
    }
}
