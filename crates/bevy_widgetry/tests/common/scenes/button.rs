use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed},
};
use bevy_widgetry::button::{StyledButton, StyledButtonPlugin};

pub const WIDTH: u32 = 256;

pub const HEIGHT: u32 = 256;

pub fn setup(app: &mut App) {
    app.add_plugins(StyledButtonPlugin);
}

pub fn spawn(app: &mut App, camera: Entity) {
    app.world_mut()
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(8),
                ..default()
            },
            UiTargetCamera(camera),
        ))
        .with_children(|parent| {
            // Default
            parent.spawn((
                StyledButton,
                Node {
                    width: px(120),
                    height: px(40),
                    ..default()
                },
            ));

            // Hover
            parent.spawn((
                StyledButton,
                Hovered(true),
                Node {
                    width: px(120),
                    height: px(40),
                    ..default()
                },
            ));

            // Pressed
            parent.spawn((
                StyledButton,
                Hovered(true),
                Pressed,
                Node {
                    width: px(120),
                    height: px(40),
                    ..default()
                },
            ));

            // Disabled
            parent.spawn((
                StyledButton,
                InteractionDisabled,
                Node {
                    width: px(120),
                    height: px(40),
                    ..default()
                },
            ));
        });
}
