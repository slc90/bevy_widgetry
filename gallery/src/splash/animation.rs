use super::{SplashSession, SplashStatus, presentation::SplashPresentation, scene::*};
use crate::assets::{SplashArtboard, SplashLayerKind};
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::{RequestRedraw, WindowOccluded};
use bevy::winit::WINIT_WINDOWS;
use std::sync::atomic::Ordering;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub(super) struct SplashPreview {
    time_ms: f32,
    paused: bool,
    replay: bool,
    speed: f32,
    repeat: bool,
}

#[derive(Resource, Default)]
struct SplashAnimationClock {
    elapsed: f32,
    started: bool,
    occluded: bool,
    advancing: bool,
}

pub(super) fn install(app: &mut App) -> Result {
    let mode = std::env::var("GALLERY_SPLASH_PREVIEW").unwrap_or_default();
    let static_time = match mode.as_str() {
        "static" | "baseline" => Some(800.0),
        "" | "play" | "benchmark" => None,
        value => Some(value.parse::<f32>().map_err(|error| {
            error!(mode = value, error = %error, "Splash 预览时间无效");
            BevyError::error(error)
        })?),
    };
    if static_time.is_some_and(|time| !time.is_finite() || !(0.0..=800.0).contains(&time)) {
        error!(mode, "Splash 预览时间必须位于 0–800ms");
        return Err(BevyError::error("Splash 预览时间必须位于 0–800ms"));
    }
    app.insert_resource(SplashPreview {
        time_ms: static_time.unwrap_or_default(),
        paused: static_time.is_some(),
        replay: false,
        speed: 1.0,
        repeat: mode == "benchmark",
    })
    .register_type::<SplashPreview>()
    .init_resource::<SplashAnimationClock>()
    .add_systems(
        Update,
        animate
            .after(super::update)
            .run_if(resource_exists::<SplashSession>),
    );
    Ok(())
}

fn animate(
    time: Res<Time<Real>>,
    artboard: Res<SplashArtboard>,
    session: Res<SplashSession>,
    presentation: Res<SplashPresentation>,
    mut clock: ResMut<SplashAnimationClock>,
    mut preview: ResMut<SplashPreview>,
    mut status: ResMut<SplashStatus>,
    windows: Query<&Window>,
    _main_thread: NonSendMarker,
    mut occlusion: MessageReader<WindowOccluded>,
    mut characters: Query<&mut UiTransform, With<SplashCharacter>>,
    mut layers: Query<(&SplashLayerNode, &mut UiTransform), Without<SplashCharacter>>,
    mut images: Query<(&SplashLayerImage, &mut ImageNode)>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    for event in occlusion.read() {
        if event.window == presentation.window {
            clock.occluded = event.occluded;
        }
    }
    let visible = windows.get(presentation.window).is_ok_and(|window| {
        window.visible && window.physical_width() > 0 && window.physical_height() > 0
    }) && !clock.occluded
        && WINIT_WINDOWS.with_borrow(|native| {
            native
                .get_window(presentation.window)
                .is_some_and(|window| {
                    window.is_visible() != Some(false) && window.is_minimized() != Some(true)
                })
        });
    if preview.replay {
        preview.replay = false;
        clock.elapsed = 0.0;
        clock.advancing = false;
    }
    if presentation.presented.load(Ordering::Acquire) && !clock.started {
        clock.started = true;
        clock.advancing = false;
    }
    let advancing = clock.started && visible && !preview.paused;
    if advancing && clock.advancing {
        let speed = if preview.speed.is_finite() {
            preview.speed.clamp(0.0, 10.0)
        } else {
            1.0
        };
        clock.elapsed = (clock.elapsed + time.delta_secs() * 1000.0 * speed).min(800.0);
        if clock.elapsed >= 800.0 && preview.repeat {
            clock.elapsed = 0.0;
        }
    }
    clock.advancing = advancing;
    let elapsed = if preview.paused && preview.time_ms.is_finite() {
        preview.time_ms.clamp(0.0, 800.0)
    } else {
        clock.elapsed
    };
    status.elapsed_ms = elapsed;
    status.playing = advancing && (elapsed < 800.0 || preview.repeat);
    if status.playing {
        redraw.write(RequestRedraw);
    }
    let progress = (elapsed / 200.0).clamp(0.0, 1.0);
    let back = 1.0 + 2.2 * (progress - 1.0).powi(3) + 1.2 * (progress - 1.0).powi(2);
    for mut character in &mut characters {
        character.translation = Val2::px(0.0, (artboard.artboard[1] + 1.0) * (1.0 - back));
    }
    let wave = ((elapsed - 200.0) / 350.0).clamp(0.0, 1.0);
    let ribbon = (elapsed / 800.0).clamp(0.0, 1.0);
    for (node, mut transform) in &mut layers {
        let Some((layout, _)) = session
            .loaded
            .iter()
            .find(|(layer, _)| layer.name == node.0)
        else {
            continue;
        };
        let degrees = match node.0 {
            SplashLayerKind::Arm => {
                6.0 * (wave * std::f32::consts::TAU * 2.0).sin()
                    * (wave * std::f32::consts::PI).sin()
            }
            SplashLayerKind::RibbonUpper => {
                3.0 * (ribbon * std::f32::consts::TAU * 2.0).sin() * (1.0 - ribbon)
            }
            SplashLayerKind::RibbonLower => {
                4.0 * (ribbon * std::f32::consts::TAU * 2.0 + 0.8).sin()
                    * (ribbon * std::f32::consts::PI).sin()
            }
            _ => 0.0,
        };
        transform.rotation = Rot2::degrees(layout.rotation_degrees + degrees);
    }
    let kind = if (610.0..680.0).contains(&elapsed) {
        SplashLayerKind::BodyClosed
    } else {
        SplashLayerKind::BodyOpen
    };
    if let Some((_, handle)) = session.loaded.iter().find(|(layer, _)| layer.name == kind) {
        for (node, mut image) in &mut images {
            if node.0 == SplashLayerKind::BodyOpen && image.image != *handle {
                image.image = handle.clone();
            }
        }
    }
}
