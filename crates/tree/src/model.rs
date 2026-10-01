use crate::WidgetryTreeChildrenState;
use bevy::prelude::*;
use bevy_widgetry_list_view::WidgetryListModel;
use bevy_widgetry_log::widgetry_error;
use std::collections::{HashMap, HashSet};

/// 业务 Tree node 的 marker；hierarchy 与业务 Component 由调用方维护。
#[derive(Component, Default)]
pub struct WidgetryTreeNode;

/// Tree 的 UI authority；expanded 与 selected 不属于业务 node。
#[derive(Debug, Default)]
pub struct WidgetryTreeState {
    /// 保留隐藏 descendant 的展开意图。
    pub(crate) expanded: HashSet<Entity>,
    /// 以 Entity 而非 visible index 表示 selection。
    pub(crate) selected: Option<Entity>,
}

/// Tree 到 ListView 的 projection，不承载业务数据。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryTreeVisibleItem {
    /// 用户业务 node 的 Entity identity。
    pub entity: Entity,
    /// root direct child 从 0 开始的 hierarchy depth。
    pub depth: u16,
    /// 是否存在直接的 Tree node child。
    pub has_children: bool,
    /// 当前 UI 展开意图。
    pub expanded: bool,
}

/// root 是不显示的容器；只遍历带 WidgetryTreeNode 的 direct children。
/// 不跨越未标记的 entity；缺失 root 投影为空。业务 hierarchy 不复制到 model。
#[derive(Component)]
#[require(WidgetryListModel<WidgetryTreeVisibleItem>)]
pub struct WidgetryTreeModel {
    /// 当前 model 的 hierarchy 容器，不作为 visible item。
    root: Entity,
    /// 该 model 唯一 UI authority。
    pub(crate) state: WidgetryTreeState,
    /// DFS preorder 的当前可见 projection。
    visible_items: Vec<WidgetryTreeVisibleItem>,
    /// 当前 visible index cache，永远不作为 selection identity。
    entity_to_index: HashMap<Entity, usize>,
}

impl WidgetryTreeState {
    /// 读取展开意图，隐藏 descendant 仍可以保留展开 state。
    pub fn is_expanded(&self, entity: Entity) -> bool {
        self.expanded.contains(&entity)
    }

    /// 读取 logical selection；不依赖物理 row 是否存在。
    pub fn selected(&self) -> Option<Entity> {
        self.selected
    }
}

impl WidgetryTreeModel {
    /// 创建空 projection；调用 refresh 从当前 World hierarchy 派生内容。
    pub fn new(root: Entity) -> Self {
        Self {
            root,
            state: WidgetryTreeState::default(),
            visible_items: Vec::new(),
            entity_to_index: HashMap::new(),
        }
    }

    /// 读取 hierarchy 容器 Entity。
    pub fn root(&self) -> Entity {
        self.root
    }

    /// 只读访问 UI authority，避免调用方绕过行为 API 改 state。
    pub fn state(&self) -> &WidgetryTreeState {
        &self.state
    }

    /// 只读访问当前 DFS projection。
    pub fn visible_items(&self) -> &[WidgetryTreeVisibleItem] {
        &self.visible_items
    }

    /// 将业务 identity 解析为当前 visible index，隐藏或失效 node 返回 None。
    pub fn visible_index(&self, entity: Entity) -> Option<usize> {
        self.entity_to_index.get(&entity).copied()
    }

    /// 根据 World 更新 projection；不改写 hierarchy 或业务 Component。
    pub fn refresh(&mut self, world: &World) {
        let (items, reachable) = self.projection(world);
        self.apply_projection(items, &reachable);
    }

