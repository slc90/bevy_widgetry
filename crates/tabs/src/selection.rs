use crate::error::{failure, missing, structural};
use crate::tabs::{Pair, Parts, State, TabOwner, TabParts};
use crate::{TabsError, TabsFailure, TabsStructureError, WidgetryTab, WidgetryTabs};
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::DiagnosticBatch};

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

pub(crate) fn initialize(world: &mut World) -> Result {
    let roots = std::mem::take(&mut world.resource_mut::<Pending>().0);
    let mut batch = DiagnosticBatch::default();
    for root in roots {
        if world.get::<WidgetryTabs>(root).is_none() {
            continue;
        }
        if let Err(error) = prepare(world, root) {
            batch.push(error.with_context(ErrorFrame::new("tabs.initialize").field("root", root)));
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

fn parent(
    world: &World,
    root: Entity,
    entity: Entity,
    expected: Entity,
    part: &'static str,
) -> Result<(), TabsFailure> {
    let actual = world.get::<ChildOf>(entity).map(ChildOf::parent);
    if actual != Some(expected) {
        return Err(structural(
            TabsStructureError::InvalidParent {
                root,
                entity,
                part,
                expected,
                actual,
            },
            "tabs.structure",
        ));
    }
    Ok(())
}

fn structure(world: &World, root: Entity) -> Result<Vec<Pair>, TabsFailure> {
    if world.get::<WidgetryTabs>(root).is_none() {
        return Err(failure(
            TabsError::InvalidTarget {
                entity: root,
                expected: "WidgetryTabs",
            },
            ErrorFrame::new("tabs.structure"),
        ));
    }
    let parts = world
        .get::<Parts>(root)
        .ok_or_else(|| missing(root, root, "Parts"))?;
    let state = world
        .get::<State>(root)
        .ok_or_else(|| missing(root, root, "State"))?;
    for (container, part) in [(parts.bar, "bar"), (parts.pages, "pages")] {
        if world.get::<Node>(container).is_none() {
            return Err(missing(root, container, "Node"));
        }
        parent(world, root, container, root, part)?;
    }
    let strip = crate::scrolling::strip(world, root, parts.bar)?;
    let children = world.get::<Children>(strip);
    let headers: Vec<_> = children
        .into_iter()
        .flat_map(|children| children.iter())
        .filter(|entity| world.get::<WidgetryTab>(*entity).is_some())
        .collect();
    if headers.is_empty() {
        if !state.pairs.is_empty() {
            return Err(fixed_pairs_error(root, &state.pairs, &[]));
        }
        return Err(structural(
            TabsStructureError::Empty { root, strip },
            "tabs.structure",
        ));
    }
    let mut pairs = Vec::with_capacity(headers.len());
    let mut allowed: EntityHashSet = headers.iter().copied().collect();
    for header in &headers {
        if world.get::<Node>(*header).is_none() {
            return Err(missing(root, *header, "Node"));
        }
        let tab = world
            .get::<TabParts>(*header)
            .ok_or_else(|| missing(root, *header, "TabParts"))?;
        allowed.insert(tab.ellipsis);
        let expected_parent = if state.pairs.is_empty() {
            *header
        } else {
            parts.pages
        };
        if world.get::<Node>(tab.page).is_none() {
            return Err(missing(root, tab.page, "Node"));
        }
        parent(world, root, tab.page, expected_parent, "page")?;
        pairs.push(Pair {
            header: *header,
            page: tab.page,
        });
    }
    if let Some(child) = children
        .into_iter()
        .flat_map(|children| children.iter())
        .find(|child| !allowed.contains(child))
    {
        return Err(structural(
            TabsStructureError::UnexpectedChild {
                root,
                parent: strip,
                entity: child,
            },
            "tabs.structure",
        ));
    }
    if !state.pairs.is_empty() && state.pairs != pairs {
        return Err(fixed_pairs_error(root, &state.pairs, &pairs));
    }
    if state.selected >= pairs.len() {
        return Err(index_error(
            root,
            state.selected,
            pairs.len(),
            state.pairs.is_empty(),
        ));
    }
    if !state.pairs.is_empty() {
        let actual = world.get::<Children>(parts.pages).map_or(0, Children::len);
        if actual != pairs.len() {
            return Err(structural(
                TabsStructureError::PageCount {
                    root,
                    container: parts.pages,
                    expected: pairs.len(),
                    actual,
                },
                "tabs.structure",
            ));
        }
    }
    Ok(pairs)
}

fn fixed_pairs_error(root: Entity, expected: &[Pair], actual: &[Pair]) -> TabsFailure {
    failure(
        TabsError::FixedPairs {
            root,
            expected: expected
                .iter()
                .map(|pair| [pair.header, pair.page])
                .collect(),
            actual: actual.iter().map(|pair| [pair.header, pair.page]).collect(),
        },
        ErrorFrame::new("tabs.structure"),
    )
}

fn index_error(root: Entity, index: usize, count: usize, initial: bool) -> TabsFailure {
    failure(
        TabsError::InvalidIndex {
            root,
            index,
            count,
            initial,
        },
        ErrorFrame::new("tabs.validate_index"),
    )
}

fn prepare(world: &mut World, root: Entity) -> Result<Vec<Pair>, TabsFailure> {
    let pairs = structure(world, root)?;
    let Some(state) = world.get::<State>(root) else {
        return Err(missing(root, root, "State"));
    };
    if state.pairs.is_empty() {
        let selected = state.selected;
        let pages = world
            .get::<Parts>(root)
            .ok_or_else(|| missing(root, root, "Parts"))?
            .pages;
        let bar = world
            .get::<Parts>(root)
            .ok_or_else(|| missing(root, root, "Parts"))?
            .bar;
        let strip = crate::scrolling::strip(world, root, bar)?;
        crate::scrolling::initialize(world, bar, root).map_err(|error| {
            error.with_context(ErrorFrame::new("tabs.prepare.scrollbar").field("root", root))
        })?;
        if world
            .get::<State>(root)
            .is_some_and(|state| state.orientation == crate::TabsOrientation::Vertical)
            && let Some(mut node) = world.get_mut::<Node>(strip)
        {
            node.width = percent(100);
            node.max_width = percent(100);
        }
        world
            .entity_mut(bar)
            .insert(crate::style::FrameOwner::new(root));
        world
            .entity_mut(pages)
            .insert(crate::style::FrameOwner::new(root));
        for (index, pair) in pairs.iter().enumerate() {
            world.entity_mut(pair.header).insert((
                TabOwner { root, index },
                crate::scrolling::HeaderGeometry::default(),
            ));
            if let Some(parts) = world.get::<TabParts>(pair.header) {
                let ellipsis = parts.ellipsis;
                world
                    .entity_mut(ellipsis)
                    .insert((crate::style::LabelOwner::new(pair.header), ChildOf(strip)));
            }
            world.entity_mut(pair.page).insert(ChildOf(pages));
        }
        project(world, &pairs, selected);
        world
            .get_mut::<State>(root)
            .ok_or_else(|| missing(root, root, "State"))?
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
    pub fn selected_index(world: &World, root: Entity) -> Result<usize, TabsFailure> {
        structure(world, root)?;
        Ok(world
            .get::<State>(root)
            .ok_or_else(|| missing(root, root, "State"))?
            .selected)
    }

    pub fn header(world: &World, root: Entity, index: usize) -> Result<Entity, TabsFailure> {
        let pairs = structure(world, root)?;
        pairs
            .get(index)
            .map(|pair| pair.header)
            .ok_or_else(|| index_error(root, index, pairs.len(), false))
    }

    pub fn page(world: &World, root: Entity, index: usize) -> Result<Entity, TabsFailure> {
        let pairs = structure(world, root)?;
        pairs
            .get(index)
            .map(|pair| pair.page)
            .ok_or_else(|| index_error(root, index, pairs.len(), false))
    }

    pub fn set_selected(commands: &mut Commands, root: Entity, index: usize) {
        commands.queue(move |world: &mut World| {
            Self::set_selected_in_world(world, root, index)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }

    pub fn set_selected_in_world(
        world: &mut World,
        root: Entity,
        index: usize,
    ) -> Result<bool, TabsFailure> {
        // Validate the complete request before initialization can reparent or project anything.
        let pairs = structure(world, root).map_err(|error| {
            error.with_context(
                ErrorFrame::new("tabs.set_selected.validate")
                    .field("root", root)
                    .field("index", index),
            )
        })?;
        if index >= pairs.len() {
            return Err(index_error(root, index, pairs.len(), false));
        }
        prepare(world, root).map_err(|error| {
            error.with_context(
                ErrorFrame::new("tabs.set_selected.prepare")
                    .field("root", root)
                    .field("index", index),
            )
        })?;
        let old = world
            .get::<State>(root)
            .ok_or_else(|| missing(root, root, "State"))?
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
            .ok_or_else(|| missing(root, root, "State"))?
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

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn missing_runtime_parts_keep_root_header_and_part_identity() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(crate::WidgetryTabsPlugin);
        let root = app.world_mut().spawn_scene(bsn! {
            @WidgetryTabs { @children: bsn! { @WidgetryTab { @header: bsn! { Node }, @content: bsn! { Node } } } }
        }).unwrap().id();
        let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
        let parts = app.world_mut().entity_mut(root).take::<Parts>().unwrap();
        let failure = WidgetryTabs::selected_index(app.world(), root).unwrap_err();
        assert!(
            matches!(failure.kind(), TabsError::Structure { source: TabsStructureError::MissingPart { root: actual, entity, part: "Parts" } } if *actual == root && *entity == root)
        );
        app.world_mut().entity_mut(root).insert(parts);
        let state = app.world_mut().entity_mut(root).take::<State>().unwrap();
        let failure = WidgetryTabs::selected_index(app.world(), root).unwrap_err();
        assert!(
            matches!(failure.kind(), TabsError::Structure { source: TabsStructureError::MissingPart { root: actual, entity, part: "State" } } if *actual == root && *entity == root)
        );
        app.world_mut().entity_mut(root).insert(state);
        app.world_mut().entity_mut(header).remove::<TabParts>();
        let failure = WidgetryTabs::selected_index(app.world(), root).unwrap_err();
        assert!(
            matches!(failure.kind(), TabsError::Structure { source: TabsStructureError::MissingPart { root: actual, entity, part: "TabParts" } } if *actual == root && *entity == header)
        );
    }
}
