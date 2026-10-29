use super::{
    SplashSession, SplashStatus,
    animation::{SplashAnimationClock, SplashPreview},
    presentation::SplashPresentation,
};
use crate::{
    gallery::{GalleryPage, GalleryPageContent, GalleryPageHost},
    startup_cursor::GalleryFirstFrameReady,
};
use bevy::{asset::LoadState, prelude::*, window::RequestRedraw};
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Default, Debug, Reflect, PartialEq, Eq)]
enum SplashPhase {
    #[default]
    WaitingAssets,
    WaitingFirstSplashPresent,
    Playing,
    HoldingForGallery,
    Fading,
    Previewing,
    Finished,
    Fallback,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct SplashLifecycle {
    phase: SplashPhase,
    active_ms: f32,
    wait_ms: f32,
    fade_ms: f32,
}

#[derive(Resource)]
pub(super) struct SplashValidation {
    assets_delay_ms: f32,
    gallery_delay_ms: f32,
    timeout_ms: f32,
}

pub(super) fn install(app: &mut App) -> Result {
    let fault = std::env::var("GALLERY_SPLASH_FAULT").unwrap_or_default();
    if !matches!(fault.as_str(), "" | "asset") {
        error!(fault, "Splash 失败验证模式无效");
        return Err(BevyError::error("Splash 失败验证模式无效"));
    }
    app.insert_resource(SplashValidation {
        assets_delay_ms: duration("GALLERY_SPLASH_ASSETS_DELAY_MS", 0.0)?,
        gallery_delay_ms: duration("GALLERY_SPLASH_GALLERY_DELAY_MS", 0.0)?,
        timeout_ms: duration("GALLERY_SPLASH_TIMEOUT_MS", 15000.0)?,
    })
    .init_resource::<SplashLifecycle>()
    .register_type::<SplashLifecycle>()
    .add_systems(
        Update,
        advance
            .after(super::animation::animate)
            .run_if(resource_exists::<SplashSession>),
    );
    Ok(())
}

fn duration(name: &str, default_value: f32) -> Result<f32> {
    let Some(value) = std::env::var_os(name) else {
        return Ok(default_value);
    };
    let value = value.to_string_lossy().parse::<f32>().map_err(|error| {
        error!(name, value = ?value, %error, "Splash 验证参数无效");
        BevyError::error(error)
    })?;
    if !value.is_finite() || !(0.0..=60000.0).contains(&value) {
        error!(name, value, "Splash 验证时间必须位于0–60000ms");
        return Err(BevyError::error("Splash 验证时间必须位于0–60000ms"));
    }
    Ok(value)
}

fn advance(
    mut commands: Commands,
    mut session: ResMut<SplashSession>,
    mut presentation: ResMut<SplashPresentation>,
    mut lifecycle: ResMut<SplashLifecycle>,
    validation: Res<SplashValidation>,
    clock: Res<SplashAnimationClock>,
    preview: Res<SplashPreview>,
    mut status: ResMut<SplashStatus>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    windows: Query<&Window>,
    mut display: Query<&mut ImageNode>,
    gallery: (
        Query<&UiTargetCamera, With<Node>>,
        Query<(), With<GalleryPageHost>>,
        Query<&GalleryPageContent>,
        Res<State<GalleryPage>>,
        Option<Res<GalleryFirstFrameReady>>,
    ),
    mut redraw: MessageWriter<RequestRedraw>,
) {
    if windows.get(presentation.window).is_err() {
        lifecycle.phase = SplashPhase::Finished;
        super::cleanup(&mut commands, &mut session);
        return;
    }
    lifecycle.active_ms += clock.visible_delta_ms;
    let (nodes, hosts, content, page, ready) = gallery;
    let cpu_ready = session.loaded.iter().all(|h| images.contains(h.id()))
        && lifecycle.active_ms >= validation.assets_delay_ms;
    status.cpu_assets_ready = cpu_ready;
    let gallery_ready = ready.is_some_and(|ready| {
        ready.window == presentation.window && nodes.get(ready.background).is_ok()
    }) && *page.get() == GalleryPage::Button
        && hosts.iter().count() == 1
        && content
            .single()
            .is_ok_and(|content| content.0 == GalleryPage::Button)
        && lifecycle.active_ms >= validation.gallery_delay_ms;
    let previous_phase = lifecycle.phase;
    let mut failure = session
        .loaded
        .iter()
        .find_map(|h| match server.get_load_state(h.id()) {
            Some(LoadState::Failed(error)) => Some(format!("SplashAssetFailed: {error}")),
            _ => None,
        });
    if !cpu_ready {
        lifecycle.phase = SplashPhase::WaitingAssets;
    } else if !status.presented {
        lifecycle.phase = SplashPhase::WaitingFirstSplashPresent;
    } else if preview.retain {
        lifecycle.phase = SplashPhase::Previewing;
    } else if status.elapsed_ms < 5000.0 {
        lifecycle.phase = SplashPhase::Playing;
    } else if !gallery_ready {
        lifecycle.phase = SplashPhase::HoldingForGallery;
    } else {
        lifecycle.phase = SplashPhase::Fading;
    }
    if lifecycle.phase != previous_phase {
        lifecycle.wait_ms = 0.0;
    }
    let waiting = matches!(
        lifecycle.phase,
        SplashPhase::WaitingAssets
            | SplashPhase::WaitingFirstSplashPresent
            | SplashPhase::HoldingForGallery
    );
    if waiting || (lifecycle.phase == SplashPhase::Fading && lifecycle.fade_ms >= 200.0) {
        lifecycle.wait_ms += clock.visible_delta_ms;
        if lifecycle.wait_ms >= validation.timeout_ms {
            failure = Some(format!("Splash 等待超时: {:?}", lifecycle.phase));
        }
    }
    status.playing = cpu_ready
        && status.presented
        && (lifecycle.phase == SplashPhase::Playing
            || (preview.repeat && lifecycle.phase == SplashPhase::Previewing));
    if status.presented
        && cpu_ready
        && let Some(fallback) = session.fallback.take()
    {
        commands.entity(fallback).try_despawn();
    }
    if lifecycle.phase == SplashPhase::Fading {
        lifecycle.fade_ms = (lifecycle.fade_ms + clock.visible_delta_ms).min(200.0);
        status.alpha = 1.0 - lifecycle.fade_ms / 200.0;
        if let Ok(mut image) = display.get_mut(session.overlay) {
            image.color.set_alpha(status.alpha);
        } else {
            failure = Some("Splash 显示节点丢失".into());
        }
        if lifecycle.fade_ms >= 200.0 {
            presentation.submit_terminal();
        }
        if presentation.terminal_presented.load(Ordering::Acquire) {
            lifecycle.phase = SplashPhase::Finished;
            status.playing = false;
            super::cleanup(&mut commands, &mut session);
            redraw.write(RequestRedraw);
            return;
        }
    } else {
        status.alpha = 1.0;
    }
    if let Some(reason) = failure {
        warn!(
            reason,
            elapsed_ms = lifecycle.active_ms,
            "Splash 降级并释放覆盖，继续 Gallery"
        );
        lifecycle.phase = SplashPhase::Fallback;
        status.playing = false;
        super::cleanup(&mut commands, &mut session);
        redraw.write(RequestRedraw);
        return;
    }
    // Windows has no winit occlusion events; polling also detects exposure without a focus change.
    redraw.write(RequestRedraw);
}
