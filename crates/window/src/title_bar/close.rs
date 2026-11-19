use crate::{
    WidgetryWindowAction,
    title_bar::controls::window_control_button_node,
    window_root::{WindowRoot, find_window_root},
};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::ChildOf,
        observer::On,
        query::With,
        system::{Commands, Query},
    },
    picking::hover::Hovered,
    ui::{BackgroundColor, Node},
    ui_widgets::{Activate, Button},
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
    roots: Query<(Entity, &WindowRoot)>,
    mut commands: Commands,
) {
    let Ok(()) = buttons.get(event.entity) else {
        return;
    };

    let Some((root, _)) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    WidgetryWindowAction::RequestClose.queue(&mut commands, root);
}
