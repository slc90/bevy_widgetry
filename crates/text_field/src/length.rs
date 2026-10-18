use crate::text_field::TextFieldEditor;
use bevy::{
    clipboard::Clipboard,
    prelude::*,
    text::{EditableText, EditableTextFilter, FontCx, LayoutCx, TextEdit},
};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_warn};
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
        .take(max.saturating_add(3))
        .map(|(index, _)| index)
        .collect();
    // Only the two replacement seams can merge input graphemes with surrounding text.
    // More than max + 2 input graphemes cannot fit, even when both seams merge.
    if ends.len() < max.saturating_add(3) {
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

pub(crate) fn initialize_limits(
    mut query: Query<(&TextFieldEditor, &mut EditableText), Added<TextFieldEditor>>,
    limits: Query<&LengthLimit>,
) -> Result {
    let mut failure = None;
    for (owner, mut edit) in &mut query {
        let Ok(limit) = limits.get(owner.root) else {
            edit.pending_edits.clear();
            edit.pending_paste = None;
            widgetry_error!(root = ?owner.root, "TextField 长度配置缺失");
            failure = Some(BevyError::error("TextField 长度配置缺失"));
            continue;
        };
        edit.max_characters = None;
        let text = edit.value().to_string();
        let limited = truncate(&text, limit.0);
        if limited != text {
            edit.editor_mut().set_text(limited);
        }
    }
    failure.map_or(Ok(()), Err)
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
    fonts: Option<ResMut<FontCx>>,
    layout: Option<ResMut<LayoutCx>>,
    clipboard: Option<ResMut<Clipboard>>,
) -> Result {
    let (Some(mut fonts), Some(mut layout), Some(mut clipboard)) = (fonts, layout, clipboard)
    else {
        return Ok(());
    };
    let mut failure = None;
    for (entity, owner, mut edit, filter) in &mut query {
        if edit.pending_edits.is_empty() && edit.pending_paste.is_none() {
            continue;
        }
        let Ok(limit) = limits.get(owner.root) else {
            edit.pending_edits.clear();
            edit.pending_paste = None;
            widgetry_error!(root = ?owner.root, "TextField 长度配置缺失");
            failure = Some(BevyError::error("TextField 长度配置缺失"));
            continue;
        };
        let Some(max) = limit.0 else { continue };
        // Bevy keeps the predicate private; never silently bypass a filter on constrained input.
        if filter.is_some() {
            edit.pending_edits.clear();
            edit.pending_paste = None;
            widgetry_error!(?entity, "max_length 不能与 EditableTextFilter 组合");
            failure = Some(BevyError::error(
                "max_length 不能与 EditableTextFilter 组合",
            ));
            continue;
        }
        let mut pending = std::collections::VecDeque::from(std::mem::take(&mut edit.pending_edits));
        if let Some(mut read) = edit.pending_paste.take() {
            match read.poll_result() {
                Some(Ok(text)) => pending.push_front(TextEdit::Insert(text.into())),
                Some(Err(error)) => widgetry_warn!(?entity, %error, "TextField 剪贴板读取失败"),
                None => {
                    edit.pending_paste = Some(read);
                    edit.pending_edits.extend(pending);
                    continue;
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
                            widgetry_warn!(?entity, %error, "TextField 剪贴板读取失败");
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
                TextEdit::Insert(value) => {
                    TextEdit::Insert(insertion_prefix(&edit, &value, max).into())
                }
                TextEdit::ImeCommit { value } => {
                    // Parley removes preedit and restores the insertion point before an IME commit.
                    edit.editor_mut()
                        .driver(&mut fonts, &mut layout.0)
                        .clear_compose();
                    TextEdit::ImeCommit {
                        value: insertion_prefix(&edit, &value, max).into(),
                    }
                }
                other => other,
            };
            edit.queue_edit(request);
            edit.apply_pending_edits(&mut fonts, &mut layout.0, &mut clipboard, |_| true);
        }
        // The official apply_text_edits still observes the generation and emits TextEditChange.
    }
    failure.map_or(Ok(()), Err)
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
