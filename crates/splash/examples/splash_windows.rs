#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use bevy::{
    camera::{ClearColorConfig, RenderTarget},
    log::LogPlugin,
    prelude::*,
    render::RenderPlugin,
    tasks::block_on,
    window::{PrimaryWindow, WindowRef},
};
use bevy_brp_runtime::BrpRuntimePlugin;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_splash::{Splash, SplashError, SplashFailure, SplashPlugin, SplashWindow};
use bevy_widgetry_ui_scale::{WidgetryUiScale, WidgetryUiScalePlugin};
use bevy_widgetry_window::{WidgetryWindowPlugin, transparent_render_creation};

const MAIN_SIZE: UVec2 = UVec2::new(640, 400);
const SPLASH_SIZE: UVec2 = UVec2::new(480, 270);
const MAIN_COLOR: Color = Color::srgb(0.12, 0.25, 0.4);
const SPLASH_COLOR: Color = Color::srgb(0.8, 0.35, 0.1);

#[derive(Resource)]
struct DemoSplash(Entity);

fn main() -> Result {
    let mut app = App::new();
    app.add_plugins(LogPlugin::default());
    let render_creation =
        block_on(transparent_render_creation()).map_err(|failure| failure.into_bevy_error())?;
    app.insert_resource(WidgetryUiScale::new(1.25).map_err(|failure| failure.into_bevy_error())?)
        .add_plugins(WidgetryUiScalePlugin)
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .set(RenderPlugin {
                    render_creation,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Splash host: S shows content, 1/2/3 scale".into(),
                        resolution: MAIN_SIZE.into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins((
            WidgetryWindowPlugin,
            SplashPlugin,
            BrpRuntimePlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, input);
    app.run();
    Ok(())
}

fn setup(world: &mut World) -> Result {
    let main = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .ok_or_else(|| {
            SplashFailure::new(
                SplashError::MainWindowMissing {
                    main: Entity::PLACEHOLDER,
                },
                ErrorFrame::new("splash.example.main"),
            )
            .into_bevy_error()
        })?;
    world.entity_mut(main).insert(Name::new("Splash Demo Main"));
    world.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(MAIN_COLOR),
            ..default()
        },
    ));
    let splash = Splash::start(
        world,
        main,
        Window {
            title: "Fixed physical splash".into(),
            resolution: SPLASH_SIZE.into(),
            ..default()
        },
    )
    .map_err(|failure| failure.into_bevy_error())?;
    world
        .entity_mut(splash)
        .insert(Name::new("Splash Demo Window"));
    world.spawn((
        Name::new("Splash Demo Camera"),
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(SPLASH_COLOR),
            ..default()
        },
        RenderTarget::Window(WindowRef::Entity(splash)),
    ));
    world.insert_resource(DemoSplash(splash));
    Ok(())
}

fn input(
    keys: Res<ButtonInput<KeyCode>>,
    splash: Res<DemoSplash>,
    mut scale: ResMut<WidgetryUiScale>,
    mut windows: Query<&mut Window>,
    states: Query<&SplashWindow>,
) -> Result {
    for (key, value) in [
        (KeyCode::Digit1, 1.0),
        (KeyCode::Digit2, 1.25),
        (KeyCode::Digit3, 2.0),
    ] {
        if keys.just_pressed(key) {
            scale
                .set(value)
                .map_err(|failure| failure.into_bevy_error())?;
        }
    }
    if keys.just_pressed(KeyCode::KeyS)
        && states.get(splash.0).is_ok_and(SplashWindow::is_positioned)
        && let Ok(mut window) = windows.get_mut(splash.0)
    {
        window.visible = true;
    }
    Ok(())
}
