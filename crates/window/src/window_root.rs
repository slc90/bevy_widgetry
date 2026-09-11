use bevy::prelude::*;

/// 关联 UI 层级与真实窗口，使子节点上的交互作用于正确的窗口。
#[derive(Component)]
#[require(Node = window_root_node())]
pub(crate) struct WindowRoot {
    /// 需要接收该 UI 层级窗口操作的真实窗口实体。
    pub target_window: Entity,
}

/// 窗口标题栏之外的内容容器，占据根布局的剩余空间。
#[derive(Component)]
#[require(Node = window_content_node(), Pickable::IGNORE)]
pub(crate) struct WindowContent;

/// 使窗口 UI 根填满可用空间，并按纵向排列标题栏和内容。
fn window_root_node() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// 让应用内容占据标题栏之外的剩余区域。
fn window_content_node() -> Node {
    Node {
        width: percent(100),
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// 从交互实体或其祖先查找窗口关联，非窗口层级返回 None。
pub(crate) fn find_window_root<'a>(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &'a Query<&WindowRoot>,
) -> Option<&'a WindowRoot> {
    parents
        .iter_ancestors(entity)
        .find_map(|ancestor| roots.get(ancestor).ok())
}
