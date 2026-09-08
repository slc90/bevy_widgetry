use crate::{
    WindowRoot, title_bar::controls::window_control_button_node, window_root::find_window_root,
};
use bevy::{
    asset::AssetServer,
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
        observer::On,
        query::With,
        system::{NonSendMarker, Query, Res},
    },
    picking::hover::Hovered,
    ui::{BackgroundColor, Node},
    ui_widgets::{Activate, Button},
    window::Window,
    winit::WINIT_WINDOWS,
};
use bevy_widgetry_core::icon::Icon;

#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub struct MaximizeButton;

pub(super) fn on_maximize_restore(
    event: On<Activate>,
    _non_send_marker: NonSendMarker,
    buttons: Query<(), With<MaximizeButton>>,
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

    let actual_maximized = WINIT_WINDOWS.with_borrow(|winit_windows| {
        winit_windows
            .get_window(root.target_window)
            .map(|window| window.is_maximized())
    });

    let Some(actual_maximized) = actual_maximized else {
        return;
    };

    let Ok(mut window) = windows.get_mut(root.target_window) else {
        return;
    };

    window.set_maximized(!actual_maximized);
}

pub(super) fn sync_maximize_state(
    _non_send_marker: NonSendMarker,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    children: Query<&Children>,
    buttons: Query<Entity, With<MaximizeButton>>,
    mut icons: Query<&mut Icon>,
    asset_server: Res<AssetServer>,
) {
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        for button_entity in &buttons {
            let Some(root) = find_window_root(button_entity, &parents, &roots) else {
                continue;
            };

            let Some(winit_window) = winit_windows.get_window(root.target_window) else {
                continue;
            };

            let maximized = winit_window.is_maximized();

            let Ok(button_children) = children.get(button_entity) else {
                continue;
            };

            for &child in button_children.iter() {
                let Ok(mut icon) = icons.get_mut(child) else {
                    continue;
                };

                if maximized {
                    icon.set_svg(
                        &asset_server,
                        "embedded://bevy_widgetry_window/icons/restore.svg",
                    );
                } else {
                    icon.set_svg(
                        &asset_server,
                        "embedded://bevy_widgetry_window/icons/maximize.svg",
                    );
                }

                break;
            }
        }
    });
}
