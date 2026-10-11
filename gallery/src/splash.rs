mod animation;
mod benchmark;
mod input;
mod lifecycle;
mod material;
mod presentation;
mod ripple;
mod scene;

pub(crate) use material::SplashRevealMaterial;
pub(crate) use scene::setup;

use crate::assets::{SplashArtboard, SplashLayer};
use bevy::prelude::*;
use bevy::shader::Shader;
use bevy::window::RequestRedraw;
use presentation::SplashPresentation;
use std::sync::atomic::Ordering;

#[derive(Resource)]
struct SplashSession {
    gallery_camera: Entity,
    camera: Entity,
    scene_root: Entity,
    fallback: Option<Entity>,
    overlay: Entity,
    material: Handle<SplashRevealMaterial>,
    loaded: Vec<(SplashLayer, Handle<Image>)>,
    shader: Handle<Shader>,
    reveal_center: Option<Vec2>,
    terminal_submitted: bool,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct SplashStatus {
    source_ready: bool,
    presented: bool,
    elapsed_ms: f32,
    playing: bool,
    visible: bool,
    cpu_assets_ready: bool,
}

pub(crate) fn install(app: &mut App) -> Result {
    if [
        "GALLERY_SPLASH_DISABLE",
        "GALLERY_WAVEFORM_BENCH_OUTPUT",
        "GALLERY_FILE_DIALOG_BENCH_OUTPUT",
        "GALLERY_PAGE_LIFECYCLE_BENCH_OUTPUT",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
    {
        return Ok(());
    }
    app.insert_resource(SplashArtboard::load()?)
        .init_resource::<SplashStatus>()
        .register_type::<SplashStatus>()
        .add_plugins(UiMaterialPlugin::<SplashRevealMaterial>::default())
        .add_systems(Update, update.run_if(resource_exists::<SplashSession>));
    app.add_systems(
        PostUpdate,
        guard_clear
            .after(bevy::ui::UiSystems::Prepare)
            .run_if(resource_exists::<SplashSession>),
    );
    animation::install(app)?;
    lifecycle::install(app)?;
    input::install(app);
    benchmark::install(app)?;
    presentation::install(app)
}

fn update(
    mut commands: Commands,
    mut session: ResMut<SplashSession>,
    presentation: Res<SplashPresentation>,
    mut status: ResMut<SplashStatus>,
    windows: Query<&Window>,
    artboard: Res<SplashArtboard>,
    mut materials: ResMut<Assets<SplashRevealMaterial>>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    status.source_ready = presentation.source_ready.load(Ordering::Acquire);
    status.presented = presentation.presented.load(Ordering::Acquire);
    if status.source_ready && status.cpu_assets_ready {
        commands.entity(session.overlay).insert(Visibility::Visible);
    }
    if status.presented {
        if let Some(fallback) = session.fallback.take() {
            commands.entity(fallback).despawn();
        }
    } else {
        redraw.write(RequestRedraw);
    }
    if let Ok(window) = windows.get(presentation.window) {
        let viewport = window.physical_size().as_vec2();
        let artboard = Vec2::from_array(artboard.artboard);
        let scale = (viewport / artboard).max_element();
        if scale > 0.0 {
            let visible = viewport / (artboard * scale);
            if let Some(mut material) = materials.get_mut(&session.material) {
                let cover = ((Vec2::ONE - visible) * 0.5)
                    .extend(visible.x)
                    .extend(visible.y);
                if material.cover != cover {
                    material.cover = cover;
                }
            }
        }
    }
}

fn cleanup(commands: &mut Commands, session: &mut SplashSession) {
    let camera = session.gallery_camera;
    commands.queue(move |world: &mut World| {
        if let Some(mut camera) = world.get_mut::<Camera>(camera) {
            camera.clear_color = ClearColorConfig::Custom(Color::NONE);
        }
    });
    commands.entity(session.camera).try_despawn();
    commands.entity(session.scene_root).try_despawn();
    commands.entity(session.overlay).try_despawn();
    if let Some(fallback) = session.fallback.take() {
        commands.entity(fallback).try_despawn();
    }
    commands.remove_resource::<SplashSession>();
    commands.remove_resource::<SplashPresentation>();
    commands.remove_resource::<SplashArtboard>();
    animation::cleanup(commands);
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
