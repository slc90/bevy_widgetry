mod animation;
mod benchmark;
mod material;
mod presentation;
mod scene;

pub(crate) use material::SplashRevealMaterial;
pub(crate) use scene::setup;

use crate::assets::{SplashArtboard, SplashLayer};
use bevy::prelude::*;
use bevy::window::RequestRedraw;
use presentation::SplashPresentation;
use std::sync::atomic::Ordering;

#[derive(Resource)]
struct SplashSession {
    camera: Entity,
    scene_root: Entity,
    fallback: Option<Entity>,
    overlay: Entity,
    material: Handle<SplashRevealMaterial>,
    loaded: Vec<(SplashLayer, Handle<Image>)>,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct SplashStatus {
    source_ready: bool,
    presented: bool,
    elapsed_ms: f32,
    playing: bool,
}

pub(crate) fn install(app: &mut App) -> Result {
    if std::env::var_os("GALLERY_SPLASH_PREVIEW").is_none() {
        return Ok(());
    }
    app.insert_resource(SplashArtboard::load()?)
        .init_resource::<SplashStatus>()
        .register_type::<SplashStatus>()
        .add_plugins(UiMaterialPlugin::<SplashRevealMaterial>::default())
        .add_systems(Update, update.run_if(resource_exists::<SplashSession>));
    animation::install(app)?;
    benchmark::install(app)?;
    presentation::install(app)
}

fn update(
    mut commands: Commands,
    mut session: ResMut<SplashSession>,
    presentation: Res<SplashPresentation>,
    mut status: ResMut<SplashStatus>,
    windows: Query<&Window>,
    mut materials: ResMut<Assets<SplashRevealMaterial>>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    status.source_ready = presentation.source_ready.load(Ordering::Acquire);
    status.presented = presentation.presented.load(Ordering::Acquire);
    if status.source_ready {
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
        let artboard = Vec2::new(1672.0, 941.0);
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
    if windows.get(presentation.window).is_err() {
        commands.entity(session.camera).despawn();
        commands.entity(session.scene_root).despawn();
        commands.entity(session.overlay).despawn();
        if let Some(fallback) = session.fallback.take() {
            commands.entity(fallback).despawn();
        }
        commands.remove_resource::<SplashSession>();
        commands.remove_resource::<SplashPresentation>();
    }
}
