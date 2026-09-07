use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResolution},
};
use bevy_widgetry::window::{
    CloseButton, MaximizeButton, MinimizeButton, TitleBar, TitleBarPlugin, WindowControls,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Widget Gallery".into(),
                resolution: WindowResolution::new(1200, 800),
                decorations: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TitleBarPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, primary_window: Query<Entity, With<PrimaryWindow>>) {
    let window = primary_window.single().unwrap();
    commands.spawn(Camera2d);

    commands
        .spawn((
            TitleBar {
                target_window: window,
            },
            Node {
                width: percent(100),
                height: px(36),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
        ))
        .with_children(|title_bar| {
            title_bar.spawn(WindowControls).with_children(|controls| {
                controls.spawn(MinimizeButton).with_child(Text::new("-"));
                controls
                    .spawn(MaximizeButton::default())
                    .with_child(Text::new("[]"));
                controls.spawn(CloseButton).with_child(Text::new("x"));
            });
        });
}
