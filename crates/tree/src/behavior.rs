use crate::{WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeVisibleItem};
use bevy::prelude::*;
use bevy_widgetry_list_view::WidgetryListModel;
use bevy_widgetry_log::widgetry_error;
use std::collections::{HashMap, HashSet};

/// 调用方维护的 lazy children lifecycle；无此 Component 的 node 使用已有 hierarchy。
/// Unknown 在首次 expand 时进入 Loading 并发送一次 ChildrenRequested；加载方完成后写入 Loaded。
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryTreeChildrenState {
    /// 尚未请求 children，保留 expander。
    #[default]
    Unknown,
    /// 请求已发出；collapse/re-expand 不重复请求。
    Loading,
    /// 外部加载完成，是否有 children 由真实 hierarchy 决定。
    Loaded,
}

/// Tree 语义 event，target 是 model source，不暴露 ListModel mutation。
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WidgetryTreeEvent {
    /// 持有 WidgetryTreeModel 的 entity。
    pub entity: Entity,
    /// 业务 node 与操作语义。
    pub kind: WidgetryTreeEventKind,
}

/// Expanded/Collapsed 表达 state transition；Selected 仅用于用户确认。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryTreeEventKind {
    /// node 从收起进入展开。
    Expanded(Entity),
    /// node 从展开进入收起。
    Collapsed(Entity),
    /// 用户选择了业务 node。
    Selected(Entity),
    /// 外部 loader 应为 node 创建 children 并更新 lazy state。
    ChildrenRequested(Entity),
}

/// 在 ListView state repair 之前从 ECS hierarchy 同步所有 Tree source。
pub(crate) fn sync_models(world: &mut World) {
    let sources = world
        .query_filtered::<Entity, With<WidgetryTreeModel>>()
        .iter(world)
        .collect::<Vec<_>>();
    for source in sources {
        sync_source(world, source);
    }
}

/// 不替换 ListModel、不 clear；保留仍可见 node 的 entry identity 与未修改内容版本。
fn reconcile_list(
    list: &mut WidgetryListModel<WidgetryTreeVisibleItem>,
    items: &[WidgetryTreeVisibleItem],
) {
    let desired: HashSet<_> = items.iter().map(|item| item.entity).collect();
    for index in (0..list.len()).rev() {
        if list
            .get(index)
            .is_some_and(|item| !desired.contains(&item.entity))
        {
            invariant(list.remove(index));
        }
    }
    let ids: HashMap<_, _> = (0..list.len())
        .map(|index| (invariant(list.get(index)).entity, invariant(list.id(index))))
        .collect();
    for (index, item) in items.iter().enumerate() {
        if list.get(index).is_none_or(|old| old.entity != item.entity) {
            if let Some(id) = ids.get(&item.entity) {
                let from = invariant(list.index_of(*id));
                if !list.move_item(from, index) {
                    invariant::<()>(None);
                }
            } else if list.insert(index, *item).is_err() {
                invariant::<()>(None);
            }
        }
        if list.get(index) != Some(item) {
            *invariant(list.get_mut(index)) = *item;
        }
    }
}

/// 必需内部 source contract 不允许降级为静默失效。
fn invariant<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| {
        widgetry_error!("Tree source 缺少必需的 ListModel 或 projection invariant 失效");
        panic!("WidgetryTree source invariant failed");
    })
}

/// 计算后分离 World borrow，统一更新 Tree projection 与 ListModel。
fn sync_source(world: &mut World, source: Entity) {
    let Some(tree) = world.get::<WidgetryTreeModel>(source) else {
        return;
    };
    let (items, reachable) = tree.projection(world);
    invariant(world.get_mut::<WidgetryTreeModel>(source))
        .apply_projection(items.clone(), &reachable);
    let list = invariant(world.get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source));
    let unchanged = list.len() == items.len()
        && items
            .iter()
            .enumerate()
            .all(|(index, item)| list.get(index) == Some(item));
    if !unchanged {
        let mut list =
            invariant(world.get_mut::<WidgetryListModel<WidgetryTreeVisibleItem>>(source));
        reconcile_list(&mut list, &items);
    }
}

/// 检查真实 ChildOf chain，不依赖上次 update 的 visible cache，也允许隐藏 node。
fn contains_node(world: &World, source: Entity, node: Entity) -> bool {
    let Some(tree) = world.get::<WidgetryTreeModel>(source) else {
        return false;
    };
    let mut current = node;
    let mut seen = HashSet::new();
    while current != tree.root() && seen.insert(current) {
        if world.get::<WidgetryTreeNode>(current).is_none() {
            return false;
        }
        let Some(parent) = world.get::<ChildOf>(current) else {
            return false;
        };
        current = parent.parent();
    }
    node != tree.root() && current == tree.root() && world.get_entity(current).is_ok()
}

impl WidgetryTreeModel {
    /// 展开 source 内的 node，重复、失效或无 children 的普通 leaf 为 no-op。
    /// 成功发送 Expanded；lazy Unknown 同时进入 Loading 并发送 ChildrenRequested。
    pub fn expand(world: &mut World, source: Entity, node: Entity) -> bool {
        if !contains_node(world, source, node) {
            return false;
        }
        let lazy = world.get::<WidgetryTreeChildrenState>(node).copied();
        let has_children = world.get::<Children>(node).is_some_and(|children| {
            children
                .iter()
                .any(|child| world.get::<WidgetryTreeNode>(child).is_some())
        });
        if !has_children && lazy.is_none_or(|state| state == WidgetryTreeChildrenState::Loaded) {
            return false;
        }
        if !invariant(world.get_mut::<Self>(source))
            .state
            .expanded
            .insert(node)
        {
            return false;
        }
        let request = lazy == Some(WidgetryTreeChildrenState::Unknown);
        if request {
            world
                .entity_mut(node)
                .insert(WidgetryTreeChildrenState::Loading);
        }
        sync_source(world, source);
        world.trigger(WidgetryTreeEvent {
            entity: source,
            kind: WidgetryTreeEventKind::Expanded(node),
        });
        if request {
            world.trigger(WidgetryTreeEvent {
                entity: source,
                kind: WidgetryTreeEventKind::ChildrenRequested(node),
            });
        }
        true
    }

    /// 收起 node，保留 descendant 展开意图与隐藏 selection；重复或失效操作为 no-op。
    pub fn collapse(world: &mut World, source: Entity, node: Entity) -> bool {
        if !contains_node(world, source, node)
            || !invariant(world.get_mut::<Self>(source))
                .state
                .expanded
                .remove(&node)
        {
            return false;
        }
        sync_source(world, source);
        world.trigger(WidgetryTreeEvent {
            entity: source,
            kind: WidgetryTreeEventKind::Collapsed(node),
        });
        true
    }

    /// 按当前展开意图执行 expand 或 collapse。
    pub fn toggle_expand(world: &mut World, source: Entity, node: Entity) -> bool {
        if world
            .get::<Self>(source)
            .is_some_and(|tree| tree.state.is_expanded(node))
        {
            Self::collapse(world, source, node)
        } else {
            Self::expand(world, source, node)
        }
    }

    /// 静默更新 Entity selection，允许隐藏但可达的 node；None 清空。
    /// source/node 失效或相同值返回 false；不受 view disabled 限制。
    pub fn select(world: &mut World, source: Entity, node: Option<Entity>) -> bool {
        if world.get::<Self>(source).is_none()
            || node.is_some_and(|node| !contains_node(world, source, node))
        {
            return false;
        }
        let mut tree = invariant(world.get_mut::<Self>(source));
        if tree.state.selected == node {
            return false;
        }
        tree.state.selected = node;
        true
    }
}
