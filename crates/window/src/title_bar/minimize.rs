use crate::{
    title_bar::controls::window_control_button_node,
    window_root::{WindowRoot, find_window_root},
};
use bevy::{
    ecs::{component::Component, hierarchy::ChildOf, observer::On, query::With, system::Query},
    picking::hover::Hovered,
    ui::{BackgroundColor, Node},
    ui_widgets::{Activate, Button},
    window::Window,
};

/// 将激活操作转发到关联真实窗口的最小化状态。
#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub(super) struct MinimizeButton;

/// 对所属真实窗口执行最小化，忽略其他实体的激活事件。
pub(super) fn on_minimize(
    event: On<Activate>,
    buttons: Query<(), With<MinimizeButton>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut windows: Query<&mut Window>,
) {
    let Ok(()) = buttons.get(event.entity) else {
        return;
    };

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    let Ok(mut window) = windows.get_mut(root.target_window) else {
        return;
    };

    if !window.enabled_buttons.minimize {
        return;
    }

    window.set_minimized(true);
}
