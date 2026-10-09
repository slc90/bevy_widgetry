use crate::{
    title_bar::controls::window_control_button_node,
    window_root::{WindowRoot, find_window_root},
};
use bevy::{
    ecs::{
        component::Component, hierarchy::ChildOf, message::MessageWriter, observer::On,
        query::With, system::Query,
    },
    picking::hover::Hovered,
    ui::{BackgroundColor, Node},
    ui_widgets::{Activate, Button},
    window::{Window, WindowCloseRequested},
};

#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub(crate) struct CloseButton;

pub(super) fn on_close(
    event: On<Activate>,
    buttons: Query<(), With<CloseButton>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    windows: Query<&Window>,
    mut close_requests: MessageWriter<WindowCloseRequested>,
) {
    let Ok(()) = buttons.get(event.entity) else {
        return;
    };

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    if !windows
        .get(root.target_window)
        .is_ok_and(|window| window.enabled_buttons.close)
    {
        return;
    }

    close_requests.write(WindowCloseRequested {
        window: root.target_window,
    });
}
