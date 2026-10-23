use crate::text_field::{
    Parts, TextFieldEditor, TextFieldFrame, WidgetryTextFieldPrefix, WidgetryTextFieldSuffix,
};
use bevy::{
    prelude::*,
    text::{EditableTextFilter, FontHinting, LineHeight, TextLayout},
};
use bevy_widgetry_internal_log::widgetry_error;

pub fn frame(world: &World, root: Entity) -> Result<Entity, BevyError> {
    world
        .get::<Parts>(root)
        .map(|parts| parts.frame)
        .filter(|frame| world.get::<Node>(*frame).is_some())
        .ok_or_else(|| {
            widgetry_error!(?root, "TextField 内部框架实体缺失");
            BevyError::error("TextField 内部框架实体缺失")
        })
}

fn configure(world: &mut World, root: Entity) -> Result {
    let parts = world.get::<Parts>(root).ok_or_else(|| {
        widgetry_error!(?root, "TextField 部件映射缺失");
        BevyError::error("TextField 部件映射缺失")
    })?;
    let (frame, editor) = (parts.frame, parts.editor);
    let children = world
        .get::<Children>(frame)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    if !children.contains(&editor) || world.get::<TextFieldEditor>(editor).is_none() {
        widgetry_error!(?root, "TextField 内部编辑实体缺失");
        return Err(BevyError::error("TextField 内部编辑实体缺失"));
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
            widgetry_error!(
                ?root,
                ?child,
                "TextField 内容必须是唯一的 Prefix 或 Suffix 容器"
            );
            return Err(BevyError::error(
                "TextField 内容必须是唯一的 Prefix 或 Suffix 容器",
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
        commands.queue(move |world: &mut World| configure(world, root));
    }
}

fn projection_target(world: &mut World, root: Entity, editor: Entity) -> Result<bool> {
    if world.get_entity(root).is_err() {
        return Ok(false);
    }
    if world.get::<TextFieldEditor>(editor).is_none() {
        widgetry_error!(?root, ?editor, "TextField 配置投影目标缺失");
        return Err(BevyError::error("TextField 配置投影目标缺失"));
    }
    Ok(true)
}

fn project<T: Component>(commands: &mut Commands, root: Entity, editor: Entity, value: T) {
    commands.queue(move |world: &mut World| -> Result {
        if projection_target(world, root, editor)? {
            world.entity_mut(editor).insert(value);
        }
        Ok(())
    });
}

fn remove_projection<T: Component>(commands: &mut Commands, root: Entity, editor: Entity) {
    commands.queue(move |world: &mut World| -> Result {
        if projection_target(world, root, editor)? {
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
