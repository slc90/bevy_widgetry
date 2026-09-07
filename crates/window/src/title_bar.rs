use bevy::{
    prelude::*,
    ui_widgets::{Activate, Button},
    window::WindowCloseRequested,
};

#[derive(Component)]
pub struct TitleBar {
    pub target_window: Entity,
}

#[derive(Component)]
#[require(
    Node = window_controls_node(),
)]
pub struct WindowControls;

fn window_controls_node() -> Node {
    Node {
        height: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Stretch,
        ..default()
    }
}

#[derive(Component)]
#[require(
    Button,
    Node = window_control_button_node(),
)]
pub struct MinimizeButton;

#[derive(Component)]
#[require(
    Button,
    Node = window_control_button_node(),
)]
pub struct MaximizeButton {
    maximized: bool,
}

impl Default for MaximizeButton {
    fn default() -> Self {
        Self { maximized: false }
    }
}

#[derive(Component)]
#[require(
    Button,
    Node = window_control_button_node(),
)]
pub struct CloseButton;

fn window_control_button_node() -> Node {
    Node {
        width: px(46),
        height: percent(100),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

pub struct TitleBarPlugin;

impl Plugin for TitleBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_minimize)
            .add_observer(on_maximize_restore)
            .add_observer(on_close);
    }
}

fn on_minimize(
    event: On<Activate>,
    buttons: Query<&ChildOf, With<MinimizeButton>>,
    parents: Query<&ChildOf>,
    title_bars: Query<&TitleBar>,
    mut windows: Query<&mut Window>,
) {
    let Ok(controls_parent) = buttons.get(event.entity) else {
        return;
    };

    let Ok(title_bar_parent) = parents.get(controls_parent.parent()) else {
        return;
    };

    let Ok(title_bar) = title_bars.get(title_bar_parent.parent()) else {
        return;
    };

    let Ok(mut window) = windows.get_mut(title_bar.target_window) else {
        return;
    };

    window.set_minimized(true);
}

fn on_maximize_restore(
    event: On<Activate>,
    parents: Query<&ChildOf>,
    title_bars: Query<&TitleBar>,
    mut buttons: Query<&mut MaximizeButton>,
    mut windows: Query<&mut Window>,
) {
    let Ok(mut button) = buttons.get_mut(event.entity) else {
        return;
    };

    let Ok(controls_parent) = parents.get(event.entity) else {
        return;
    };

    let Ok(title_bar_parent) = parents.get(controls_parent.parent()) else {
        return;
    };

    let Ok(title_bar) = title_bars.get(title_bar_parent.parent()) else {
        return;
    };

    let Ok(mut window) = windows.get_mut(title_bar.target_window) else {
        return;
    };

    button.maximized = !button.maximized;

    window.set_maximized(button.maximized);
}

fn on_close(
    event: On<Activate>,
    buttons: Query<&ChildOf, With<CloseButton>>,
    parents: Query<&ChildOf>,
    title_bars: Query<&TitleBar>,
    mut close_requests: MessageWriter<WindowCloseRequested>,
) {
    let Ok(controls_parent) = buttons.get(event.entity) else {
        return;
    };

    let Ok(title_bar_parent) = parents.get(controls_parent.parent()) else {
        return;
    };

    let Ok(title_bar) = title_bars.get(title_bar_parent.parent()) else {
        return;
    };

    close_requests.write(WindowCloseRequested {
        window: title_bar.target_window,
    });
}
