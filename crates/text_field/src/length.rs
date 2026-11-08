const REPLACEMENT_SEAM_LOOKAHEAD: usize = 3;

use crate::text_field::TextFieldEditor;
use crate::{
    TextFieldError,
    error::{failure, missing},
};
use bevy::{
    clipboard::Clipboard,
    prelude::*,
    text::{EditableText, EditableTextFilter, FontCx, LayoutCx, TextEdit},
};
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::DiagnosticBatch};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Component)]
pub(crate) struct LengthLimit(pub(crate) Option<usize>);

pub(crate) fn truncate(text: &str, max: Option<usize>) -> &str {
    let end = max
        .and_then(|max| text.grapheme_indices(true).nth(max).map(|(index, _)| index))
        .unwrap_or(text.len());
    &text[..end]
}

fn insertion_prefix<'a>(edit: &EditableText, input: &'a str, max: usize) -> &'a str {
    let text = edit.value().to_string();
    let range = edit.editor().raw_selection().text_range();
    replacement_prefix(&text, range, input, max)
}

fn replacement_prefix<'a>(
    text: &str,
    range: std::ops::Range<usize>,
    input: &'a str,
    max: usize,
) -> &'a str {
    let mut ends: Vec<_> = input
        .grapheme_indices(true)
        .take(max.saturating_add(REPLACEMENT_SEAM_LOOKAHEAD))
        .map(|(index, _)| index)
        .collect();
    // Only the two replacement seams can merge input graphemes with surrounding text.
    // More than max + 2 input graphemes cannot fit, even when both seams merge.
    if ends.len() < max.saturating_add(REPLACEMENT_SEAM_LOOKAHEAD) {
        ends.push(input.len());
    }
    let mut candidate = String::new();
    // A prefix can merge with either side of the replacement (combining marks, RI and ZWJ).
    // Check the resulting full string rather than adding separately counted lengths.
    for end in ends.into_iter().rev() {
        candidate.clear();
        candidate.push_str(&text[..range.start]);
        candidate.push_str(&input[..end]);
        candidate.push_str(&text[range.end..]);
        if candidate
            .graphemes(true)
            .take(max.saturating_add(1))
            .count()
            <= max
        {
            return &input[..end];
        }
    }
    ""
}

fn input_binding(
    roots: &Query<&crate::WidgetryTextFieldHeadless>,
    root: Entity,
    editor: Entity,
) -> Result<(), crate::TextFieldFailure> {
    let binding = roots.get(root).map_err(|_| {
        missing(
            root,
            root,
            "WidgetryTextFieldHeadless",
            "text_field.input_binding",
        )
    })?;
    if binding.editor != editor {
        return Err(failure(
            TextFieldError::InvalidEditorBinding {
                root,
                editor,
                bound: binding.editor,
            },
            ErrorFrame::new("text_field.input_binding"),
        ));
    }
    Ok(())
}

