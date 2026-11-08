mod animation;
mod benchmark;
mod lifecycle;
mod presentation;
mod scene;
mod shutdown;
mod skip;
mod visibility;

use bevy::prelude::*;
pub(crate) use scene::setup;
pub(crate) use shutdown::{begin_shutdown, cleanup_shutdown, shutdown_host, shutdown_main};

#[derive(Resource)]
pub(crate) struct SplashEnabled;

#[derive(Resource)]
struct SplashSession {
    main: Entity,
    window: Entity,
    camera: Entity,
    overlay: Entity,
    loaded: Vec<Handle<Image>>,
    finish_requested: bool,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct SplashStatus {
    source_ready: bool,
    presented: bool,
    elapsed_ms: f32,
    frame_index: usize,
    playing: bool,
    visible: bool,
    cpu_assets_ready: bool,
    alpha: f32,
}

pub(crate) fn enabled_for_run() -> bool {
    !other_benchmark() && std::env::var_os("GALLERY_SPLASH_DISABLE").is_none()
}

fn other_benchmark() -> bool {
    [
        "GALLERY_WAVEFORM_BENCH_OUTPUT",
        "GALLERY_FILE_DIALOG_BENCH_OUTPUT",
        "GALLERY_PAGE_LIFECYCLE_BENCH_OUTPUT",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
}

pub(crate) fn install(app: &mut App, enabled: bool) -> Result {
    if other_benchmark() {
        return Ok(());
    }
    benchmark::install(app)?;
    if !enabled {
        return Ok(());
    }
    app.add_plugins(bevy_widgetry::splash::SplashPlugin)
        .insert_resource(SplashEnabled)
        .init_resource::<SplashStatus>()
        .register_type::<SplashStatus>()
        .add_observer(lifecycle::on_shown)
        .add_observer(lifecycle::on_completed);
    animation::install(app)?;
    lifecycle::install(app)?;
    skip::install(app);
    presentation::install(app)
}

fn cleanup(world: &mut World) -> Result {
    let mut result = Ok(());
    if let Some(session) = world.remove_resource::<SplashSession>() {
        if bevy_widgetry::splash::Splash::phase(world, session.window).is_some() {
            result = bevy_widgetry::splash::Splash::cancel(world, session.window)
                .map_err(|error| error.into_bevy_error());
        }
        world.despawn(session.overlay);
        world.despawn(session.camera);
    }
    world.remove_resource::<presentation::SplashPresentation>();
    world.remove_resource::<SplashEnabled>();
    world.remove_resource::<animation::SplashPreview>();
    world.remove_resource::<animation::SplashAnimationClock>();
    world.remove_resource::<lifecycle::SplashValidation>();
    if let Some(mut status) = world.get_resource_mut::<SplashStatus>() {
        status.visible = false;
        status.playing = false;
    }
    result
}

pub(crate) fn is_starting(world: &World) -> bool {
    world.contains_resource::<SplashEnabled>()
}

fn fail_startup(world: &mut World, error: BevyError) -> Result {
    match crate::close::request_failed_startup_exit(world) {
        Ok(()) => Err(error),
        Err(cleanup_error) => Err(crate::diagnostics::combine(error, cleanup_error)),
    }
}
