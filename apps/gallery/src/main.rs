use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResolution},
};
use bevy_widgetry::window::{TitleBar, TitleBarPlugin, WindowResizeArea};

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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    let window = primary_window.single().unwrap();
    commands.spawn(Camera2d);
    WindowResizeArea::spawn(&mut commands, window);
    TitleBar::spawn(&mut commands, &asset_server, window, |content| {
        content.spawn(Text::new("Widget Gallery"));
    });
}