pub(crate) fn initialize_limits(
    mut query: Query<
        (
            Entity,
            &TextFieldEditor,
            &mut EditableText,
            Option<&LengthLimit>,
        ),
        Added<TextFieldEditor>,
    >,
    roots: Query<&crate::WidgetryTextFieldHeadless>,
) -> Result {
    let mut failures = DiagnosticBatch::default();
    for (entity, owner, mut edit, limit) in &mut query {
        let validated = input_binding(&roots, owner.root, entity).and_then(|()| {
            limit.ok_or_else(|| {
                missing(
                    owner.root,
                    entity,
                    "LengthLimit",
                    "text_field.initialize_limit",
                )
            })
        });
        let limit = match validated {
            Ok(limit) => limit,
            Err(error) => {
                edit.pending_edits.clear();
                edit.pending_paste = None;
                failures.push(error);
                continue;
            }
        };
        edit.max_characters = None;
        let text = edit.value().to_string();
        let limited = truncate(&text, limit.0);
        if limited != text {
            edit.editor_mut().set_text(limited);
        }
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}

pub(crate) fn apply_limited_edits(
    mut query: Query<
        (
            Entity,
            &TextFieldEditor,
            &mut EditableText,
            Option<&EditableTextFilter>,
        ),
        With<TextFieldEditor>,
    >,
    limits: Query<&LengthLimit>,
    roots: Query<&crate::WidgetryTextFieldHeadless>,
    fonts: Option<ResMut<FontCx>>,
    layout: Option<ResMut<LayoutCx>>,
    clipboard: Option<ResMut<Clipboard>>,
) -> Result {
    let (Some(mut fonts), Some(mut layout), Some(mut clipboard)) = (fonts, layout, clipboard)
    else {
        return Ok(());
    };
    let mut failures = DiagnosticBatch::default();
    for (entity, owner, mut edit, filter) in &mut query {
        if edit.pending_edits.is_empty() && edit.pending_paste.is_none() {
            continue;
        }
        if let Err(error) = input_binding(&roots, owner.root, entity) {
            edit.pending_edits.clear();
            edit.pending_paste = None;
            failures.push(error);
            continue;
        }
        let Ok(limit) = limits.get(entity) else {
            edit.pending_edits.clear();
            edit.pending_paste = None;
            failures.push(missing(
                owner.root,
                entity,
                "LengthLimit",
                "text_field.apply_length_limit",
            ));
            continue;
        };
        let Some(max) = limit.0 else { continue };
        // Bevy keeps the predicate private; never silently bypass a filter on constrained input.
        if filter.is_some() {
            edit.pending_edits.clear();
            edit.pending_paste = None;
            failures.push(failure(
                TextFieldError::FilterConflict {
                    root: owner.root,
                    editor: entity,
                    max,
                },
                ErrorFrame::new("text_field.apply_limited_edits"),
            ));
            continue;
        }
        apply_requests(
            owner.root,
            entity,
            &mut edit,
            Some(max),
            &mut fonts,
            &mut layout,
            &mut clipboard,
        );
        // The official apply_text_edits still observes the generation and emits TextEditChange.
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}

fn apply_requests(
    root: Entity,
    editor: Entity,
    edit: &mut EditableText,
    max: Option<usize>,
    fonts: &mut FontCx,
    layout: &mut LayoutCx,
    clipboard: &mut Clipboard,
) {
    let mut pending = std::collections::VecDeque::from(std::mem::take(&mut edit.pending_edits));
    if let Some(mut read) = edit.pending_paste.take() {
        match read.poll_result() {
            Some(Ok(text)) => pending.push_front(TextEdit::Insert(text.into())),
            Some(Err(error)) => abandon_paste(root, editor, error),
            None => {
                edit.pending_paste = Some(read);
                edit.pending_edits.extend(pending);
                return;
            }
        }
    }
    while let Some(request) = pending.pop_front() {
        // Preedit can start earlier in this same queue, after the keyboard observer's guard.
        // Keep keyboard insertion/paste owned by the IME until its explicit commit.
        if edit.is_composing() && matches!(request, TextEdit::Insert(_) | TextEdit::Paste) {
            continue;
        }
        let request = match request {
            TextEdit::Paste => {
                let mut read = clipboard.fetch_text();
                match read.poll_result() {
                    Some(Ok(text)) => TextEdit::Insert(text.into()),
                    Some(Err(error)) => {
                        abandon_paste(root, editor, error);
                        continue;
                    }
                    None => {
                        edit.pending_paste = Some(read);
                        edit.pending_edits.extend(pending);
                        break;
                    }
                }
            }
            other => other,
        };
        let request = match request {
            TextEdit::Insert(value) => TextEdit::Insert(
                max.map_or(value.as_str(), |max| insertion_prefix(edit, &value, max))
                    .into(),
            ),
            TextEdit::ImeCommit { value } => {
                // Parley removes preedit and restores the insertion point before an IME commit.
                edit.editor_mut()
                    .driver(fonts, &mut layout.0)
                    .clear_compose();
                TextEdit::ImeCommit {
                    value: max
                        .map_or(value.as_str(), |max| insertion_prefix(edit, &value, max))
                        .into(),
                }
            }
            other => other,
        };
        edit.queue_edit(request);
        edit.apply_pending_edits(fonts, &mut layout.0, clipboard, |_| true);
    }
}

impl crate::WidgetryTextFieldHeadless {
    /// Consumes queued edits early for composites that build their UI from the edited text.
    /// Bevy's normal editing stage still emits its change notification and handles layout.
    /// Opaque native character filters cannot be evaluated early and are rejected.
    pub fn apply_pending_edits_in_world(
        world: &mut World,
        root: Entity,
    ) -> Result<(), crate::TextFieldFailure> {
        let editor = Self::editor_entity(world, root)?;
        let max = Self::max_length(world, root)?;
        if world.get::<bevy::ui::InteractionDisabled>(root).is_some()
            || world.get::<bevy::ui::InteractionDisabled>(editor).is_some()
        {
            if let Some(mut edit) = world.get_mut::<EditableText>(editor) {
                edit.pending_edits.clear();
                edit.pending_paste = None;
            }
            return Ok(());
        }
        if world.get::<EditableTextFilter>(editor).is_some() {
            return Err(failure(
                TextFieldError::EarlyEditFilter { root, editor },
                ErrorFrame::new("text_field.apply_pending_edits"),
            ));
        }
        for (present, resource) in [
            (world.contains_resource::<FontCx>(), "FontCx"),
            (world.contains_resource::<LayoutCx>(), "LayoutCx"),
            (world.contains_resource::<Clipboard>(), "Clipboard"),
        ] {
            if !present {
                return Err(failure(
                    TextFieldError::MissingResource { root, resource },
                    ErrorFrame::new("text_field.apply_pending_edits").field("editor", editor),
                ));
            }
        }
        let read_only = Self::is_read_only(world, root)?;
        world.resource_scope(|world, mut fonts: Mut<FontCx>| {
            world.resource_scope(|world, mut layout: Mut<LayoutCx>| {
                world.resource_scope(|world, mut clipboard: Mut<Clipboard>| {
                    let mut edit = world.get_mut::<EditableText>(editor).ok_or_else(|| {
                        missing(
                            root,
                            editor,
                            "EditableText",
                            "text_field.apply_pending_edits",
                        )
                    })?;
                    if read_only {
                        edit.pending_edits.retain(|edit| {
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
                        edit.pending_paste = None;
                    }
                    apply_requests(
                        root,
                        editor,
                        &mut edit,
                        max,
                        &mut fonts,
                        &mut layout,
                        &mut clipboard,
                    );
                    Ok(())
                })
            })
        })
    }
}

fn abandon_paste(root: Entity, editor: Entity, source: bevy::clipboard::ClipboardError) {
    let error = failure(
        TextFieldError::Clipboard {
            root,
            editor,
            source,
        },
        ErrorFrame::new("text_field.read_clipboard"),
    );
    error.record_handled(
        "abandon_paste",
        "已丢弃本次失败的粘贴请求，继续处理其余编辑",
    );
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn fragments() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            prop::sample::select(vec![
                "a", "中", "\u{301}", "🇦", "🇧", "👨", "👩", "\u{200d}", "🏽", "\r", "\n", "क",
                "\u{94d}", "\u{600}",
            ]),
            0..24,
        )
        .prop_map(|pieces| pieces.concat())
    }

    proptest! {
        #[test]
        fn bounded_search_matches_exhaustive_trial_replacement(left in fragments(), right in fragments(), input in fragments(), max in 0usize..12) {
            let text = format!("{left}discard{right}");
            let range = left.len()..left.len()+7;
            let expected = std::iter::once(input.len())
                .chain(input.grapheme_indices(true).map(|(i,_)| i).rev())
                .find(|&end| format!("{left}{}{right}", &input[..end]).graphemes(true).count() <= max)
                .unwrap_or(0);
            prop_assert_eq!(replacement_prefix(&text, range, &input, max), &input[..expected]);
        }
    }
}
