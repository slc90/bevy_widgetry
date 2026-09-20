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

/// 将 Activate 转发到关联真实 window 的 minimized state。
#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub(super) struct MinimizeButton;

/// 对所属真实 window 执行 minimize，忽略其他 entity 的 Activate event。
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
