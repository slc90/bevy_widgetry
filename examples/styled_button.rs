use bevy::{
    DefaultPlugins,
    app::{App, Startup},
    camera::Camera2d,
    ecs::system::Commands,
    ui::{InteractionDisabled, Node, PositionType, UiRect, px, widget::Text},
    utils::default,
};
use bevy_widgetry::styled_button::{StyledButton, StyledButtonPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(StyledButtonPlugin)
        .add_systems(Startup, start_up)
        .run();
}

fn start_up(mut commands: Commands) {
    commands.spawn(Camera2d);

    // 普通按钮
    commands
        .spawn((
            StyledButton,
            Node {
                width: px(160),
                height: px(48),
                position_type: PositionType::Absolute,
                left: px(40),
                top: px(40),
                padding: UiRect::axes(px(12), px(6)),
                border: UiRect::all(px(1)),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(Text::new("Button"));
        });

    // Disabled 按钮
    commands
        .spawn((
            StyledButton,
            InteractionDisabled,
            Node {
                width: px(160),
                height: px(48),
                position_type: PositionType::Absolute,
                left: px(40),
                top: px(110),
                padding: UiRect::axes(px(12), px(6)),
                border: UiRect::all(px(1)),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(Text::new("Disabled"));
        });
}
