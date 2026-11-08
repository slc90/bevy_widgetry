#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use bevy::{
    camera::{ClearColorConfig, RenderTarget},
    log::LogPlugin,
    prelude::*,
    render::RenderPlugin,
    tasks::block_on,
    window::{PrimaryWindow, WindowRef},
    winit::WinitSettings,
};
use bevy_brp_runtime::BrpRuntimePlugin;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_splash::{
    Splash, SplashError, SplashFailure, SplashPlugin, SplashReady, SplashShown,
};
use bevy_widgetry_ui_scale::{WidgetryUiScale, WidgetryUiScalePlugin};
use bevy_widgetry_window::transparent_render_creation;

const MAIN_SIZE: UVec2 = UVec2::new(640, 400);
const SPLASH_SIZE: UVec2 = UVec2::new(480, 270);
const MAIN_COLOR: Color = Color::srgb(0.12, 0.25, 0.4);
const SPLASH_COLOR: Color = Color::srgb(0.8, 0.35, 0.1);

#[derive(Resource)]
struct DemoSplash(Entity);

fn main() -> Result {
    let hidden_main = std::env::args().any(|argument| argument == "--hidden-main");
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
                        title: "Splash host: S declares content ready, 1/2/3 scale".into(),
                        resolution: MAIN_SIZE.into(),
                        visible: !hidden_main,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(WinitSettings::desktop_app())
        .add_plugins((SplashPlugin, BrpRuntimePlugin::default()))
        .add_observer(|event: On<SplashShown>, world: &World| {
            info!(splash = ?event.splash, main = ?event.main,
                phase = ?Splash::phase(world, event.splash), "Splash 首帧已显示");
        })
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
        Name::new("Splash Demo Main Camera"),
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
    mut commands: Commands,
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
    if keys.just_pressed(KeyCode::KeyS) {
        info!(splash = ?splash.0, "声明 Splash 内容已就绪");
        commands.trigger(SplashReady { splash: splash.0 });
    }
    Ok(())
}
