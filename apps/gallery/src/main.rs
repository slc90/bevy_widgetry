use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, WgpuSettings},
    },
};
use bevy_widgetry::window::{
    TitleBar, TitleBarPlugin, WindowContent, WindowResizeArea, WindowRoot,
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Widget Gallery".into(),
                        resolution: WindowResolution::new(1200, 800),
                        decorations: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        // Windows上选择Vulkan时拉伸有黑色
                        backends: Some(Backends::DX12),
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        )
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
    commands
        .spawn(WindowRoot {
            target_window: window,
        })
        .with_children(|root| {
            TitleBar::spawn(root, &asset_server, |content| {
                content.spawn(Text::new("Widget Gallery"));
            });

            root.spawn(WindowContent).with_children(|_content| {
                // Gallery 正文以后放这里
            });

            WindowResizeArea::spawn(root);
        });
}
