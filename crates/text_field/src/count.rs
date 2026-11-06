use crate::{
    TextFieldFailure,
    error::{missing, query},
};
use crate::{WidgetryTextFieldHeadless, length::LengthLimit};
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
    roots: Query<&WidgetryTextFieldHeadless>,
    edits: Query<(Ref<EditableText>, &LengthLimit)>,
) -> Result {
    for (parent, mut text) in &mut displays {
        let root = parent.parent();
        let binding = roots.get(root).map_err(|source| {
            query(
                root,
                root,
                "WidgetryTextFieldHeadless",
                source,
                "text_field.sync_count",
            )
            .into_bevy_error()
        })?;
        let (edit, limit) = edits.get(binding.editor).map_err(|source| {
            query(
                root,
                binding.editor,
                "EditableText / LengthLimit",
                source,
                "text_field.sync_count",
            )
            .into_bevy_error()
        })?;
        if edit.is_changed() {
            text.set_if_neq(Text(label(&edit, limit.0)));
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
    let editor = WidgetryTextFieldHeadless::editor_entity(world, root)?;
    let limit = world
        .get::<LengthLimit>(editor)
        .ok_or_else(|| missing(root, editor, "LengthLimit", "text_field.refresh_count"))?;
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
