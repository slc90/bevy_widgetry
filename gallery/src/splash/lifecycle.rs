const DEFAULT_TIMEOUT_MS: f32 = 15000.0;
const MAX_TIMEOUT_MS: f32 = 60000.0;
const FADE_DURATION_MS: f32 = 200.0;

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
    HandingOff,
    Failed,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub(super) struct SplashLifecycle {
    phase: SplashPhase,
    active_ms: f32,
    wait_ms: f32,
    fade_ms: f32,
    ready_sent: bool,
    main_ready_sent: bool,
    finished_sent: bool,
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
        return Err(crate::diagnostics::at(
            "Splash 失败验证模式无效",
            bevy_widgetry::diagnostics::ErrorFrame::new("Splash 失败验证模式无效")
                .field("fault", fault),
        ));
    }
    app.insert_resource(SplashValidation {
        assets_delay_ms: duration("GALLERY_SPLASH_ASSETS_DELAY_MS", 0.0)?,
        gallery_delay_ms: duration("GALLERY_SPLASH_GALLERY_DELAY_MS", 0.0)?,
        timeout_ms: duration("GALLERY_SPLASH_TIMEOUT_MS", DEFAULT_TIMEOUT_MS)?,
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
        crate::diagnostics::at(
            error,
            bevy_widgetry::diagnostics::ErrorFrame::new("Splash 验证参数无效")
                .field("name", name)
                .field("value", format_args!("{:?}", value)),
        )
    })?;
    if !value.is_finite() || !(0.0..=MAX_TIMEOUT_MS).contains(&value) {
        return Err(crate::diagnostics::at(
            "Splash 验证时间必须位于0–60000ms",
            bevy_widgetry::diagnostics::ErrorFrame::new("Splash 验证时间必须位于0–60000ms")
                .field("name", name)
                .field("value", value),
        ));
    }
    Ok(value)
}

