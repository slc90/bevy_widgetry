use crate::{
    title_bar::controls::window_control_button_node,
    window_root::{WindowRoot, find_window_root},
};
use bevy::{
    color::Color,
    ecs::{
        component::Component,
        hierarchy::ChildOf,
        lifecycle::RemovedComponents,
        message::MessageWriter,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        system::Query,
    },
    picking::hover::Hovered,
    ui::{BackgroundColor, Node, Pressed},
    ui_widgets::{Activate, Button},
    window::WindowCloseRequested,
};

#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub struct CloseButton;

pub(super) fn on_close(
    event: On<Activate>,
    buttons: Query<(), With<CloseButton>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut close_requests: MessageWriter<WindowCloseRequested>,
) {
    let Ok(()) = buttons.get(event.entity) else {
        return;
    };

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    close_requests.write(WindowCloseRequested {
        window: root.target_window,
    });
}

fn close_button_background(hovered: bool, pressed: bool) -> Color {
    if pressed {
        Color::srgb_u8(180, 30, 30)
    } else if hovered {
        Color::srgb_u8(196, 43, 28)
    } else {
        Color::NONE
    }
}

pub(super) fn update_close_button_style_changed(
    mut query: Query<
        (&Hovered, Has<Pressed>, &mut BackgroundColor),
        (With<CloseButton>, Or<(Changed<Hovered>, Added<Pressed>)>),
    >,
) {
    for (hovered, pressed, mut background) in &mut query {
        background.0 = close_button_background(hovered.0, pressed);
    }
}

pub(super) fn update_close_button_style_released(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut query: Query<(&Hovered, Has<Pressed>, &mut BackgroundColor), With<CloseButton>>,
) {
    for entity in removed_pressed.read() {
        let Ok((hovered, pressed, mut background)) = query.get_mut(entity) else {
            continue;
        };

        background.0 = close_button_background(hovered.0, pressed);
    }
}
