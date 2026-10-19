use crate::WidgetryTreeChildrenState;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Component, Default)]
pub struct WidgetryTreeNode;

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[component(immutable)]
pub struct WidgetryTreeState {
    pub(crate) expanded: HashSet<Entity>,
    pub(crate) visible_items: Vec<WidgetryTreeVisibleItem>,
    pub(crate) entity_to_index: HashMap<Entity, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryTreeVisibleItem {
    pub entity: Entity,
    pub depth: u16,
    pub has_children: bool,
    pub expanded: bool,
}

#[derive(Component)]
pub struct WidgetryTreeModel {
    root: Entity,
}

impl WidgetryTreeState {
    pub fn is_expanded(&self, entity: Entity) -> bool {
        self.expanded.contains(&entity)
    }

    pub fn expanded_entities(&self) -> &HashSet<Entity> {
        &self.expanded
    }

    pub fn visible_items(&self) -> &[WidgetryTreeVisibleItem] {
        &self.visible_items
    }

    pub fn visible_index(&self, entity: Entity) -> Option<usize> {
        self.entity_to_index.get(&entity).copied()
    }

    pub(crate) fn projection(
        &self,
        world: &World,
        root: Entity,
    ) -> Result<(Vec<WidgetryTreeVisibleItem>, HashSet<Entity>), BevyError> {
        let mut items = Vec::new();
        let mut reachable = HashSet::new();
        let mut stack = tree_children(world, root)
            .into_iter()
            .rev()
            .map(|entity| (entity, 0usize, true))
            .collect::<Vec<_>>();
        while let Some((entity, depth, visible)) = stack.pop() {
            // An invalid ECS cycle would make DFS run indefinitely without deduplication.
            // Deduplicate with reachable to bound traversal and emit each valid node only once.
            if !reachable.insert(entity) {
                continue;
            }
            let children = tree_children(world, entity);
            let expanded = self.is_expanded(entity);
            if visible {
                let Ok(depth) = u16::try_from(depth) else {
                    return Err(BevyError::error("WidgetryTree depth overflow"));
                };
                items.push(WidgetryTreeVisibleItem {
                    entity,
                    depth,
                    has_children: !children.is_empty()
                        || world
                            .get::<WidgetryTreeChildrenState>(entity)
                            .is_some_and(|state| *state != WidgetryTreeChildrenState::Loaded),
                    expanded,
                });
            }
            stack.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, depth + 1, visible && expanded)),
            );
        }
        Ok((items, reachable))
    }

    pub(crate) fn apply_projection(
        &mut self,
        items: Vec<WidgetryTreeVisibleItem>,
        reachable: &HashSet<Entity>,
    ) {
        self.expanded.retain(|entity| reachable.contains(entity));
        self.entity_to_index = items
            .iter()
            .enumerate()
            .map(|(index, item)| (item.entity, index))
            .collect();
        self.visible_items = items;
    }
}

fn tree_children(world: &World, parent: Entity) -> Vec<Entity> {
    world
        .get::<Children>(parent)
        .map(|children| {
            children
                .iter()
                .filter(|&child| world.get::<WidgetryTreeNode>(child).is_some())
                .collect()
        })
        .unwrap_or_default()
}

impl WidgetryTreeModel {
    pub fn new(root: Entity) -> Self {
        Self { root }
    }
    pub fn root(&self) -> Entity {
        self.root
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    #[test]
    fn projection_obeys_hierarchy_order_and_hidden_membership() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let a = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let b = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let c = world.spawn((WidgetryTreeNode, ChildOf(a))).id();
        let mut state = WidgetryTreeState::default();
        let (items, reachable) = state.projection(&world, root).unwrap();
        assert_eq!(
            items.iter().map(|item| item.entity).collect::<Vec<_>>(),
            vec![a, b]
        );
        assert!(reachable.contains(&c));
        state.expanded.insert(a);
        let (items, reachable) = state.projection(&world, root).unwrap();
        state.apply_projection(items, &reachable);
        assert_eq!(state.visible_index(c), Some(1));
        assert_eq!(state.visible_items()[1].depth, 1);
        world.entity_mut(a).remove::<WidgetryTreeNode>();
        let (items, reachable) = state.projection(&world, root).unwrap();
        state.apply_projection(items, &reachable);
        assert_eq!(state.visible_index(c), None);
        assert!(!state.is_expanded(a));
    }
    #[test]
    fn deep_hierarchy_uses_iterative_projection() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let mut parent = root;
        let mut state = WidgetryTreeState::default();
        for _ in 0..2048 {
            parent = world.spawn((WidgetryTreeNode, ChildOf(parent))).id();
            state.expanded.insert(parent);
        }
        let (items, reachable) = state.projection(&world, root).unwrap();
        state.apply_projection(items, &reachable);
        assert_eq!(state.visible_index(parent), Some(2047));
        assert_eq!(state.visible_items()[2047].depth, 2047);
    }
}
