use crate::{
    title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton},
    window_root::{WindowRoot, find_window_root},
};
use bevy::prelude::{ChildOf, Commands, Entity, Window};
use bevy::{
    ecs::{
        query::{Has, Or, With},
        system::Query,
    },
    ui::{AlignItems, BorderRadius, JustifyContent, Node, percent, px},
    utils::default,
};
use bevy_widgetry_core::disabled::queue_intrinsic_disabled;

pub(super) fn window_control_button_node() -> Node {
    Node {
        width: px(46),
        height: percent(100),
        border_radius: BorderRadius::all(px(4)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

pub(super) fn sync_enabled_buttons(
    buttons: Query<
        (Entity, Has<MinimizeButton>, Has<MaximizeButton>),
        Or<(
            With<MinimizeButton>,
            With<MaximizeButton>,
            With<CloseButton>,
        )>,
    >,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    for (entity, minimize, maximize) in &buttons {
        let Some(root) = find_window_root(entity, &parents, &roots) else {
            continue;
        };
        let Ok(window) = windows.get(root.target_window) else {
            continue;
        };
        let enabled = if minimize {
            window.enabled_buttons.minimize
        } else if maximize {
            window.enabled_buttons.maximize
        } else {
            window.enabled_buttons.close
        };
        queue_intrinsic_disabled(&mut commands, entity, !enabled);
    }
}
