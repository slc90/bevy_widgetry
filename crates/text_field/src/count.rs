use crate::{length::LengthLimit, text_field::Parts};
use bevy::{prelude::*, text::EditableText};
use bevy_widgetry_internal_log::widgetry_error;
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

fn missing(root: Entity) -> BevyError {
    widgetry_error!(?root, "TextField 计数显示部件缺失");
    BevyError::error("TextField 计数显示部件缺失")
}

pub(crate) fn sync_count(
    mut displays: Query<(&ChildOf, &mut Text), With<CountDisplay>>,
    roots: Query<(&Parts, &LengthLimit)>,
    edits: Query<&EditableText, Changed<EditableText>>,
) -> Result {
    for (parent, mut text) in &mut displays {
        let root = parent.parent();
        let (parts, limit) = roots.get(root).map_err(|_| missing(root))?;
        if let Ok(edit) = edits.get(parts.editor) {
            text.set_if_neq(Text(label(edit, limit.0)));
        }
    }
    Ok(())
}

pub(crate) fn refresh_in_world(world: &mut World, root: Entity) -> Result {
    let display = world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|&entity| world.get::<CountDisplay>(entity).is_some())
    });
    let Some(display) = display else {
        return Ok(());
    };
    let parts = world.get::<Parts>(root).ok_or_else(|| missing(root))?;
    let limit = world
        .get::<LengthLimit>(root)
        .ok_or_else(|| missing(root))?;
    let edit = world
        .get::<EditableText>(parts.editor)
        .ok_or_else(|| missing(root))?;
    let value = label(edit, limit.0);
    world
        .get_mut::<Text>(display)
        .ok_or_else(|| missing(root))?
        .set_if_neq(Text(value));
    Ok(())
}
