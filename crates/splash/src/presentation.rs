use crate::{Splash, SplashError, SplashFailure, error::failure};
use bevy::{
    camera::{CameraOutputMode, NormalizedRenderTarget},
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        camera::ExtractedCamera,
        sync_world::MainEntity,
        view::{ViewTarget, window::ExtractedWindow},
    },
    winit::{EventLoopProxyWrapper, WINIT_WINDOWS, WinitUserEvent},
};
use std::{
    collections::HashMap,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplashPhase {
    WaitingForContent,
    WaitingForFrame,
    Visible,
}

#[derive(Event, Reflect)]
#[reflect(Event)]
pub struct SplashReady {
    pub splash: Entity,
}

#[derive(Event)]
pub struct SplashShown {
    pub splash: Entity,
    pub main: Entity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FrameGeometry {
    window: Entity,
    size: UVec2,
    scale: f32,
}

#[derive(Clone)]
struct FrameRequest {
    geometry: FrameGeometry,
    outcome: Arc<OnceLock<Result<(), Arc<SplashFailure>>>>,
    canceled: Arc<AtomicBool>,
}

impl FrameRequest {
    fn new(geometry: FrameGeometry) -> Self {
        Self {
            geometry,
            outcome: default(),
            canceled: default(),
        }
    }

    fn cancel(&self) {
        self.canceled.store(true, Ordering::Release);
        if let Some(Err(failure)) = self.outcome.get() {
            failure.record_handled("取消过期的 Splash 呈现检查", "该帧不再用于显示窗口");
        }
    }
}

struct Session {
    main: Entity,
    ready: bool,
    visible: bool,
    halted: bool,
    ticket: Option<FrameRequest>,
}

impl Session {
    fn new(main: Entity) -> Self {
        Self {
            main,
            ready: false,
            visible: false,
            halted: false,
            ticket: None,
        }
    }

    fn phase(&self) -> SplashPhase {
        if self.visible {
            SplashPhase::Visible
        } else if self.ready {
            SplashPhase::WaitingForFrame
        } else {
            SplashPhase::WaitingForContent
        }
    }

    fn ready(&mut self) -> bool {
        let changed = !self.ready;
        self.ready = true;
        changed
    }

    fn prepare(&mut self, geometry: FrameGeometry) {
        if !self.ready
            || self.visible
            || self.halted
            || self
                .ticket
                .as_ref()
                .is_some_and(|ticket| matches!(ticket.outcome.get(), Some(Err(_))))
        {
            return;
        }
        if self
            .ticket
            .as_ref()
            .is_none_or(|ticket| ticket.geometry != geometry)
        {
            if let Some(previous) = self.ticket.take() {
                previous.cancel();
            }
            self.ticket = Some(FrameRequest::new(geometry));
        }
    }

    fn should_show(&self, geometry: FrameGeometry) -> bool {
        self.ready
            && !self.visible
            && !self.halted
            && self.ticket.as_ref().is_some_and(|ticket| {
                ticket.geometry == geometry
                    && !ticket.canceled.load(Ordering::Acquire)
                    && matches!(ticket.outcome.get(), Some(Ok(())))
            })
    }

    fn poll_failure(&mut self) -> Option<Arc<SplashFailure>> {
        if self.halted {
            return None;
        }
        if let Some(Err(failure)) = self.ticket.as_ref().and_then(|ticket| ticket.outcome.get()) {
            self.halted = true;
            return Some(Arc::clone(failure));
        }
        None
    }
}

#[derive(Resource, Default)]
pub(crate) struct Registry {
    sessions: HashMap<Entity, Session>,
}

pub(crate) fn register(world: &mut World, splash: Entity, main: Entity) {
    world
        .resource_mut::<Registry>()
        .sessions
        .insert(splash, Session::new(main));
}

impl Splash {
    pub fn phase(world: &World, splash: Entity) -> Option<SplashPhase> {
        world
            .get_resource::<Registry>()?
            .sessions
            .get(&splash)
            .map(Session::phase)
    }

    pub fn ready(world: &mut World, splash: Entity) -> Result<bool, SplashFailure> {
        let Some(mut registry) = world.get_resource_mut::<Registry>() else {
            return Err(failure(SplashError::PluginMissing, "splash.ready"));
        };
        let session = registry
            .sessions
            .get_mut(&splash)
            .ok_or_else(|| failure(SplashError::UnknownSplash { splash }, "splash.ready"))?;
        Ok(session.ready())
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Registry>()
        .register_type::<SplashReady>()
        .add_observer(on_ready)
        .add_observer(on_removed)
        .add_systems(PostUpdate, sync.in_set(crate::SplashSystems::Presentation));
}

fn on_ready(event: On<SplashReady>, registry: Res<Registry>, mut commands: Commands) {
    let splash = event.splash;
    if registry.sessions.contains_key(&splash) {
        commands.queue(move |world: &mut World| -> Result {
            Splash::ready(world, splash)
                .map(|_| ())
                .map_err(|failure| failure.into_bevy_error())
        });
    }
}

fn on_removed(event: On<Remove<crate::SplashWindow>>, mut registry: ResMut<Registry>) {
    if let Some(session) = registry.sessions.remove(&event.entity)
        && let Some(ticket) = session.ticket
    {
        ticket.cancel();
    }
}

fn sync(world: &mut World) -> Result {
    use bevy_widgetry_diagnostics::bevy::DiagnosticBatch;
    let (shown, failures) = world.resource_scope(|world, mut registry: Mut<Registry>| {
        let mut shown = Vec::new();
        let mut failures = DiagnosticBatch::default();
        for (&splash, session) in &mut registry.sessions {
            if session.visible || session.halted {
                continue;
            }
            if let Some(error) = session.poll_failure() {
                failures.push_shared(error);
                continue;
            }
            let Some(window) = world.get::<Window>(splash) else {
                session.halted = true;
                failures.push_shared(Arc::new(failure(
                    SplashError::WindowMissing { window: splash },
                    "splash.present",
                )));
                continue;
            };
            let geometry = FrameGeometry {
                window: splash,
                size: window.physical_size(),
                scale: window.scale_factor(),
            };
            if window.visible
                && let Some(mut window) = world.get_mut::<Window>(splash)
            {
                window.visible = false;
            }
            if !world
                .get::<crate::SplashWindow>(splash)
                .is_some_and(crate::SplashWindow::is_positioned)
            {
                if let Some(ticket) = session.ticket.take() {
                    ticket.cancel();
                }
                continue;
            }
            session.prepare(geometry);
            if session.should_show(geometry) {
                let native_shown = WINIT_WINDOWS.with_borrow(|windows| {
                    let Some(native) = windows.get_window(splash) else {
                        return false;
                    };
                    native.set_visible(true);
                    true
                });
                if !native_shown {
                    session.halted = true;
                    failures.push_shared(Arc::new(failure(
                        SplashError::WindowMissing { window: splash },
                        "splash.show",
                    )));
                    continue;
                }
                if let Some(mut window) = world.get_mut::<Window>(splash) {
                    window.visible = true;
                }
                session.visible = true;
                shown.push(SplashShown {
                    splash,
                    main: session.main,
                });
            }
        }
        (shown, failures)
    });
    for event in shown {
        world.trigger(event);
    }
    match failures.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[derive(Resource, Default)]
struct PendingFrames {
    requests: Vec<FrameRequest>,
    acquired: Vec<Entity>,
    wake: Option<winit::event_loop::EventLoopProxy<WinitUserEvent>>,
    needs_wake: bool,
}

pub(crate) fn finish(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<PendingFrames>()
        .add_systems(ExtractSchedule, extract)
        .add_systems(
            Render,
            acquired
                .after(RenderSystems::Prepare)
                .before(RenderSystems::Render),
        )
        .add_systems(
            Render,
            presented
                .after(RenderSystems::Render)
                .before(RenderSystems::Cleanup),
        );
}

fn extract(
    registry: Extract<Res<Registry>>,
    windows: Extract<Query<&Window>>,
    proxy: Extract<Option<Res<EventLoopProxyWrapper>>>,
    exiting: Extract<Option<Res<Messages<AppExit>>>>,
    mut pending: ResMut<PendingFrames>,
) {
    pending.requests.clear();
    pending.acquired.clear();
    pending.needs_wake = false;
    pending.wake = proxy.as_ref().map(|proxy| (***proxy).clone());
    if exiting
        .as_ref()
        .is_some_and(|messages| !messages.is_empty())
    {
        return;
    }
    for session in registry
        .sessions
        .values()
        .filter(|session| session.ready && !session.visible && !session.halted)
    {
        pending.needs_wake = true;
        let Some(ticket) = session.ticket.as_ref() else {
            continue;
        };
        if windows.get(ticket.geometry.window).is_ok_and(|window| {
            window.physical_size() == ticket.geometry.size
                && window.scale_factor() == ticket.geometry.scale
        }) {
            pending.requests.push(ticket.clone());
        }
    }
}

fn acquired(windows: Query<(&MainEntity, &ExtractedWindow)>, mut pending: ResMut<PendingFrames>) {
    pending.acquired = pending
        .requests
        .iter()
        .filter_map(|request| {
            windows
                .iter()
                .any(|(main, window)| {
                    main.id() == request.geometry.window
                        && window.swap_chain_texture.is_some()
                        && UVec2::new(window.physical_width, window.physical_height)
                            == request.geometry.size
                })
                .then_some(request.geometry.window)
        })
        .collect();
}

fn presented(
    pending: Res<PendingFrames>,
    windows: Query<(&MainEntity, &ExtractedWindow)>,
    cameras: Query<(&ViewTarget, &ExtractedCamera)>,
) {
    for request in &pending.requests {
        if request.canceled.load(Ordering::Acquire) || request.outcome.get().is_some() {
            continue;
        }
        let window = request.geometry.window;
        let consumed = pending.acquired.contains(&window)
            && windows
                .iter()
                .any(|(main, frame)| main.id() == window && frame.swap_chain_texture.is_none());
        let written = cameras.iter().any(|(view, camera)| view.needs_present()
            && matches!(camera.output_mode, CameraOutputMode::Write { .. })
            && matches!(camera.target, Some(NormalizedRenderTarget::Window(target)) if target.entity() == window)
            && camera.physical_target_size == Some(request.geometry.size));
        if consumed && written {
            let outcome = request.outcome.get_or_init(|| {
                winsafe::DwmFlush().map_err(|source| {
                    Arc::new(failure(
                        SplashError::Presentation { window, source },
                        "splash.present",
                    ))
                })
            });
            if request.canceled.load(Ordering::Acquire)
                && let Err(failure) = outcome
            {
                failure.record_handled("取消过期的 Splash 呈现检查", "该帧不再用于显示窗口");
            }
        }
    }
    if pending.needs_wake
        && let Some(proxy) = &pending.wake
    {
        // Hidden Windows do not reliably produce redraw callbacks.
        // A closed event loop means the host already ended this work.
        if let Err(winit::event_loop::EventLoopClosed(_)) = proxy.send_event(WinitUserEvent::WakeUp)
        {
            for request in &pending.requests {
                request.cancel();
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_diagnostics::ErrorFrame;

    fn frame(window: Entity, width: u32) -> FrameGeometry {
        FrameGeometry {
            window,
            size: UVec2::new(width, 270),
            scale: 1.0,
        }
    }

    #[test]
    fn source_and_exact_presented_frame_are_both_required_and_show_only_once() {
        let mut session = Session::new(Entity::PLACEHOLDER);
        let geometry = frame(Entity::from_bits(1), 480);
        session.prepare(geometry);
        assert!(session.ticket.is_none());
        assert!(!session.should_show(geometry));
        assert!(session.ready());
        assert!(!session.ready());
        session.prepare(geometry);
        assert!(!session.should_show(geometry));
        session
            .ticket
            .as_ref()
            .unwrap()
            .outcome
            .set(Ok(()))
            .unwrap();
        assert!(session.should_show(geometry));
        session.visible = true;
        assert!(!session.should_show(geometry));
        assert_eq!(session.phase(), SplashPhase::Visible);
    }

    #[test]
    fn stale_geometry_and_other_windows_cannot_unlock_a_new_request() {
        let mut session = Session::new(Entity::PLACEHOLDER);
        session.ready();
        let old_geometry = frame(Entity::from_bits(1), 480);
        session.prepare(old_geometry);
        let old = session.ticket.as_ref().unwrap().clone();
        let current = frame(Entity::from_bits(1), 640);
        session.prepare(current);
        old.outcome.set(Ok(())).unwrap();
        assert!(old.canceled.load(Ordering::Acquire));
        assert!(!session.should_show(current));
        session
            .ticket
            .as_ref()
            .unwrap()
            .outcome
            .set(Ok(()))
            .unwrap();
        assert!(!session.should_show(frame(Entity::from_bits(2), 640)));
        assert!(session.should_show(current));
        let scaled = FrameGeometry {
            scale: 2.0,
            ..current
        };
        session.prepare(scaled);
        assert!(!session.should_show(scaled));
        assert!(!session.should_show(current));
    }

    #[test]
    fn presentation_error_retains_identity_and_halts_without_a_visibility_change() {
        let mut session = Session::new(Entity::PLACEHOLDER);
        session.ready();
        let geometry = frame(Entity::from_bits(1), 480);
        session.prepare(geometry);
        let error = Arc::new(SplashFailure::new(
            SplashError::Presentation {
                window: geometry.window,
                source: winsafe::co::HRESULT::E_FAIL,
            },
            ErrorFrame::new("splash.test.present"),
        ));
        session
            .ticket
            .as_ref()
            .unwrap()
            .outcome
            .set(Err(Arc::clone(&error)))
            .unwrap();
        let reported = session.poll_failure().unwrap();
        assert!(Arc::ptr_eq(&error, &reported));
        assert!(session.halted && !session.visible);
        assert!(!session.should_show(geometry));
        session.prepare(frame(geometry.window, 640));
        assert_eq!(session.ticket.as_ref().unwrap().geometry, geometry);
        assert!(session.poll_failure().is_none());
    }

    #[test]
    fn readiness_after_presentation_sync_still_requests_the_next_update() {
        use bevy::ecs::system::RunSystemOnce;
        use bevy::render::MainWorld;

        let mut main = MainWorld::default();
        let mut session = Session::new(Entity::PLACEHOLDER);
        session.ready();
        main.insert_resource(Registry {
            sessions: HashMap::from([(Entity::from_bits(1), session)]),
        });
        let mut render = World::new();
        render.insert_resource(main);
        render.init_resource::<PendingFrames>();
        render.run_system_once(extract).unwrap();
        let pending = render.resource::<PendingFrames>();
        assert!(pending.requests.is_empty());
        assert!(pending.needs_wake);
        render
            .resource_mut::<MainWorld>()
            .resource_mut::<Registry>()
            .sessions
            .clear();
        render.run_system_once(extract).unwrap();
        assert!(!render.resource::<PendingFrames>().needs_wake);
    }

    #[test]
    fn render_thread_failures_reach_the_host_once_with_original_diagnostics() {
        use bevy::window::{Monitor, WindowPosition};
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
        use std::error::Error;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, crate::SplashPlugin))
            .set_error_handler(ErrorCapture::handler());
        app.world_mut().spawn(Monitor {
            name: None,
            physical_width: 1920,
            physical_height: 1080,
            physical_position: IVec2::ZERO,
            scale_factor: 1.0,
            refresh_rate_millihertz: None,
            video_modes: vec![],
        });
        let main = app
            .world_mut()
            .spawn(Window {
                visible: false,
                position: WindowPosition::At(IVec2::ZERO),
                ..default()
            })
            .id();
        let splashes = [0, 1].map(|_| {
            let splash = Splash::start(app.world_mut(), main, Window::default()).unwrap();
            Splash::ready(app.world_mut(), splash).unwrap();
            splash
        });
        app.update();
        let logs = LogCapture::default();
        let original = splashes.map(|splash| {
            let ticket = app.world().resource::<Registry>().sessions[&splash]
                .ticket
                .as_ref()
                .unwrap()
                .clone();
            let capture = logs.clone();
            std::thread::spawn(move || {
                capture.run(|| {
                    let error = Arc::new(failure(
                        SplashError::Presentation {
                            window: splash,
                            source: winsafe::co::HRESULT::E_FAIL,
                        },
                        "splash.test.render_thread",
                    ));
                    ticket.outcome.set(Err(Arc::clone(&error))).unwrap();
                    error
                })
            })
            .join()
            .unwrap()
        });
        let errors = ErrorCapture::default();
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostics = records[0]
            .managed()
            .unwrap()
            .diagnostics()
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), original.len());
        for error in &original {
            let reported = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.origin().id() == error.origin().id())
                .unwrap();
            let reported = (*reported as &dyn Error)
                .downcast_ref::<SplashFailure>()
                .unwrap();
            assert!(std::ptr::eq(error.as_ref(), reported));
            assert_eq!(
                reported
                    .kind()
                    .source()
                    .unwrap()
                    .downcast_ref::<winsafe::co::HRESULT>(),
                Some(&winsafe::co::HRESULT::E_FAIL)
            );
            assert_eq!(
                logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
        errors.run(|| app.update());
        assert!(errors.take_records().is_empty());
        for splash in splashes {
            assert!(!app.world().get::<Window>(splash).unwrap().visible);
            assert_eq!(
                Splash::phase(app.world(), splash),
                Some(SplashPhase::WaitingForFrame)
            );
        }
        assert!(!app.world().get::<Window>(main).unwrap().visible);
    }
}
