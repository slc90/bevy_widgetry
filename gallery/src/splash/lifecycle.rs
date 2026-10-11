use super::{SplashSession, SplashStatus, presentation::SplashPresentation};
use crate::{
    gallery::{GalleryPage, GalleryPageContent, GalleryPageHost},
    startup_cursor::GalleryFirstFrameReady,
};
use bevy::{asset::LoadState, prelude::*, shader::Shader, window::RequestRedraw};
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Default, Reflect, PartialEq, Eq)]
enum SplashPhase {
    #[default]
    OpaqueCover,
    WaitingAssets,
    WaitingFirstSplashPresent,
    Playing,
    HoldingForGallery,
    Revealing,
    Previewing,
    Finished,
    Fallback,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct SplashLifecycle {
    phase: SplashPhase,
    active_ms: f32,
    was_visible: bool,
    assets_ready_ms: Option<f32>,
    first_present_ms: Option<f32>,
    gallery_ready_ms: Option<f32>,
    reveal_start_ms: Option<f32>,
    reveal_done_ms: Option<f32>,
}

#[derive(Resource)]
struct SplashValidation {
    assets_delay_ms: f32,
    gallery_delay_ms: f32,
    timeout_ms: f32,
    retain: bool,
}

pub(super) fn install(app: &mut App) -> Result {
    let mode = std::env::var("GALLERY_SPLASH_PREVIEW").unwrap_or_default();
    let retain = mode != "play" && !mode.is_empty();
    let fault = std::env::var("GALLERY_SPLASH_FAULT").unwrap_or_default();
    if !matches!(fault.as_str(), "" | "asset" | "shader") {
        error!(fault, "Splash 失败验证模式无效");
        return Err(BevyError::error("Splash 失败验证模式无效"));
    }
    app.insert_resource(SplashValidation {
        assets_delay_ms: duration("GALLERY_SPLASH_ASSETS_DELAY_MS", 0.0)?,
        gallery_delay_ms: duration("GALLERY_SPLASH_GALLERY_DELAY_MS", 0.0)?,
        timeout_ms: duration("GALLERY_SPLASH_TIMEOUT_MS", 5000.0)?,
        retain,
    })
    .init_resource::<SplashLifecycle>()
    .register_type::<SplashLifecycle>()
    .add_systems(
        Update,
        advance
            .after(super::animation::SplashAnimate)
            .run_if(resource_exists::<SplashSession>),
    );
    Ok(())
}

fn duration(name: &str, default_value: f32) -> Result<f32> {
    let Some(value) = std::env::var_os(name) else {
        return Ok(default_value);
    };
    let value = value.to_string_lossy().parse::<f32>().map_err(|error| {
        error!(name, value = ?value, error = %error, "Splash 验证参数无效");
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
    mut status: ResMut<SplashStatus>,
    time: Res<Time<Real>>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    shaders: Res<Assets<Shader>>,
    windows: Query<&Window>,
    mut ripple: (
        Res<crate::assets::SplashArtboard>,
        Res<super::animation::SplashPreview>,
        ResMut<Assets<super::SplashRevealMaterial>>,
        Query<&mut MaterialNode<super::SplashRevealMaterial>>,
    ),
    gallery: (
        Query<&UiTargetCamera, With<Node>>,
        Query<(), With<GalleryPageHost>>,
        Query<&GalleryPageContent>,
        Res<State<GalleryPage>>,
        Option<Res<GalleryFirstFrameReady>>,
    ),
    mut redraw: MessageWriter<RequestRedraw>,
) {
    let (nodes, hosts, content, page, ready) = gallery;
    let Ok(window) = windows.get(presentation.window) else {
        lifecycle.phase = SplashPhase::Finished;
        super::cleanup(&mut commands, &mut session);
        commands.remove_resource::<SplashValidation>();
        return;
    };
    if status.visible && lifecycle.was_visible {
        lifecycle.active_ms += time.delta_secs() * 1000.0;
    }
    lifecycle.was_visible = status.visible;
    let mut failure = None;
    for (_, handle) in &session.loaded {
        if let Some(LoadState::Failed(error)) = server.get_load_state(handle.id()) {
            failure = Some(format!("SplashAssetFailed: {error}"));
            break;
        }
    }
    if let Some(LoadState::Failed(error)) = server.get_load_state(session.shader.id()) {
        failure = Some(format!("SplashPipelineFailed: {error}"));
    }
    let cpu_ready = session
        .loaded
        .iter()
        .all(|(_, handle)| images.contains(handle.id()))
        && shaders.contains(session.shader.id())
        && lifecycle.active_ms >= validation.assets_delay_ms;
    if cpu_ready && lifecycle.assets_ready_ms.is_none() {
        lifecycle.assets_ready_ms = Some(lifecycle.active_ms);
    }
    status.cpu_assets_ready = cpu_ready;
    if presentation.presented.load(Ordering::Acquire) && lifecycle.first_present_ms.is_none() {
        lifecycle.first_present_ms = Some(lifecycle.active_ms);
    }
    let gallery_ready = ready.is_some_and(|ready| {
        ready.window == presentation.window && nodes.get(ready.background).is_ok()
    }) && *page.get() == GalleryPage::Button
        && hosts.iter().count() == 1
        && content
            .single()
            .is_ok_and(|content| content.0 == GalleryPage::Button)
        && lifecycle.active_ms >= validation.gallery_delay_ms;
    if gallery_ready && lifecycle.gallery_ready_ms.is_none() {
        lifecycle.gallery_ready_ms = Some(lifecycle.active_ms);
    }
    if failure.is_none()
        && lifecycle.active_ms >= validation.timeout_ms
        && (!cpu_ready || lifecycle.first_present_ms.is_none() || !gallery_ready)
    {
        failure = Some(
            if !cpu_ready {
                "SplashAssetFailed: 资源等待超时"
            } else if lifecycle.first_present_ms.is_none() {
                "SplashPipelineFailed: 首帧等待超时"
            } else {
                "GalleryNotReady: 首屏等待超时"
            }
            .into(),
        );
    }
    if lifecycle.reveal_start_ms.is_some_and(|start| {
        lifecycle.active_ms - start >= validation.timeout_ms
            && !presentation.terminal_presented.load(Ordering::Acquire)
    }) {
        failure = Some("SplashPipelineFailed: 透明末帧等待超时".into());
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
        commands.remove_resource::<SplashValidation>();
        redraw.write(RequestRedraw);
        return;
    }
    if !cpu_ready {
        lifecycle.phase = SplashPhase::WaitingAssets;
    } else if lifecycle.first_present_ms.is_none() {
        lifecycle.phase = SplashPhase::WaitingFirstSplashPresent;
    } else if status.elapsed_ms < 800.0 {
        lifecycle.phase = SplashPhase::Playing;
    } else if !gallery_ready {
        lifecycle.phase = SplashPhase::HoldingForGallery;
    } else if validation.retain {
        lifecycle.phase = SplashPhase::Previewing;
        let progress = if ripple.1.reveal.is_finite() {
            ripple.1.reveal.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if !super::ripple::apply(
            &mut session,
            &mut presentation,
            &ripple.0,
            window.physical_size().as_vec2(),
            progress,
            false,
            (&mut ripple.2, &mut ripple.3),
        ) {
            warn!("SplashPipelineFailed: 预览材质丢失，释放覆盖并继续 Gallery");
            lifecycle.phase = SplashPhase::Fallback;
            super::cleanup(&mut commands, &mut session);
            commands.remove_resource::<SplashValidation>();
            redraw.write(RequestRedraw);
            return;
        }
    } else {
        lifecycle.phase = SplashPhase::Revealing;
        let active_ms = lifecycle.active_ms;
        let start = *lifecycle.reveal_start_ms.get_or_insert(active_ms);
        let progress = ((lifecycle.active_ms - start) / 200.0).clamp(0.0, 1.0);
        if !super::ripple::apply(
            &mut session,
            &mut presentation,
            &ripple.0,
            window.physical_size().as_vec2(),
            progress,
            progress >= 1.0,
            (&mut ripple.2, &mut ripple.3),
        ) {
            warn!("SplashPipelineFailed: 揭幕材质丢失，释放覆盖并继续 Gallery");
            lifecycle.phase = SplashPhase::Fallback;
            super::cleanup(&mut commands, &mut session);
            commands.remove_resource::<SplashValidation>();
            redraw.write(RequestRedraw);
            return;
        }
        if presentation.terminal_presented.load(Ordering::Acquire) {
            lifecycle.reveal_done_ms = Some(lifecycle.active_ms);
            lifecycle.phase = SplashPhase::Finished;
            super::cleanup(&mut commands, &mut session);
            commands.remove_resource::<SplashValidation>();
            redraw.write(RequestRedraw);
            return;
        }
    }
    if !validation.retain || lifecycle.first_present_ms.is_none() {
        redraw.write(RequestRedraw);
    }
}
