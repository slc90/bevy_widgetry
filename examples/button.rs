use bevy::color::palettes::basic::RED;
use bevy::prelude::*;
use bevy_widgetry::{MiniActivate, MiniButton, MiniButtonPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MiniButtonPlugin)
        .add_systems(Startup, start_up)
        .run();
}

fn start_up(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            MiniButton,
            Node {
                left: Val::Px(100.0),
                top: Val::Px(100.0),
                width: Val::Px(200.0),
                height: Val::Px(100.0),
                ..Default::default()
            },
            BackgroundColor(RED.into()),
        ))
        .observe(|mini_activate: On<MiniActivate>| {
            info!("mini_activate: {:?}", mini_activate.entity);
        });
}
