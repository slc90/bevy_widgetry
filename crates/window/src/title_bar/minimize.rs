use crate::{
    WindowRoot, title_bar::controls::window_control_button_node, window_root::find_window_root,
};
use bevy::{
    ecs::{component::Component, hierarchy::ChildOf, observer::On, query::With, system::Query},
    picking::hover::Hovered,
    ui::{BackgroundColor, Node},
    ui_widgets::{Activate, Button},
    window::Window,
};

#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub struct MinimizeButton;

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

    window.set_minimized(true);
}
