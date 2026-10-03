use crate::WidgetryTreeChildrenState;
use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_list_view::WidgetryListModel;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::collections::{HashMap, HashSet};

#[derive(Component, Default)]
pub struct WidgetryTreeNode;

#[derive(Clone, Debug, Default)]
pub struct WidgetryTreeState {
    pub(crate) expanded: HashSet<Entity>,
    pub(crate) selected: Option<Entity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryTreeVisibleItem {
    pub entity: Entity,
    pub depth: u16,
    pub has_children: bool,
    pub expanded: bool,
}

#[derive(Component)]
#[require(WidgetryListModel<WidgetryTreeVisibleItem>)]
pub struct WidgetryTreeModel {
    root: Entity,
    pub(crate) diagnostics: FailureState,
    pub(crate) state: WidgetryTreeState,
    pub(crate) visible_items: Vec<WidgetryTreeVisibleItem>,
    pub(crate) entity_to_index: HashMap<Entity, usize>,
}

impl WidgetryTreeState {
    pub fn is_expanded(&self, entity: Entity) -> bool {
        self.expanded.contains(&entity)
    }

    pub fn selected(&self) -> Option<Entity> {
        self.selected
    }
}

impl WidgetryTreeModel {
    pub fn new(root: Entity) -> Self {
        Self {
            root,
            diagnostics: FailureState::default(),
            state: WidgetryTreeState::default(),
            visible_items: Vec::new(),
            entity_to_index: HashMap::new(),
        }
    }

    pub fn root(&self) -> Entity {
        self.root
    }

    pub fn state(&self) -> &WidgetryTreeState {
        &self.state
    }

    pub fn visible_items(&self) -> &[WidgetryTreeVisibleItem] {
        &self.visible_items
    }

    pub fn visible_index(&self, entity: Entity) -> Option<usize> {
        self.entity_to_index.get(&entity).copied()
    }

    pub fn refresh(&mut self, world: &World) -> Result<(), BevyError> {
        let result = self.projection(world);
        let root = self.root;
        let (items, reachable) = self.diagnostics.observe(
            result,
            |error| widgetry_error!(?root, %error, "Tree projection 失败"),
            || widgetry_info!(?root, "Tree projection 恢复正常"),
        )?;
        self.apply_projection(items, &reachable);
        Ok(())
    }

    pub(crate) fn projection(
        &self,
        world: &World,
    ) -> Result<(Vec<WidgetryTreeVisibleItem>, HashSet<Entity>), BevyError> {
        let mut items = Vec::new();
        let mut reachable = HashSet::new();
        let mut stack = tree_children(world, self.root)
            .into_iter()
            .rev()
            .map(|entity| (entity, 0usize, true))
            .collect::<Vec<_>>();
        while let Some((entity, depth, visible)) = stack.pop() {
            // 非法 ECS cycle 会让未去重的 DFS 永不结束；用 reachable 去重，使遍历保持有界并且合法 node 只出现一次。
            if !reachable.insert(entity) {
                continue;
            }
            let children = tree_children(world, entity);
            let expanded = self.state.is_expanded(entity);
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
        self.state
            .expanded
            .retain(|entity| reachable.contains(entity));
        if self
            .state
            .selected
            .is_some_and(|entity| !reachable.contains(&entity))
        {
            self.state.selected = None;
        }
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

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_unmarked_hierarchies_are_empty() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let child = world.spawn(ChildOf(root)).id();
        world.spawn((WidgetryTreeNode, ChildOf(child)));
        let mut model = WidgetryTreeModel::new(root);
        model.refresh(&world).unwrap();
        assert!(model.visible_items().is_empty());
        world.entity_mut(root).despawn();
        model.refresh(&world).unwrap();
        assert!(model.visible_items().is_empty());
    }

    #[test]
    fn flatten_uses_hierarchy_order_and_expansion() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let a = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let b = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let c = world.spawn((WidgetryTreeNode, ChildOf(a))).id();
        let d = world.spawn((WidgetryTreeNode, ChildOf(c))).id();
        let mut model = WidgetryTreeModel::new(root);
        model.refresh(&world).unwrap();
        assert_eq!(
            model
                .visible_items()
                .iter()
                .map(|i| (i.entity, i.depth, i.has_children))
                .collect::<Vec<_>>(),
            vec![(a, 0, true), (b, 0, false)]
        );
        model.state.expanded.extend([a, c]);
        model.refresh(&world).unwrap();
        assert_eq!(
            model
                .visible_items()
                .iter()
                .map(|i| (i.entity, i.depth))
                .collect::<Vec<_>>(),
            vec![(a, 0), (c, 1), (d, 2), (b, 0)]
        );
        for (index, item) in model.visible_items().iter().enumerate() {
            assert_eq!(model.visible_index(item.entity), Some(index));
        }
        assert_eq!(model.visible_index(root), None);
    }

    #[test]
    fn external_mutation_repairs_projection_and_state() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let a = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let b = world.spawn((WidgetryTreeNode, ChildOf(a))).id();
        let mut model = WidgetryTreeModel::new(root);
        model.state.expanded.insert(a);
        model.state.selected = Some(b);
        model.refresh(&world).unwrap();
        assert_eq!(model.visible_index(b), Some(1));
        world.entity_mut(b).insert(ChildOf(root));
        model.refresh(&world).unwrap();
        assert_eq!(model.visible_items()[1].depth, 0);
        assert!(!model.visible_items()[0].has_children);
        world.entity_mut(b).remove::<WidgetryTreeNode>();
        model.refresh(&world).unwrap();
        assert_eq!(model.state().selected(), None);
        assert_eq!(model.visible_index(b), None);
        world.entity_mut(a).despawn();
        model.refresh(&world).unwrap();
        assert!(!model.state().is_expanded(a));
        assert!(model.visible_items().is_empty());
    }

    #[test]
    fn deep_hierarchy_projects_without_recursive_traversal() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let mut model = WidgetryTreeModel::new(root);
        let mut parent = root;
        let mut nodes = Vec::new();
        for _ in 0..2048 {
            let node = world.spawn((WidgetryTreeNode, ChildOf(parent))).id();
            model.state.expanded.insert(node);
            nodes.push(node);
            parent = node;
        }
        model.state.selected = Some(parent);
        model.refresh(&world).unwrap();
        assert_eq!(model.visible_items().len(), nodes.len());
        for (index, item) in model.visible_items().iter().enumerate() {
            assert_eq!(item.entity, nodes[index]);
            assert_eq!(usize::from(item.depth), index);
            assert_eq!(model.visible_index(item.entity), Some(index));
        }
        model.state.expanded.remove(&nodes[0]);
        model.refresh(&world).unwrap();
        assert_eq!(model.visible_items().len(), 1);
        assert_eq!(model.state().selected(), Some(parent));
        assert!(model.state().is_expanded(parent));
        assert_eq!(model.visible_index(parent), None);
    }
}
