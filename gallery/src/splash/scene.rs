use super::{SplashSession, presentation::SplashPresentation};
use bevy::{camera::RenderTarget, prelude::*, window::WindowRef};
use bevy_widgetry::{scene::spawn_scene, splash::Splash};

const SPLASH_PHYSICAL_SIZE: UVec2 = UVec2::new(1920, 1080);

pub(crate) fn setup(commands: &mut Commands, server: &AssetServer, main: Entity) {
    let server = server.clone();
    commands.queue(move |world: &mut World| -> Result {
        match build(world, &server, main) {
            Ok(()) => Ok(()),
            Err(error) => super::fail_startup(world, error),
        }
    });
}

fn build(world: &mut World, server: &AssetServer, main: Entity) -> Result {
    let window = Splash::start(
        world,
        main,
        Window {
            title: "Widget Gallery Splash".into(),
            resolution: SPLASH_PHYSICAL_SIZE.into(),
            skip_taskbar: true,
            ..default()
        },
    )
    .map_err(|error| error.into_bevy_error())?;
    world
        .entity_mut(window)
        .insert(Name::new("GallerySplashWindow"));
    let camera = world
        .spawn((
            Name::new("GallerySplashCamera"),
            Camera2d,
            Camera {
                clear_color: ClearColorConfig::Custom(canvas_color()),
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window)),
        ))
        .id();
    let loaded: Vec<Handle<Image>> = crate::assets::splash_frames()
        .map(|path| server.load(path))
        .collect();
    let first = loaded[0].clone();
    let overlay = match spawn_scene(
        world,
        bsn! {
            Name("SplashDisplay")
            UiTargetCamera(camera)
            @full_window()
            template(move |_| Ok(ImageNode { image: first.clone(), image_mode: NodeImageMode::Stretch, ..default() }))
            Children [@{super::skip::scene()}]
        },
    ) {
        Ok(root) => root,
        Err(error) => {
            world.despawn(camera);
            let error = error.into_bevy_error();
            return Err(match Splash::cancel(world, window) {
                Ok(()) => error,
                Err(cleanup) => crate::diagnostics::combine(error, cleanup.into_bevy_error()),
            });
        }
    };
    world.insert_resource(SplashPresentation::new(
        window,
        camera,
        overlay,
        loaded.iter().map(Handle::id).collect(),
    ));
    world.insert_resource(SplashSession {
        main,
        window,
        camera,
        overlay,
        loaded,
        finish_requested: false,
    });
    debug!(
        ?window,
        ?main,
        ?camera,
        "Gallery 已创建独立 Splash 会话，等待应用内容和首帧"
    );
    Ok(())
}

fn full_window() -> impl Scene {
    bsn! {
        Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: percent(100), height: percent(100) }
        template(|_| Ok(Pickable { should_block_lower: true, is_hoverable: false }))
    }
}

pub(super) fn canvas_color() -> Color {
    Color::srgb_u8(248, 238, 247)
}
