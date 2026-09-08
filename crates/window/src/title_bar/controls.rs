use crate::title_bar::{maximize::MaximizeButton, minimize::MinimizeButton};
use bevy::{
    color::Color,
    ecs::{
        lifecycle::RemovedComponents,
        query::{Added, Changed, Has, Or, With},
        system::Query,
    },
    picking::hover::Hovered,
    ui::{AlignItems, BackgroundColor, JustifyContent, Node, Pressed, percent, px},
    utils::default,
};

pub(super) fn window_control_button_node() -> Node {
    Node {
        width: px(46),
        height: percent(100),
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

pub(super) fn update_window_control_style_changed(
    mut query: Query<
        (&Hovered, Has<Pressed>, &mut BackgroundColor),
        (
            Or<(With<MinimizeButton>, With<MaximizeButton>)>,
            Or<(Changed<Hovered>, Added<Pressed>)>,
        ),
    >,
) {
    for (hovered, pressed, mut background) in &mut query {
        background.0 = window_control_background(hovered.0, pressed);
    }
}

pub(super) fn update_window_control_style_released(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut query: Query<
        (&Hovered, Has<Pressed>, &mut BackgroundColor),
        Or<(With<MinimizeButton>, With<MaximizeButton>)>,
    >,
) {
    for entity in removed_pressed.read() {
        let Ok((hovered, pressed, mut background)) = query.get_mut(entity) else {
            continue;
        };

        background.0 = window_control_background(hovered.0, pressed);
    }
}
