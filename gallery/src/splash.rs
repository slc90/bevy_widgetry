mod animation;
mod benchmark;
mod input;
mod lifecycle;
mod presentation;
mod scene;
mod visibility;

use bevy::prelude::*;
pub(crate) use scene::setup;

#[derive(Resource)]
pub(crate) struct SplashEnabled;

#[derive(Resource)]
struct SplashSession {
    gallery_camera: Entity,
    overlay: Entity,
    fallback: Option<Entity>,
    loaded: Vec<Handle<Image>>,
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

pub(crate) fn install(app: &mut App) -> Result {
    if [
        "GALLERY_WAVEFORM_BENCH_OUTPUT",
        "GALLERY_FILE_DIALOG_BENCH_OUTPUT",
        "GALLERY_PAGE_LIFECYCLE_BENCH_OUTPUT",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
    {
        return Ok(());
    }
    benchmark::install(app)?;
    if std::env::var_os("GALLERY_SPLASH_DISABLE").is_some() {
        return Ok(());
    }
    app.insert_resource(SplashEnabled)
        .init_resource::<SplashStatus>()
        .register_type::<SplashStatus>();
    animation::install(app)?;
    lifecycle::install(app)?;
    input::install(app);
    app.add_systems(
        PostUpdate,
        guard_clear
            .after(bevy::ui::UiSystems::Prepare)
            .run_if(resource_exists::<SplashSession>),
    );
    presentation::install(app)
}

fn cleanup(commands: &mut Commands, session: &mut SplashSession) {
    let camera = session.gallery_camera;
    commands.queue(move |world: &mut World| {
        if let Some(mut camera) = world.get_mut::<Camera>(camera) {
            camera.clear_color = ClearColorConfig::Custom(Color::NONE);
        }
    });
    commands.entity(session.overlay).try_despawn();
    if let Some(fallback) = session.fallback.take() {
        commands.entity(fallback).try_despawn();
    }
    commands.remove_resource::<SplashSession>();
    commands.remove_resource::<presentation::SplashPresentation>();
    commands.remove_resource::<SplashEnabled>();
    commands.remove_resource::<animation::SplashPreview>();
    commands.remove_resource::<animation::SplashAnimationClock>();
    commands.remove_resource::<lifecycle::SplashValidation>();
}

fn guard_clear(session: Res<SplashSession>, mut cameras: Query<&mut Camera>) {
    if let Ok(mut camera) = cameras.get_mut(session.gallery_camera) {
        camera.clear_color = ClearColorConfig::Custom(if session.fallback.is_some() {
            scene::canvas_color()
        } else {
            Color::NONE
        });
    }
}
