use super::bar::WindowControls;
use crate::{
    WidgetryWindowMaximized,
    action::queue_toggle_maximize,
    title_bar::controls::window_control_button_node,
    window_root::{WindowRoot, find_window_root},
};
use bevy::prelude::{DetectChanges, Ref};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
        observer::On,
        query::With,
        system::{Commands, NonSendMarker, Query},
    },
    picking::hover::Hovered,
    ui::{BackgroundColor, Node},
    ui_widgets::{Activate, Button},
};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;

#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub(crate) struct MaximizeButton;

pub(super) fn on_maximize_restore(
    event: On<Activate>,
    _non_send_marker: NonSendMarker,
    buttons: Query<(), With<MaximizeButton>>,
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

    queue_toggle_maximize(&mut commands, root);
}

pub(super) fn sync_default_maximize_icon(
    buttons: Query<(Entity, Ref<ChildOf>, Ref<Children>), With<MaximizeButton>>,
    controls: Query<(), With<WindowControls>>,
    parents: Query<&ChildOf>,
    roots: Query<(Entity, &WindowRoot)>,
    states: Query<Ref<WidgetryWindowMaximized>>,
    icons: Query<(), With<WidgetryIcon>>,
    mut commands: Commands,
) {
    for (button, parent, children) in &buttons {
        if !controls.contains(parent.parent()) {
            continue;
        }
        let Some((root, _)) = find_window_root(button, &parents, &roots) else {
            continue;
        };
        let Ok(state) = states.get(root) else {
            continue;
        };
        if !state.is_changed() && !parent.is_changed() && !children.is_changed() {
            continue;
        }
        let icon = if state.is_maximized() {
            BuiltinIcon::WindowRestore
        } else {
            BuiltinIcon::WindowMaximize
        };
        for &child in children.iter() {
            if icons.contains(child) {
                WidgetryIcon::set_svg(&mut commands, child, icon.path());
            }
        }
    }
}
