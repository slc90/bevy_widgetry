use crate::tabs::{Pair, Parts, State, TabOwner, TabParts};
use crate::{WidgetryTab, WidgetryTabs};
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_internal_log::widgetry_error;

pub(crate) fn invalid(reason: &str) -> BevyError {
    widgetry_error!(reason, "Tabs 请求或结构无效");
    bevy_widgetry_core::scene::logged_error(reason)
}

#[derive(Resource, Default)]
struct Pending(EntityHashSet);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Pending>().add_observer(
        |event: On<Insert<Parts>>, mut pending: ResMut<Pending>| {
            pending.0.insert(event.entity);
        },
    );
    let world = app.world_mut();
    let roots = world
        .query_filtered::<Entity, With<WidgetryTabs>>()
        .iter(world)
        .collect::<Vec<_>>();
    world.resource_mut::<Pending>().0.extend(roots);
}

pub(crate) fn initialize(world: &mut World) -> Result<(), BevyError> {
    let roots = std::mem::take(&mut world.resource_mut::<Pending>().0);
    let mut failure = None;
    for root in roots {
        if world.get::<WidgetryTabs>(root).is_none() {
            continue;
        }
        if let Err(error) = prepare(world, root)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn structure(world: &World, root: Entity) -> Result<Vec<Pair>, BevyError> {
    if world.get::<WidgetryTabs>(root).is_none() {
        return Err(invalid("WidgetryTabs root is missing or invalid"));
    }
    let parts = world
        .get::<Parts>(root)
        .ok_or_else(|| invalid("Tabs parts are missing"))?;
    let state = world
        .get::<State>(root)
        .ok_or_else(|| invalid("Tabs state is missing"))?;
    for container in [parts.bar, parts.pages] {
        if world.get::<Node>(container).is_none()
            || world
                .get::<ChildOf>(container)
                .is_none_or(|parent| parent.parent() != root)
        {
            return Err(invalid("Tabs container structure is invalid"));
        }
    }
    let headers = world
        .get::<Children>(parts.bar)
        .ok_or_else(|| invalid("Tabs requires at least one tab"))?;
    if headers.is_empty() {
        return Err(invalid("Tabs requires at least one tab"));
    }
    let mut pairs = Vec::with_capacity(headers.len());
    for header in headers {
        if world.get::<WidgetryTab>(*header).is_none() || world.get::<Node>(*header).is_none() {
            return Err(invalid("Tabs bar requires WidgetryTab children"));
        }
        let tab = world
            .get::<TabParts>(*header)
            .ok_or_else(|| invalid("Tab page pairing is missing"))?;
        let expected_parent = if state.pairs.is_empty() {
            *header
        } else {
            parts.pages
        };
        if world.get::<Node>(tab.page).is_none()
            || world
                .get::<ChildOf>(tab.page)
                .is_none_or(|parent| parent.parent() != expected_parent)
        {
            return Err(invalid("Tab content page structure is invalid"));
        }
        pairs.push(Pair {
            header: *header,
            page: tab.page,
        });
    }
    if state.selected >= pairs.len() {
        return Err(invalid("Tabs initial selected index is out of bounds"));
    }
    if !state.pairs.is_empty() {
        if state.pairs != pairs {
            return Err(invalid("Tabs items are fixed after construction"));
        }
        if world
            .get::<Children>(parts.pages)
            .is_none_or(|children| children.len() != pairs.len())
        {
            return Err(invalid("Tabs page container has invalid children"));
        }
    }
    Ok(pairs)
}

fn prepare(world: &mut World, root: Entity) -> Result<Vec<Pair>, BevyError> {
    let pairs = structure(world, root)?;
    let Some(state) = world.get::<State>(root) else {
        return Err(invalid("Tabs state is missing"));
    };
    if state.pairs.is_empty() {
        let selected = state.selected;
        let pages = world
            .get::<Parts>(root)
            .ok_or_else(|| invalid("Tabs parts are missing"))?
            .pages;
        for (index, pair) in pairs.iter().enumerate() {
            world
                .entity_mut(pair.header)
                .insert(TabOwner { root, index });
            world.entity_mut(pair.page).insert(ChildOf(pages));
        }
        project(world, &pairs, selected);
        world
            .get_mut::<State>(root)
            .ok_or_else(|| invalid("Tabs state is missing"))?
            .pairs = pairs.clone();
    }
    Ok(pairs)
}

fn project(world: &mut World, pairs: &[Pair], selected: usize) {
    for (index, pair) in pairs.iter().enumerate() {
        world.entity_mut(pair.page).insert(if index == selected {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        // Keyboard dispatch can run before Bevy's PostUpdate visibility propagation.
        // Keep its derived navigation inputs consistent throughout the retained page subtree.
        project_inherited_visibility(world, pair.page);
        if index == selected {
            world.entity_mut(pair.header).insert(Checked);
        } else {
            world.entity_mut(pair.header).remove::<Checked>();
        }
        if let Some(mut node) = world.get_mut::<Node>(pair.page) {
            node.display = if index == selected {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

fn project_inherited_visibility(world: &mut World, page: Entity) {
    let mut ancestor = world.get::<ChildOf>(page).map(ChildOf::parent);
    let mut parent_visible = true;
    while let Some(entity) = ancestor {
        match world.get::<Visibility>(entity) {
            Some(Visibility::Hidden) => {
                parent_visible = false;
                break;
            }
            Some(Visibility::Inherited) => {
                ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent)
            }
            _ => break,
        }
    }
    let mut pending = vec![(page, parent_visible)];
    while let Some((entity, parent_visible)) = pending.pop() {
        let visible = match world.get::<Visibility>(entity) {
            Some(Visibility::Hidden) => false,
            Some(Visibility::Visible) => true,
            Some(Visibility::Inherited) => parent_visible,
            None => continue,
        };
        let Some(mut inherited) = world.get_mut::<InheritedVisibility>(entity) else {
            continue;
        };
        inherited.set_if_neq(if visible {
            InheritedVisibility::VISIBLE
        } else {
            InheritedVisibility::HIDDEN
        });
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter().map(|child| (child, visible)));
        }
    }
}

fn inside(world: &World, mut entity: Entity, ancestor: Entity) -> bool {
    loop {
        if entity == ancestor {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return false;
        };
        entity = parent.parent();
    }
}

impl WidgetryTabs {
    pub fn selected_index(world: &World, root: Entity) -> Result<usize, BevyError> {
        structure(world, root)?;
        Ok(world
            .get::<State>(root)
            .ok_or_else(|| invalid("Tabs state is missing"))?
            .selected)
    }

    pub fn header(world: &World, root: Entity, index: usize) -> Result<Entity, BevyError> {
        structure(world, root)?
            .get(index)
            .map(|pair| pair.header)
            .ok_or_else(|| invalid("Tabs index is out of bounds"))
    }

    pub fn page(world: &World, root: Entity, index: usize) -> Result<Entity, BevyError> {
        structure(world, root)?
            .get(index)
            .map(|pair| pair.page)
            .ok_or_else(|| invalid("Tabs index is out of bounds"))
    }

    pub fn set_selected(commands: &mut Commands, root: Entity, index: usize) {
        commands.queue(move |world: &mut World| {
            Self::set_selected_in_world(world, root, index).map(|_| ())
        });
    }

    pub fn set_selected_in_world(
        world: &mut World,
        root: Entity,
        index: usize,
    ) -> Result<bool, BevyError> {
        // Validate the complete request before initialization can reparent or project anything.
        let pairs = structure(world, root)?;
        if index >= pairs.len() {
            return Err(invalid("Tabs index is out of bounds"));
        }
        prepare(world, root)?;
        let old = world
            .get::<State>(root)
            .ok_or_else(|| invalid("Tabs state is missing"))?
            .selected;
        if old == index {
            return Ok(false);
        }
        if world
            .get_resource::<InputFocus>()
            .and_then(InputFocus::get)
            .is_some_and(|focused| inside(world, focused, pairs[old].page))
        {
            world.resource_mut::<InputFocus>().clear();
        }
        world
            .get_mut::<State>(root)
            .ok_or_else(|| invalid("Tabs state is missing"))?
            .selected = index;
        project(world, &pairs, index);
        world.trigger(ValueChange {
            source: root,
            value: index,
            is_final: true,
        });
        Ok(true)
    }
}
