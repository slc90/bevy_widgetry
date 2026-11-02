use crate::text_field::{
    Parts, TextFieldEditor, TextFieldFrame, WidgetryTextFieldPrefix, WidgetryTextFieldSuffix,
};
use crate::{
    TextFieldError, TextFieldFailure,
    error::{failure, missing},
};
use bevy::{
    prelude::*,
    text::{EditableTextFilter, FontHinting, LineHeight, TextLayout},
};
use bevy_widgetry_diagnostics::ErrorFrame;

pub fn frame(world: &World, root: Entity) -> Result<Entity, TextFieldFailure> {
    let parts = world
        .get::<Parts>(root)
        .ok_or_else(|| missing(root, root, "Parts", "text_field.frame"))?;
    if world.get::<Node>(parts.frame).is_none() {
        return Err(missing(root, parts.frame, "Node", "text_field.frame")
            .with_context(ErrorFrame::new("text_field.frame").field("editor", parts.editor)));
    }
    Ok(parts.frame)
}

fn configure(world: &mut World, root: Entity) -> Result<(), TextFieldFailure> {
    let parts = world
        .get::<Parts>(root)
        .ok_or_else(|| missing(root, root, "Parts", "text_field.configure_slots"))?;
    let (frame, editor) = (parts.frame, parts.editor);
    let children = world
        .get::<Children>(frame)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    if !children.contains(&editor) || world.get::<TextFieldEditor>(editor).is_none() {
        return Err(failure(
            TextFieldError::InvalidEditorMapping {
                root,
                frame,
                editor,
            },
            ErrorFrame::new("text_field.configure_slots"),
        ));
    }
    let mut prefix = None;
    let mut suffix = None;
    for child in children.iter().copied().filter(|&e| e != editor) {
        let is_prefix = world.get::<WidgetryTextFieldPrefix>(child).is_some();
        let is_suffix = world.get::<WidgetryTextFieldSuffix>(child).is_some();
        if is_prefix == is_suffix
            || (is_prefix && prefix.replace(child).is_some())
            || (is_suffix && suffix.replace(child).is_some())
        {
            return Err(failure(
                TextFieldError::InvalidSlots {
                    root,
                    frame,
                    child,
                    prefix: is_prefix,
                    suffix: is_suffix,
                },
                ErrorFrame::new("text_field.configure_slots"),
            ));
        }
    }
    let order: Vec<_> = prefix
        .into_iter()
        .chain(std::iter::once(editor))
        .chain(suffix)
        .collect();
    for slot in prefix.into_iter().chain(suffix) {
        if let Some(mut node) = world.get_mut::<Node>(slot) {
            node.flex_shrink = 0.0;
        }
    }
    if children != order {
        world.entity_mut(frame).replace_children(&order);
    }
    Ok(())
}

pub(crate) fn configure_slots(
    frames: Query<&TextFieldFrame, Changed<Children>>,
    added: Query<
        &ChildOf,
        Or<(
            Added<WidgetryTextFieldPrefix>,
            Added<WidgetryTextFieldSuffix>,
        )>,
    >,
    parents: Query<&ChildOf>,
    owners: Query<&TextFieldFrame>,
    mut removed_prefix: RemovedComponents<WidgetryTextFieldPrefix>,
    mut removed_suffix: RemovedComponents<WidgetryTextFieldSuffix>,
    mut commands: Commands,
) {
    let mut roots: std::collections::HashSet<Entity> =
        frames.iter().map(|owner| owner.root).collect();
    for parent in &added {
        if let Ok(owner) = owners.get(parent.parent()) {
            roots.insert(owner.root);
        }
    }
    for entity in removed_prefix.read().chain(removed_suffix.read()) {
        if let Ok(parent) = parents.get(entity)
            && let Ok(owner) = owners.get(parent.parent())
        {
            roots.insert(owner.root);
        }
    }
    for root in roots {
        commands.queue(move |world: &mut World| {
            configure(world, root).map_err(|error| error.into_bevy_error())
        });
    }
}

fn projection_target(
    world: &mut World,
    root: Entity,
    editor: Entity,
) -> Result<bool, TextFieldFailure> {
    if world.get_entity(root).is_err() {
        return Ok(false);
    }
    if world.get::<TextFieldEditor>(editor).is_none() {
        return Err(missing(
            root,
            editor,
            "TextFieldEditor",
            "text_field.project_configuration",
        ));
    }
    Ok(true)
}

fn project<T: Component>(commands: &mut Commands, root: Entity, editor: Entity, value: T) {
    commands.queue(move |world: &mut World| -> Result {
        if projection_target(world, root, editor).map_err(|error| error.into_bevy_error())? {
            world.entity_mut(editor).insert(value);
        }
        Ok(())
    });
}

fn remove_projection<T: Component>(commands: &mut Commands, root: Entity, editor: Entity) {
    commands.queue(move |world: &mut World| -> Result {
        if projection_target(world, root, editor).map_err(|error| error.into_bevy_error())? {
            world.entity_mut(editor).remove::<T>();
        }
        Ok(())
    });
}

pub(crate) fn project_text_configuration(
    fonts: Query<(Entity, &Parts, &TextFont), Changed<TextFont>>,
    layouts: Query<(Entity, &Parts, &TextLayout), Changed<TextLayout>>,
    lines: Query<(Entity, &Parts, &LineHeight), Changed<LineHeight>>,
    hints: Query<(Entity, &Parts, &FontHinting), Changed<FontHinting>>,
    filters: Query<(Entity, &Parts, &EditableTextFilter), Changed<EditableTextFilter>>,
    select_all: Query<(Entity, &Parts), Added<bevy::ui_widgets::SelectAllOnFocus>>,
    roots: Query<(
        &Parts,
        Has<EditableTextFilter>,
        Has<bevy::ui_widgets::SelectAllOnFocus>,
    )>,
    mut removed_filters: RemovedComponents<EditableTextFilter>,
    mut removed_select_all: RemovedComponents<bevy::ui_widgets::SelectAllOnFocus>,
    mut commands: Commands,
) {
    for (root, parts, value) in &fonts {
        project(&mut commands, root, parts.editor, value.clone());
    }
    for (root, parts, value) in &layouts {
        project(&mut commands, root, parts.editor, *value);
    }
    for (root, parts, value) in &lines {
        project(&mut commands, root, parts.editor, *value);
    }
    for (root, parts, value) in &hints {
        project(&mut commands, root, parts.editor, *value);
    }
    for (root, parts, value) in &filters {
        project(&mut commands, root, parts.editor, value.clone());
    }
    for (root, parts) in &select_all {
        project(
            &mut commands,
            root,
            parts.editor,
            bevy::ui_widgets::SelectAllOnFocus,
        );
    }
    for root in removed_filters.read() {
        if let Ok((parts, false, _)) = roots.get(root) {
            remove_projection::<EditableTextFilter>(&mut commands, root, parts.editor);
        }
    }
    for root in removed_select_all.read() {
        if let Ok((parts, _, false)) = roots.get(root) {
            remove_projection::<bevy::ui_widgets::SelectAllOnFocus>(
                &mut commands,
                root,
                parts.editor,
            );
        }
    }
}
