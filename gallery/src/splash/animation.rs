pub(super) const ANIMATION_DURATION_MS: f32 = 5000.0;
const MILLISECONDS_PER_SECOND: f32 = 1000.0;
const FRAME_DURATION_MS: f32 = 125.0;
const ANIMATION_FRAME_COUNT: usize = 40;
const SOURCE_SIZE: Vec2 = Vec2::new(1920.0, 1080.0);

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
    visibility_error: Option<super::visibility::VisibilityError>,
    advancing: bool,
    pub(super) visible_delta_ms: f32,
}

pub(super) fn install(app: &mut App) -> Result {
    let mode = std::env::var("GALLERY_SPLASH_PREVIEW").unwrap_or_default();
    let static_time = match mode.as_str() {
        "static" => Some(0.0),
        "" | "play" | "benchmark" => None,
        value => Some(value.parse::<f32>().map_err(|error| {
            crate::diagnostics::at(
                error,
                bevy_widgetry::diagnostics::ErrorFrame::new("Splash 预览时间无效")
                    .field("mode", value),
            )
        })?),
    };
    if static_time
        .is_some_and(|time| !time.is_finite() || !(0.0..=ANIMATION_DURATION_MS).contains(&time))
    {
        return Err(crate::diagnostics::at(
            "Splash 预览时间必须位于 0–5000ms",
            bevy_widgetry::diagnostics::ErrorFrame::new("Splash 预览时间必须位于 0–5000ms")
                .field("mode", mode),
        ));
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
                    let failure = crate::diagnostics::failure(
                        crate::diagnostics::GalleryError::Operation(error.clone().into()),
                        bevy_widgetry::diagnostics::ErrorFrame::new("gallery.splash.visibility")
                            .field("window", format_args!("{:?}", presentation.window)),
                    );
                    clock.visibility_error = Some(error);
                    failure.record_handled(
                        "降级为可见窗口继续播放 Splash",
                        "保持动画并等待可见性查询恢复",
                    );
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
        time.delta_secs() * MILLISECONDS_PER_SECOND
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
    let advancing = clock.started && visible && !preview.paused && !session.finish_requested;
    if advancing && clock.advancing && status.playing {
        status.elapsed_ms = (status.elapsed_ms + clock.visible_delta_ms).min(ANIMATION_DURATION_MS);
        if status.elapsed_ms >= ANIMATION_DURATION_MS && preview.repeat {
            status.elapsed_ms = 0.0;
        }
    }
    clock.advancing = advancing;
    if preview.paused && !session.finish_requested {
        status.elapsed_ms = preview.time_ms.clamp(0.0, ANIMATION_DURATION_MS);
    }
    status.frame_index =
        ((status.elapsed_ms / FRAME_DURATION_MS).floor() as usize).min(ANIMATION_FRAME_COUNT - 1);
    if let Ok(mut image) = images.get_mut(session.overlay) {
        image.image = session.loaded[status.frame_index].clone();
        if let Ok(window) = windows.get(presentation.window) {
            let viewport = window.physical_size().as_vec2();
            let source = SOURCE_SIZE;
            let scale = (viewport / source).max_element();
            if scale > 0.0 {
                let crop = viewport / scale;
                let min = (source - crop) * 0.5;
                image.rect = Some(Rect::from_corners(min, min + crop));
            }
        }
    }
}