fn advance(
    mut commands: Commands,
    session: Res<SplashSession>,
    presentation: Res<SplashPresentation>,
    mut lifecycle: ResMut<SplashLifecycle>,
    validation: Res<SplashValidation>,
    clock: Res<SplashAnimationClock>,
    time: Res<Time<Real>>,
    preview: Res<SplashPreview>,
    mut status: ResMut<SplashStatus>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    windows: Query<&Window>,
    mut display: Query<&mut ImageNode>,
    mut skip: Query<&mut Visibility, With<super::skip::SplashSkip>>,
    gallery: (
        Query<&UiTargetCamera, With<Node>>,
        Query<(), With<GalleryPageHost>>,
        Query<&GalleryPageContent>,
        Res<State<GalleryPage>>,
        Option<Res<GalleryFirstFrameReady>>,
    ),
    mut redraw: MessageWriter<RequestRedraw>,
) {
    use bevy_widgetry::splash::{MainWindowReady, SplashFinished, SplashReady};
    if matches!(lifecycle.phase, SplashPhase::Finished | SplashPhase::Failed) {
        return;
    }
    lifecycle.active_ms += time.delta_secs() * super::animation::MILLISECONDS_PER_SECOND;
    let (nodes, hosts, content, page, ready) = gallery;
    let cpu_ready = session
        .loaded
        .iter()
        .all(|handle| images.contains(handle.id()))
        && lifecycle.active_ms >= validation.assets_delay_ms;
    status.cpu_assets_ready = cpu_ready;
    let gallery_ready = ready
        .is_some_and(|ready| ready.window == session.main && nodes.get(ready.background).is_ok())
        && *page.get() != GalleryPage::Initializing
        && hosts.iter().count() == 1
        && content
            .single()
            .is_ok_and(|content| content.0 == *page.get())
        && lifecycle.active_ms >= validation.gallery_delay_ms;
    let mut failures: Vec<BevyError> = session
        .loaded
        .iter()
        .filter_map(|handle| match server.get_load_state(handle.id()) {
            Some(LoadState::Failed(source)) => Some(crate::diagnostics::at(
                source,
                bevy_widgetry::diagnostics::ErrorFrame::new("gallery.splash.asset")
                    .field("asset", format_args!("{:?}", handle.id())),
            )),
            _ => None,
        })
        .collect();
    if windows.get(session.window).is_err() {
        failures.push(crate::diagnostics::at(
            SplashContentError::WindowLost {
                window: session.window,
            },
            bevy_widgetry::diagnostics::ErrorFrame::new("gallery.splash.window"),
        ));
    }
    if cpu_ready && presentation.source_ready.load(Ordering::Acquire) && !lifecycle.ready_sent {
        lifecycle.ready_sent = true;
        commands.trigger(SplashReady {
            splash: session.window,
        });
    }
    if gallery_ready && !lifecycle.main_ready_sent {
        lifecycle.main_ready_sent = true;
        commands.trigger(MainWindowReady {
            splash: session.window,
        });
    }
    let previous_phase = lifecycle.phase;
    if !cpu_ready {
        lifecycle.phase = SplashPhase::WaitingAssets;
    } else if !status.presented {
        lifecycle.phase = SplashPhase::WaitingFirstSplashPresent;
    } else if lifecycle.finished_sent {
        lifecycle.phase = SplashPhase::HandingOff;
    } else if preview.retain && !session.finish_requested {
        lifecycle.phase = SplashPhase::Previewing;
    } else if !session.finish_requested
        && status.elapsed_ms < super::animation::ANIMATION_DURATION_MS
    {
        lifecycle.phase = SplashPhase::Playing;
    } else if !gallery_ready {
        lifecycle.phase = SplashPhase::HoldingForGallery;
    } else {
        lifecycle.phase = SplashPhase::Fading;
    }
    for mut visible in &mut skip {
        *visible = if status.presented
            && cpu_ready
            && !session.finish_requested
            && status.elapsed_ms < super::animation::ANIMATION_DURATION_MS
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if lifecycle.phase != previous_phase {
        lifecycle.wait_ms = 0.0;
        debug!(from = ?previous_phase, to = ?lifecycle.phase, "Splash 阶段已切换");
    }
    let waiting = matches!(
        lifecycle.phase,
        SplashPhase::WaitingAssets
            | SplashPhase::WaitingFirstSplashPresent
            | SplashPhase::HoldingForGallery
            | SplashPhase::HandingOff
    );
    if waiting {
        lifecycle.wait_ms += time.delta_secs() * super::animation::MILLISECONDS_PER_SECOND;
        if lifecycle.wait_ms >= validation.timeout_ms {
            failures.push(crate::diagnostics::at(
                SplashContentError::Timeout {
                    phase: lifecycle.phase,
                    timeout_ms: validation.timeout_ms,
                },
                bevy_widgetry::diagnostics::ErrorFrame::new("gallery.splash.timeout"),
            ));
        }
    }
    status.playing = cpu_ready
        && status.presented
        && (lifecycle.phase == SplashPhase::Playing
            || (preview.repeat && lifecycle.phase == SplashPhase::Previewing));
    if lifecycle.phase == SplashPhase::Fading {
        lifecycle.fade_ms = (lifecycle.fade_ms + clock.visible_delta_ms).min(FADE_DURATION_MS);
        status.alpha = 1.0 - lifecycle.fade_ms / FADE_DURATION_MS;
        if let Ok(mut image) = display.get_mut(session.overlay) {
            image.color.set_alpha(status.alpha);
        } else {
            failures.push(crate::diagnostics::at(
                SplashContentError::DisplayMissing {
                    display: session.overlay,
                },
                bevy_widgetry::diagnostics::ErrorFrame::new("gallery.splash.display"),
            ));
        }
        if lifecycle.fade_ms >= FADE_DURATION_MS && !lifecycle.finished_sent {
            lifecycle.finished_sent = true;
            commands.trigger(SplashFinished {
                splash: session.window,
            });
            debug!(window = ?session.window, "Gallery Splash 内容已结束，等待库安全交接");
        }
    } else if !lifecycle.finished_sent {
        status.alpha = 1.0;
    }
    if let Some(error) = failures.into_iter().reduce(crate::diagnostics::combine) {
        lifecycle.phase = SplashPhase::Failed;
        status.playing = false;
        commands.queue(move |world: &mut World| super::fail_startup(world, error));
    }
    // Windows does not report native occlusion; keep the host animation and startup waits advancing.
    redraw.write(RequestRedraw);
}

pub(super) fn on_shown(
    event: On<bevy_widgetry::splash::SplashShown>,
    session: Option<Res<SplashSession>>,
    presentation: Option<Res<SplashPresentation>>,
    mut status: ResMut<SplashStatus>,
) {
    if session
        .as_ref()
        .is_some_and(|session| session.window == event.splash)
        && let Some(presentation) = presentation
    {
        presentation.presented.store(true, Ordering::Release);
        status.presented = true;
        debug!(window = ?event.splash, "Gallery Splash 首帧已安全显示");
    }
}

pub(super) fn on_completed(
    event: On<bevy_widgetry::splash::SplashCompleted>,
    session: Option<Res<SplashSession>>,
    mut lifecycle: ResMut<SplashLifecycle>,
    mut status: ResMut<SplashStatus>,
    mut commands: Commands,
) {
    if !session
        .as_ref()
        .is_some_and(|session| session.window == event.splash)
    {
        return;
    }
    lifecycle.phase = SplashPhase::Finished;
    status.playing = false;
    status.visible = false;
    let window = event.splash;
    debug!(?window, main = ?event.main, "Gallery 已完成独立 Splash 交接");
    commands.queue(move |world: &mut World| -> Result {
        if world
            .get_resource::<SplashSession>()
            .is_some_and(|session| session.window == window)
        {
            super::cleanup(world)?;
        }
        Ok(())
    });
}

#[derive(Debug)]
enum SplashContentError {
    Timeout { phase: SplashPhase, timeout_ms: f32 },
    WindowLost { window: Entity },
    DisplayMissing { display: Entity },
}
impl std::fmt::Display for SplashContentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { phase, timeout_ms } => write!(
                formatter,
                "Gallery Splash 等待超时：phase={phase:?}, timeout_ms={timeout_ms}"
            ),
            Self::WindowLost { window } => {
                write!(formatter, "Gallery Splash 原生窗口已丢失：{window:?}")
            }
            Self::DisplayMissing { display } => {
                write!(formatter, "Gallery Splash 内容节点已丢失：{display:?}")
            }
        }
    }
}
impl std::error::Error for SplashContentError {}
