use super::SplashSession;
use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};
use bevy_widgetry::{scene::spawn_scene, splash::Splash, theme::WidgetryTheme};

const SHUTDOWN_PHYSICAL_SIZE: UVec2 = UVec2::new(1280, 720);

#[derive(Resource)]
struct StartupShutdown {
    main: Entity,
    view: Option<ShutdownView>,
}

struct ShutdownView {
    window: Entity,
    camera: Entity,
    root: Entity,
}

pub(crate) fn begin_shutdown(world: &mut World) -> Result {
    if world.contains_resource::<StartupShutdown>() || !super::is_starting(world) {
        return Ok(());
    }
    let main = world
        .get_resource::<SplashSession>()
        .map(|session| session.main)
        .or_else(|| {
            world
                .query_filtered::<Entity, With<PrimaryWindow>>()
                .iter(world)
                .next()
        });
    if let Some(main) = main {
        world.insert_resource(StartupShutdown { main, view: None });
    }
    super::cleanup(world)
}

pub(crate) fn shutdown_main(world: &World) -> Option<Entity> {
    world
        .get_resource::<StartupShutdown>()
        .map(|shutdown| shutdown.main)
}

pub(crate) fn shutdown_host(world: &mut World) -> Result<Option<Entity>> {
    let Some(shutdown) = world.get_resource::<StartupShutdown>() else {
        return Ok(None);
    };
    if let Some(view) = &shutdown.view {
        return Ok(Some(view.root));
    }
    let main = shutdown.main;
    let mut background = world
        .resource::<WidgetryTheme>()
        .window
        .frame
        .normal
        .background;
    background.set_alpha(1.0);
    let title = crate::i18n::LocalizedText::new(crate::i18n::TextKey::GalleryClose(
        crate::i18n::CloseKey::Waiting,
    ))
    .render(*world.resource::<crate::i18n::Language>());
    let window = Splash::start(
        world,
        main,
        Window {
            title,
            resolution: SHUTDOWN_PHYSICAL_SIZE.into(),
            skip_taskbar: true,
            ..default()
        },
    )
    .map_err(|error| error.into_bevy_error())?;
    world
        .entity_mut(window)
        .insert(Name::new("GalleryStartupShutdownWindow"));
    let camera = world
        .spawn((
            Name::new("GalleryStartupShutdownCamera"),
            Camera2d,
            Camera {
                clear_color: ClearColorConfig::Custom(background),
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window)),
        ))
        .id();
    let root = match spawn_scene(
        world,
        bsn! {
            Name("GalleryStartupShutdownHost")
            UiTargetCamera(camera)
            Node { width: percent(100), height: percent(100) }
            BackgroundColor(background)
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
    world.resource_mut::<StartupShutdown>().view = Some(ShutdownView {
        window,
        camera,
        root,
    });
    // The shutdown view uses an opaque host background and the existing shutdown panel.
    // It does not depend on the brand image that may have failed to load.
    Splash::ready(world, window).map_err(|error| error.into_bevy_error())?;
    Ok(Some(root))
}

pub(crate) fn cleanup_shutdown(world: &mut World) -> Result {
    let Some(shutdown) = world.remove_resource::<StartupShutdown>() else {
        return Ok(());
    };
    let Some(view) = shutdown.view else {
        return Ok(());
    };
    let result = if Splash::phase(world, view.window).is_some() {
        Splash::cancel(world, view.window).map_err(|error| error.into_bevy_error())
    } else {
        Ok(())
    };
    world.despawn(view.root);
    world.despawn(view.camera);
    result
}
