use crate::{WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeVisibleItem};
use bevy::prelude::*;
use bevy_widgetry_list_view::WidgetryListModel;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::collections::{HashMap, HashSet};

/// 调用方维护的 lazy children lifecycle；无此 Component 的 node 使用已有 hierarchy。
/// Unknown 在首次 expand 时进入 Loading 并发送一次 ChildrenRequested。
/// 初始化可插入 Unknown/Loaded；加载方先创建业务 children，再调用 set_loaded 完成请求。
/// 运行期替换/移除此 Component 不属于支持的 loader state 更新，不保证请求去重。
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[component(immutable)]
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

/// UI 与程序操作共享 Model state transition 通知，先提交 authority 再发送。
/// 初始化、projection repair 与合法同值不通知；不承诺多个 observer 的执行顺序。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryTreeEventKind {
    /// node 从收起进入展开。
    Expanded(Entity),
    /// node 从展开进入收起。
    Collapsed(Entity),
    /// UI 或程序更新后的 selection；None 表达显式清空。
    Selected(Option<Entity>),
    /// 外部 loader 应为 node 创建 children 并更新 lazy state。
    ChildrenRequested(Entity),
}

/// 在 ListView state repair 之前从 ECS hierarchy 同步所有 Tree source。
pub(crate) fn sync_models(world: &mut World) -> Result<(), BevyError> {
    let sources = world
        .query_filtered::<Entity, With<WidgetryTreeModel>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for source in sources {
        if let Err(error) = sync_source(world, source)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

/// 不替换 ListModel、不 clear；保留仍可见 node 的 entry identity 与未修改内容版本。
fn reconcile_list(
    list: &mut WidgetryListModel<WidgetryTreeVisibleItem>,
    items: &[WidgetryTreeVisibleItem],
) -> Result<(), BevyError> {
    let desired: HashSet<_> = items.iter().map(|item| item.entity).collect();
    for index in (0..list.len()).rev() {
        if list
            .get(index)
            .is_some_and(|item| !desired.contains(&item.entity))
        {
            invariant(list.remove(index))?;
        }
    }
    let ids: HashMap<_, _> = (0..list.len())
        .map(|index| {
            Ok((
                invariant(list.get(index))?.entity,
                invariant(list.id(index))?,
            ))
        })
        .collect::<Result<_, BevyError>>()?;
    for (index, item) in items.iter().enumerate() {
        if list.get(index).is_none_or(|old| old.entity != item.entity) {
            if let Some(id) = ids.get(&item.entity) {
                let from = invariant(list.index_of(*id))?;
                if !list.move_item(from, index) {
                    invariant::<()>(None)?;
                }
            } else {
                list.insert(index, *item)?;
            }
        }
        if list.get(index) != Some(item) {
            *invariant(list.get_mut(index)?)? = *item;
        }
    }
    Ok(())
}

/// 必需内部 source contract 不允许降级为静默失效。
fn invariant<T>(value: Option<T>) -> Result<T, BevyError> {
    value.ok_or_else(|| BevyError::error("WidgetryTree source invariant failed"))
}

/// 计算后分离 World borrow，统一更新 Tree projection 与 ListModel。
fn sync_source(world: &mut World, source: Entity) -> Result<(), BevyError> {
    let result = sync_source_inner(world, source);
    if let Some(mut tree) = world.get_mut::<WidgetryTreeModel>(source) {
        tree.diagnostics.observe(
            result,
            |error| widgetry_error!(?source, %error, "Tree source projection 同步失败"),
            || widgetry_info!(?source, "Tree source projection 恢复正常"),
        )
    } else {
        result
    }
}

fn sync_source_inner(world: &mut World, source: Entity) -> Result<(), BevyError> {
    let Some(tree) = world.get::<WidgetryTreeModel>(source) else {
        return Ok(());
    };
    invariant(world.get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source))?;
    let (items, reachable) = tree.projection(world)?;
    let list = invariant(world.get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source))?;
    let unchanged = list.len() == items.len()
        && items
            .iter()
            .enumerate()
            .all(|(index, item)| list.get(index) == Some(item));
    if !unchanged {
        let mut list =
            invariant(world.get_mut::<WidgetryListModel<WidgetryTreeVisibleItem>>(source))?;
        reconcile_list(&mut list, &items)?;
    }
    invariant(world.get_mut::<WidgetryTreeModel>(source))?.apply_projection(items, &reachable);
    Ok(())
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

/// 程序更新按实时 hierarchy 校验，不将无效 source/node 混同合法 no-op。
fn validate_target(world: &World, source: Entity, node: Option<Entity>) -> Result<(), BevyError> {
    if world.get::<WidgetryTreeModel>(source).is_none()
        || node.is_some_and(|node| !contains_node(world, source, node))
    {
        widgetry_error!(?source, ?node, "Tree 更新目标缺少 Model 或 node 不可达");
        return Err(BevyError::error(
            "Tree requires a live Model source and reachable node",
        ));
    }
    Ok(())
}

impl WidgetryTreeChildrenState {
    /// 同步完成 node 的 Loading 请求，不新增完成 event；重复 Loaded 返回 false。
    /// node 必须仍持有 WidgetryTreeNode 与 Loading/Loaded；失效或未请求时 ERROR 后返回 Err。
    /// 不要求 node 可见或展开；业务 children 由调用方维护，下次 projection 消费真实 hierarchy。
    pub fn set_loaded(world: &mut World, node: Entity) -> Result<bool, BevyError> {
        let state = world.get::<Self>(node).copied();
        if world.get::<WidgetryTreeNode>(node).is_none()
            || !matches!(state, Some(Self::Loading | Self::Loaded))
        {
            widgetry_error!(?node, ?state, "Tree loader 完成目标失效或尚未请求");
            return Err(BevyError::error(
                "Tree loader requires a live requested node",
            ));
        }
        if state == Some(Self::Loaded) {
            return Ok(false);
        }
        world.entity_mut(node).insert(Self::Loaded);
        Ok(true)
    }
}

impl WidgetryTreeModel {
    /// 同步展开 source 内的 node；重复或无 children 的普通 leaf 为合法 no-op。
    /// 先同步 projection 再发送 Expanded；lazy Unknown 先进入 Loading，再发 Expanded/ChildrenRequested。
    /// source/node 失效时 ERROR 后返回 Err；同步失败回滚展开 state，不发通知或留下 Loading。
    pub fn expand(world: &mut World, source: Entity, node: Entity) -> Result<bool, BevyError> {
        validate_target(world, source, Some(node))?;
        let previous = world.get::<Self>(source).map(|tree| {
            (
                tree.state.clone(),
                tree.visible_items.clone(),
                tree.entity_to_index.clone(),
            )
        });
        let lazy = world.get::<WidgetryTreeChildrenState>(node).copied();
        let has_children = world.get::<Children>(node).is_some_and(|children| {
            children
                .iter()
                .any(|child| world.get::<WidgetryTreeNode>(child).is_some())
        });
        if !has_children && lazy.is_none_or(|state| state == WidgetryTreeChildrenState::Loaded) {
            return Ok(false);
        }
        if !invariant(world.get_mut::<Self>(source))?
            .state
            .expanded
            .insert(node)
        {
            return Ok(false);
        }
        let request = lazy == Some(WidgetryTreeChildrenState::Unknown);
        if let Err(error) = sync_source(world, source) {
            if let Some((state, items, indices)) = previous
                && let Some(mut tree) = world.get_mut::<Self>(source)
            {
                tree.state = state;
                tree.visible_items = items;
                tree.entity_to_index = indices;
            }
            return Err(error);
        }
        if request {
            world
                .entity_mut(node)
                .insert(WidgetryTreeChildrenState::Loading);
        }

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
        Ok(true)
    }

    /// 同步收起 node，保留 descendant 展开意图与隐藏 selection；重复为合法 no-op。
    /// 先同步 projection 再发 Collapsed；source/node 失效时 ERROR 后返回 Err。
    pub fn collapse(world: &mut World, source: Entity, node: Entity) -> Result<bool, BevyError> {
        validate_target(world, source, Some(node))?;
        let previous = world.get::<Self>(source).map(|tree| {
            (
                tree.state.clone(),
                tree.visible_items.clone(),
                tree.entity_to_index.clone(),
            )
        });
        if !invariant(world.get_mut::<Self>(source))?
            .state
            .expanded
            .remove(&node)
        {
            return Ok(false);
        }
        if let Err(error) = sync_source(world, source) {
            if let Some((state, items, indices)) = previous
                && let Some(mut tree) = world.get_mut::<Self>(source)
            {
                tree.state = state;
                tree.visible_items = items;
                tree.entity_to_index = indices;
            }
            return Err(error);
        }

        world.trigger(WidgetryTreeEvent {
            entity: source,
            kind: WidgetryTreeEventKind::Collapsed(node),
        });
        Ok(true)
    }

    /// 按当前展开意图同步执行 expand/collapse，沿用其通知、no-op 与错误契约。
    pub fn toggle_expand(
        world: &mut World,
        source: Entity,
        node: Entity,
    ) -> Result<bool, BevyError> {
        if world
            .get::<Self>(source)
            .is_some_and(|tree| tree.state.is_expanded(node))
        {
            Self::collapse(world, source, node)
        } else {
            Self::expand(world, source, node)
        }
    }

    /// 同步提交 Entity selection 后发 Selected，允许隐藏但可达的 node；None 显式清空。
    /// 合法同值返回 false 且不通知；source/node 失效时 ERROR 后返回 Err，保留旧 state。
    /// 不受 view disabled 限制，不展开祖先、不修改 focus 或主动 reveal；视觉 projection 后续同步。
    pub fn select(
        world: &mut World,
        source: Entity,
        node: Option<Entity>,
    ) -> Result<bool, BevyError> {
        validate_target(world, source, node)?;
        let mut tree = invariant(world.get_mut::<Self>(source))?;
        if tree.state.selected == node {
            return Ok(false);
        }
        tree.state.selected = node;
        world.trigger(WidgetryTreeEvent {
            entity: source,
            kind: WidgetryTreeEventKind::Selected(node),
        });
        Ok(true)
    }
}
