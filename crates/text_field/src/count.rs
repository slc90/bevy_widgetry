use crate::{
    TextFieldFailure,
    error::{missing, query},
};
use crate::{length::LengthLimit, text_field::Parts};
use bevy::{prelude::*, text::EditableText};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Component, Clone, Default)]
pub(crate) struct CountDisplay;

pub(crate) fn label(edit: &EditableText, max: Option<usize>) -> String {
    let count = edit.value().to_string().graphemes(true).count();
    match max {
        Some(max) => format!("{count} / {max}"),
        None => count.to_string(),
    }
}

pub(crate) fn sync_count(
    mut displays: Query<(&ChildOf, &mut Text), With<CountDisplay>>,
    roots: Query<(&Parts, &LengthLimit)>,
    edits: Query<&EditableText, Changed<EditableText>>,
) -> Result {
    for (parent, mut text) in &mut displays {
        let root = parent.parent();
        let (parts, limit) = roots.get(root).map_err(|source| {
            query(
                root,
                root,
                "Parts / LengthLimit",
                source,
                "text_field.sync_count",
            )
            .into_bevy_error()
        })?;
        if let Ok(edit) = edits.get(parts.editor) {
            text.set_if_neq(Text(label(edit, limit.0)));
        }
    }
    Ok(())
}

pub(crate) fn refresh_in_world(world: &mut World, root: Entity) -> Result<(), TextFieldFailure> {
    let display = world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|&entity| world.get::<CountDisplay>(entity).is_some())
    });
    let Some(display) = display else {
        return Ok(());
    };
    let parts = world
        .get::<Parts>(root)
        .ok_or_else(|| missing(root, root, "Parts", "text_field.refresh_count"))?;
    let limit = world
        .get::<LengthLimit>(root)
        .ok_or_else(|| missing(root, root, "LengthLimit", "text_field.refresh_count"))?;
    let editor = parts.editor;
    let edit = world
        .get::<EditableText>(editor)
        .ok_or_else(|| missing(root, editor, "EditableText", "text_field.refresh_count"))?;
    let value = label(edit, limit.0);
    world
        .get_mut::<Text>(display)
        .ok_or_else(|| missing(root, display, "Text", "text_field.refresh_count"))?
        .set_if_neq(Text(value));
    Ok(())
}
