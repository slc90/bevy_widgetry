#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use bevy::{
    camera::{RenderTarget, Viewport},
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};
use bevy_brp_runtime::BrpRuntimePlugin;
use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
use bevy_widgetry_ui_scale::{WidgetryUiScale, WidgetryUiScalePlugin, WindowScaleExcluded};

const DESIGN: UVec2 = UVec2::new(640, 400);
const PANEL_SIZE: Vec2 = Vec2::new(400.0, 240.0);
const PANEL_MARGIN: f32 = 32.0;
const TILE_SIZE: f32 = 120.0;
const TILE_MARGIN: f32 = 24.0;
const PANEL_COLOR: Color = Color::srgb(0.2, 0.45, 0.75);
const TILE_COLOR: Color = Color::srgb(0.9, 0.6, 0.2);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Global scale: keys 1 / 2 / 3".into(),
                resolution: DESIGN.into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((WidgetryUiScalePlugin, BrpRuntimePlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, set_scale)
        .run();
}

fn setup(mut commands: Commands, primary: Single<(Entity, &Window), With<PrimaryWindow>>) {
    let (main, window) = *primary;
    commands
        .entity(main)
        .insert(Name::new("Scale Example Main"));
    view(&mut commands, main, Some(window.physical_size()));
    let ordinary = commands
        .spawn((
            Name::new("Scale Example Popup"),
            Window {
                title: "Managed popup".into(),
                resolution: DESIGN.into(),
                ..default()
            },
        ))
        .id();
    view(&mut commands, ordinary, None);
    let excluded = commands
        .spawn((
            Name::new("Scale Example Excluded"),
            Window {
                title: "Excluded window".into(),
                resolution: DESIGN.into(),
                ..default()
            },
            WindowScaleExcluded,
        ))
        .id();
    view(&mut commands, excluded, None);
}

fn view(commands: &mut Commands, window: Entity, viewport: Option<UVec2>) {
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                viewport: viewport.map(|physical_size| Viewport {
                    physical_size,
                    ..default()
                }),
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window)),
        ))
        .id();
    commands.spawn_scene_with_error_handler(bsn! {
        Node { width: px(PANEL_SIZE.x), height: px(PANEL_SIZE.y), margin: UiRect::all(px(PANEL_MARGIN)) }
        UiTargetCamera(camera)
        BackgroundColor(PANEL_COLOR)
        Children [
            Node { width: px(TILE_SIZE), height: px(TILE_SIZE), margin: UiRect::all(px(TILE_MARGIN)) }
            BackgroundColor(TILE_COLOR)
        ]
    });
}

fn set_scale(keys: Res<ButtonInput<KeyCode>>, mut scale: ResMut<WidgetryUiScale>) -> Result {
    let choices = [
        (KeyCode::Digit1, 1.0),
        (KeyCode::Digit2, 1.25),
        (KeyCode::Digit3, 2.0),
    ];
    for (key, value) in choices {
        if keys.just_pressed(key) {
            scale
                .set(value)
                .map_err(|failure| failure.into_bevy_error())?;
        }
    }
    Ok(())
}
