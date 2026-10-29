use super::{SplashSession, SplashStatus, presentation::SplashPresentation};
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::{Monitor, RawHandleWrapper};
use bevy::winit::WINIT_WINDOWS;
use std::sync::atomic::Ordering;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub(super) struct SplashPreview {
    pub(super) time_ms: f32,
    pub(super) paused: bool,
    pub(super) repeat: bool,
    pub(super) retain: bool,
}

#[derive(Resource, Default)]
pub(super) struct SplashAnimationClock {
    started: bool,
    visibility_error: Option<String>,
    advancing: bool,
    pub(super) visible_delta_ms: f32,
}

pub(super) fn install(app: &mut App) -> Result {
    let mode = std::env::var("GALLERY_SPLASH_PREVIEW").unwrap_or_default();
    let static_time = match mode.as_str() {
        "static" => Some(0.0),
        "" | "play" | "benchmark" => None,
        value => Some(value.parse::<f32>().map_err(|error| {
            error!(mode = value, %error, "Splash 预览时间无效");
            BevyError::error(error)
        })?),
    };
    if static_time.is_some_and(|time| !time.is_finite() || !(0.0..=5000.0).contains(&time)) {
        error!(mode, "Splash 预览时间必须位于 0–5000ms");
        return Err(BevyError::error("Splash 预览时间必须位于 0–5000ms"));
    }
    app.insert_resource(SplashPreview {
        time_ms: static_time.unwrap_or_default(),
        paused: static_time.is_some(),
        repeat: mode == "benchmark",
        retain: static_time.is_some() || mode == "benchmark",
    })
    .register_type::<SplashPreview>()
    .init_resource::<SplashAnimationClock>()
    .add_systems(Update, animate.run_if(resource_exists::<SplashSession>));
    Ok(())
}

pub(super) fn animate(
    time: Res<Time<Real>>,
    session: Res<SplashSession>,
    presentation: Res<SplashPresentation>,
    mut clock: ResMut<SplashAnimationClock>,
    preview: Res<SplashPreview>,
    mut status: ResMut<SplashStatus>,
    windows: Query<&Window>,
    _main_thread: NonSendMarker,
    handles: Query<&RawHandleWrapper>,
    monitors: Query<&Monitor>,
    mut images: Query<&mut ImageNode>,
) {
    let uncovered = match handles.get(presentation.window) {
        Ok(handle) => match super::visibility::is_uncovered(
            handle,
            monitors.iter().map(|monitor| winsafe::RECT {
                left: monitor.physical_position.x,
                top: monitor.physical_position.y,
                right: monitor.physical_position.x + monitor.physical_width as i32,
                bottom: monitor.physical_position.y + monitor.physical_height as i32,
            }),
        ) {
            Ok(visible) => {
                if clock.visibility_error.take().is_some() {
                    info!("Splash 窗口遮挡检测已恢复");
                }
                visible
            }
            Err(error) => {
                if clock.visibility_error.as_ref() != Some(&error) {
                    warn!(%error, "Splash 遮挡检测失败，按可见窗口继续播放");
                    clock.visibility_error = Some(error);
                }
                true
            }
        },
        Err(_) => false,
    };
    let visible = windows.get(presentation.window).is_ok_and(|window| {
        window.visible && window.physical_width() > 0 && window.physical_height() > 0
    }) && uncovered
        && WINIT_WINDOWS.with_borrow(|native| {
            native
                .get_window(presentation.window)
                .is_some_and(|window| {
                    window.is_visible() != Some(false) && window.is_minimized() != Some(true)
                })
        });
    clock.visible_delta_ms = if visible && status.visible {
        time.delta_secs() * 1000.0
    } else {
        0.0
    };
    status.visible = visible;
    status.source_ready = presentation.source_ready.load(Ordering::Acquire);
    status.presented = presentation.presented.load(Ordering::Acquire);
    if status.presented && !clock.started {
        clock.started = true;
        clock.advancing = false;
    }
    let advancing = clock.started && visible && !preview.paused;
    if advancing && clock.advancing && status.playing {
        status.elapsed_ms = (status.elapsed_ms + clock.visible_delta_ms).min(5000.0);
        if status.elapsed_ms >= 5000.0 && preview.repeat {
            status.elapsed_ms = 0.0;
        }
    }
    clock.advancing = advancing;
    if preview.paused {
        status.elapsed_ms = preview.time_ms.clamp(0.0, 5000.0);
    }
    status.frame_index = ((status.elapsed_ms / 125.0).floor() as usize).min(39);
    if let Ok(mut image) = images.get_mut(session.overlay) {
        image.image = session.loaded[status.frame_index].clone();
        if let Ok(window) = windows.get(presentation.window) {
            let viewport = window.physical_size().as_vec2();
            let source = Vec2::new(1920.0, 1080.0);
            let scale = (viewport / source).max_element();
            if scale > 0.0 {
                let crop = viewport / scale;
                let min = (source - crop) * 0.5;
                image.rect = Some(Rect::from_corners(min, min + crop));
            }
        }
    }
}
