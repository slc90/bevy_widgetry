use bevy::{
    DefaultPlugins,
    app::{App, Startup},
    camera::Camera2d,
    color::palettes::css::RED,
    ecs::{observer::On, system::Commands},
    log::info,
    ui::{BackgroundColor, Node, Val},
};
use bevy_widgetry::button::{LongPressButton, LongPressEvent, LongPressPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LongPressPlugin)
        .add_systems(Startup, start_up)
        .run();
}

fn start_up(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            LongPressButton::default(),
            Node {
                left: Val::Px(100.0),
                top: Val::Px(100.0),
                width: Val::Px(200.0),
                height: Val::Px(100.0),
                ..Default::default()
            },
            BackgroundColor(RED.into()),
        ))
        .observe(|long_press: On<LongPressEvent>| {
            info!("long_press: {:?}", long_press.entity);
        });
}
