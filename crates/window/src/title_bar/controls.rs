use crate::{
    title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton},
    window_root::{WindowRoot, find_window_root},
};
use bevy::prelude::{ChildOf, Commands, Entity, Window};
use bevy::ui::InteractionDisabled;
use bevy::{
    color::Color,
    ecs::{
        lifecycle::RemovedComponents,
        query::{Added, Changed, Has, Or, With},
        system::Query,
    },
    picking::hover::Hovered,
    ui::{AlignItems, BackgroundColor, BorderRadius, JustifyContent, Node, Pressed, percent, px},
    utils::default,
};

type ChangedControlStyleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Hovered, Has<Pressed>, &'static mut BackgroundColor),
    (
        Or<(With<MinimizeButton>, With<MaximizeButton>)>,
        Or<(Changed<Hovered>, Added<Pressed>)>,
    ),
>;

type ControlStyleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Hovered, Has<Pressed>, &'static mut BackgroundColor),
    Or<(With<MinimizeButton>, With<MaximizeButton>)>,
>;

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

fn window_control_background(hovered: bool, pressed: bool) -> Color {
    if pressed {
        Color::srgba(1.0, 1.0, 1.0, 0.14)
    } else if hovered {
        Color::srgba(1.0, 1.0, 1.0, 0.08)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.0)
    }
}

pub(super) fn update_window_control_style_changed(mut query: ChangedControlStyleQuery<'_, '_>) {
    for (hovered, pressed, mut background) in &mut query {
        background.0 = window_control_background(hovered.0, pressed);
    }
}

pub(super) fn update_window_control_style_released(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut query: ControlStyleQuery<'_, '_>,
) {
    for entity in removed_pressed.read() {
        let Ok((hovered, pressed, mut background)) = query.get_mut(entity) else {
            continue;
        };

        background.0 = window_control_background(hovered.0, pressed);
    }
}

pub(super) fn sync_enabled_buttons(
    buttons: Query<
        (
            Entity,
            Has<MinimizeButton>,
            Has<MaximizeButton>,
            Has<InteractionDisabled>,
        ),
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
    for (entity, minimize, maximize, disabled) in &buttons {
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
        if enabled && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        } else if !enabled && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        }
    }
}