    /// 同时遍历隐藏 node 以修复失效 identity，避免递归深树消耗 native stack。
    pub(crate) fn projection(
        &self,
        world: &World,
    ) -> (Vec<WidgetryTreeVisibleItem>, HashSet<Entity>) {
        let mut items = Vec::new();
        let mut reachable = HashSet::new();
        let mut stack = tree_children(world, self.root)
            .into_iter()
            .rev()
            .map(|entity| (entity, 0usize, true))
            .collect::<Vec<_>>();
        while let Some((entity, depth, visible)) = stack.pop() {
            // 对非法 ECS cycle 保持有界遍历；合法 hierarchy 每个 node 仅出现一次。
            if !reachable.insert(entity) {
                continue;
            }
            let children = tree_children(world, entity);
            let expanded = self.state.is_expanded(entity);
            if visible {
                let Ok(depth) = u16::try_from(depth) else {
                    widgetry_error!(?entity, depth, "Tree depth 超出 u16 范围");
                    panic!("WidgetryTree depth overflow");
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
        (items, reachable)
    }

    /// cache 与 UI authority 在同一次 projection 更新中修复。
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

/// 未标记 entity 是 Tree hierarchy 的边界，不跨越其 descendant。
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 缺失 root、空容器、未标记 child 都不产生 Tree row。
    #[test]
    fn empty_and_unmarked_hierarchies_are_empty() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let child = world.spawn(ChildOf(root)).id();
        world.spawn((WidgetryTreeNode, ChildOf(child)));
        let mut model = WidgetryTreeModel::new(root);
        model.refresh(&world);
        assert!(model.visible_items().is_empty());
        world.entity_mut(root).despawn();
        model.refresh(&world);
        assert!(model.visible_items().is_empty());
    }

    /// 仅 expanded ancestor 的 descendant 可见，DFS 顺序和 depth 对齐真实 hierarchy。
    #[test]
    fn flatten_uses_hierarchy_order_and_expansion() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let a = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let b = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let c = world.spawn((WidgetryTreeNode, ChildOf(a))).id();
        let d = world.spawn((WidgetryTreeNode, ChildOf(c))).id();
        let mut model = WidgetryTreeModel::new(root);
        model.refresh(&world);
        assert_eq!(
            model
                .visible_items()
                .iter()
                .map(|i| (i.entity, i.depth, i.has_children))
                .collect::<Vec<_>>(),
            vec![(a, 0, true), (b, 0, false)]
        );
        model.state.expanded.extend([a, c]);
        model.refresh(&world);
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

    /// 外部删除、reparent 与 marker 移除后 cache 和失效 UI identity 立即修复。
    #[test]
    fn external_mutation_repairs_projection_and_state() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let a = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let b = world.spawn((WidgetryTreeNode, ChildOf(a))).id();
        let mut model = WidgetryTreeModel::new(root);
        model.state.expanded.insert(a);
        model.state.selected = Some(b);
        model.refresh(&world);
        assert_eq!(model.visible_index(b), Some(1));
        world.entity_mut(b).insert(ChildOf(root));
        model.refresh(&world);
        assert_eq!(model.visible_items()[1].depth, 0);
        assert!(!model.visible_items()[0].has_children);
        world.entity_mut(b).remove::<WidgetryTreeNode>();
        model.refresh(&world);
        assert_eq!(model.state().selected(), None);
        assert_eq!(model.visible_index(b), None);
        world.entity_mut(a).despawn();
        model.refresh(&world);
        assert!(!model.state().is_expanded(a));
        assert!(model.visible_items().is_empty());
    }

    /// 深 hierarchy 的展开 projection 使用迭代遍历；隐藏后仍维护 descendant identity 与展开意图。
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
        model.refresh(&world);
        assert_eq!(model.visible_items().len(), nodes.len());
        for (index, item) in model.visible_items().iter().enumerate() {
            assert_eq!(item.entity, nodes[index]);
            assert_eq!(usize::from(item.depth), index);
            assert_eq!(model.visible_index(item.entity), Some(index));
        }
        model.state.expanded.remove(&nodes[0]);
        model.refresh(&world);
        assert_eq!(model.visible_items().len(), 1);
        assert_eq!(model.state().selected(), Some(parent));
        assert!(model.state().is_expanded(parent));
        assert_eq!(model.visible_index(parent), None);
    }
}
