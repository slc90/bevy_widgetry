use super::*;
use bevy::ecs::entity::{EntityHashMap, EntityHashSet};

#[derive(Resource, Default)]
struct ConfigurationChanges(EntityHashSet);

fn inserted(event: On<Insert<(ChildOf, TextFont)>>, mut changes: ResMut<ConfigurationChanges>) {
    changes.0.insert(event.entity);
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<ConfigurationChanges>()
        .add_observer(inserted)
        .add_systems(
            PostUpdate,
            configure
                .in_set(WidgetryUiSystems::StyleOwners)
                .before(establish_color_owners),
        );
}

fn dialog(world: &World, entity: Entity) -> Option<Entity> {
    let mut current = Some(entity);
    while let Some(entity) = current {
        if world.get::<WidgetryFileDialog>(entity).is_some() {
            return Some(entity);
        }
        current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    None
}

fn inside(world: &World, entity: Entity, host: Entity) -> bool {
    let mut current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    while let Some(entity) = current {
        if entity == host {
            return true;
        }
        current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    false
}

pub(super) fn configure(world: &mut World) -> Result {
    let pending = std::mem::take(&mut world.resource_mut::<ConfigurationChanges>().0);
    let mut targets: EntityHashMap<EntityHashSet> = default();
    for entity in pending {
        if let Some(root) = dialog(world, entity) {
            targets.entry(root).or_default().insert(entity);
        }
    }
    let roots = world
        .query_filtered::<Entity, (With<WidgetryFileDialog>, With<Shell>)>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut first_error = None;
    for root in roots {
        if world.get::<RejectedStyle>(root).is_some() {
            if let Some(changed) = targets.remove(&root) {
                world
                    .resource_mut::<ConfigurationChanges>()
                    .0
                    .extend(changed);
            }
            continue;
        }
        let style = world
            .get::<WidgetryFileDialogStyle>(root)
            .cloned()
            .unwrap_or_default();
        let full = world
            .get::<Shell>(root)
            .is_some_and(|shell| shell.configured.as_ref() != Some(&style));
        let changed = targets.remove(&root).unwrap_or_default();
        if !full && changed.is_empty() {
            continue;
        }
        let result = apply(world, root, &style, full, changed);
        if let Err(error) = result {
            world.resource_mut::<ConfigurationChanges>().0.insert(root);
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn apply(
    world: &mut World,
    root: Entity,
    style: &WidgetryFileDialogStyle,
    full: bool,
    changed: EntityHashSet,
) -> Result {
    let host = crate::window::content_host(world, root);
    if full {
        let mut node = world
            .get_mut::<Node>(host)
            .ok_or_else(|| contract_error("FileDialog node missing"))?;
        if node.flex_direction != FlexDirection::Column {
            node.flex_direction = FlexDirection::Column;
        }
        if node.row_gap != px(style.spacing) {
            node.row_gap = px(style.spacing);
        }
        if node.padding != UiRect::all(px(style.padding)) {
            node.padding = UiRect::all(px(style.padding));
        }
        let sidebar = world
            .get::<Shell>(root)
            .and_then(|shell| shell.parts.get(&PartKind::Sidebar))
            .copied();
        if let Some(sidebar) = sidebar {
            let mut node = world
                .get_mut::<Node>(sidebar)
                .ok_or_else(|| contract_error("FileDialog sidebar missing"))?;
            if node.width != px(style.sidebar_width) {
                node.width = px(style.sidebar_width);
            }
        }
    }
    let mut stack = if full {
        world
            .get::<Children>(host)
            .map(|children| children.to_vec())
            .unwrap_or_default()
    } else {
        changed
            .into_iter()
            .filter(|entity| inside(world, *entity, host))
            .collect()
    };
    let mut visited = EntityHashSet::default();
    while let Some(entity) = stack.pop() {
        if !visited.insert(entity) {
            continue;
        }
        #[cfg(test)]
        CONFIG_WORK.with(|work| work.set(work.get() + 1));
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
        if world
            .get::<ChildOf>(entity)
            .is_some_and(|parent| parent.parent() == host)
            && let Some(mut node) = world.get_mut::<Node>(entity)
            && node.column_gap != px(style.spacing)
        {
            node.column_gap = px(style.spacing);
        }
        if let Some(mut font) = world.get_mut::<TextFont>(entity)
            && font.font_size != bevy::text::FontSize::Px(style.font_size)
        {
            font.font_size = bevy::text::FontSize::Px(style.font_size);
        }
    }
    if full {
        world
            .get_mut::<Shell>(root)
            .ok_or_else(|| contract_error("FileDialog shell missing"))?
            .configured = Some(style.clone());
    }
    Ok(())
}
