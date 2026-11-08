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
    WaitingForMainContent,
    WaitingForMainFrame,
    HandingOff,
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

#[derive(Event, Reflect)]
#[reflect(Event)]
pub struct SplashFinished {
    pub splash: Entity,
}

#[derive(Event, Reflect)]
#[reflect(Event)]
pub struct MainWindowReady {
    pub splash: Entity,
}

#[derive(Event)]
pub struct SplashCompleted {
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
    finished: bool,
    main_ready: bool,
    handoff: bool,
    main_ticket: Option<FrameRequest>,
}

impl Session {
    fn new(main: Entity) -> Self {
        Self {
            main,
            ready: false,
            visible: false,
            halted: false,
            ticket: None,
            finished: false,
            main_ready: false,
            handoff: false,
            main_ticket: None,
        }
    }

    fn phase(&self) -> SplashPhase {
        if self.handoff {
            SplashPhase::HandingOff
        } else if self.visible && self.finished {
            if self.main_ready {
                SplashPhase::WaitingForMainFrame
            } else {
                SplashPhase::WaitingForMainContent
            }
        } else if self.visible {
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

    fn finish(&mut self) -> bool {
        let changed = !self.finished;
        self.finished = true;
        changed
    }

    fn main_ready(&mut self) -> bool {
        let changed = !self.main_ready;
        self.main_ready = true;
        changed
    }

    fn prepare_main(&mut self, geometry: FrameGeometry) {
        if !self.visible || !self.finished || !self.main_ready || self.halted || self.handoff {
            return;
        }
        prepare_ticket(&mut self.main_ticket, geometry);
    }

    fn can_handoff(&self, geometry: FrameGeometry) -> bool {
        self.visible
            && self.finished
            && self.main_ready
            && !self.halted
            && !self.handoff
            && ticket_ready(&self.main_ticket, geometry)
    }

    fn begin_handoff(&mut self, geometry: FrameGeometry) {
        if let Some(previous) = self.main_ticket.take() {
            previous.cancel();
        }
        self.main_ticket = Some(FrameRequest::new(geometry));
        self.handoff = true;
    }

    fn handoff_ready(&self, geometry: FrameGeometry) -> bool {
        self.handoff && !self.halted && ticket_ready(&self.main_ticket, geometry)
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
        prepare_ticket(&mut self.ticket, geometry);
    }

    fn should_show(&self, geometry: FrameGeometry) -> bool {
        self.ready && !self.visible && !self.halted && ticket_ready(&self.ticket, geometry)
    }

    fn poll_failure(&mut self) -> Option<Arc<SplashFailure>> {
        if self.halted {
            return None;
        }
        if let Some(failure) = [&self.ticket, &self.main_ticket]
            .into_iter()
            .find_map(
                |ticket| match ticket.as_ref().and_then(|ticket| ticket.outcome.get()) {
                    Some(Err(failure)) => Some(failure),
                    _ => None,
                },
            )
        {
            self.halted = true;
            return Some(Arc::clone(failure));
        }
        None
    }

    fn cancel(&self) {
        for ticket in [&self.ticket, &self.main_ticket].into_iter().flatten() {
            ticket.cancel();
        }
    }

    fn needs_wake(&self) -> bool {
        !self.halted
            && ((!self.visible && self.ready)
                || (self.visible && self.finished && self.main_ready)
                || self.handoff)
    }
}

fn prepare_ticket(slot: &mut Option<FrameRequest>, geometry: FrameGeometry) {
    if slot
        .as_ref()
        .is_some_and(|ticket| matches!(ticket.outcome.get(), Some(Err(_))))
    {
        return;
    }
    if slot
        .as_ref()
        .is_none_or(|ticket| ticket.geometry != geometry)
    {
        if let Some(previous) = slot.take() {
            previous.cancel();
        }
        *slot = Some(FrameRequest::new(geometry));
    }
}

fn ticket_ready(ticket: &Option<FrameRequest>, geometry: FrameGeometry) -> bool {
    ticket.as_ref().is_some_and(|ticket| {
        ticket.geometry == geometry
            && !ticket.canceled.load(Ordering::Acquire)
            && matches!(ticket.outcome.get(), Some(Ok(())))
    })
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
        signal(world, splash, "splash.ready", Session::ready)
    }

    pub fn finished(world: &mut World, splash: Entity) -> Result<bool, SplashFailure> {
        signal(world, splash, "splash.finished", Session::finish)
    }

    pub fn main_ready(world: &mut World, splash: Entity) -> Result<bool, SplashFailure> {
        signal(world, splash, "splash.main_ready", Session::main_ready)
    }

    pub fn cancel(world: &mut World, splash: Entity) -> Result<(), SplashFailure> {
        let Some(mut registry) = world.get_resource_mut::<Registry>() else {
            return Err(failure(SplashError::PluginMissing, "splash.cancel"));
        };
        let session = registry
            .sessions
            .remove(&splash)
            .ok_or_else(|| failure(SplashError::UnknownSplash { splash }, "splash.cancel"))?;
        session.cancel();
        WINIT_WINDOWS.with_borrow(|windows| {
            if let Some(native) = windows.get_window(splash) {
                native.set_visible(false);
            }
        });
        world.despawn(splash);
        Ok(())
    }
}

fn signal(
    world: &mut World,
    splash: Entity,
    operation: &'static str,
    submit: fn(&mut Session) -> bool,
) -> Result<bool, SplashFailure> {
    let Some(mut registry) = world.get_resource_mut::<Registry>() else {
        return Err(failure(SplashError::PluginMissing, operation));
    };
    let session = registry
        .sessions
        .get_mut(&splash)
        .ok_or_else(|| failure(SplashError::UnknownSplash { splash }, operation))?;
    Ok(submit(session))
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Registry>()
        .register_type::<SplashReady>()
        .register_type::<SplashFinished>()
        .register_type::<MainWindowReady>()
        .add_observer(on_ready)
        .add_observer(on_finished)
        .add_observer(on_main_ready)
        .add_observer(on_removed)
        .add_systems(PostUpdate, sync.in_set(crate::SplashSystems::Presentation));
}

fn on_ready(event: On<SplashReady>, mut commands: Commands) {
    queue_signal(&mut commands, event.splash, Splash::ready);
}

fn on_removed(event: On<Remove<crate::SplashWindow>>, mut registry: ResMut<Registry>) {
    if let Some(session) = registry.sessions.remove(&event.entity) {
        session.cancel();
    }
}

fn on_finished(event: On<SplashFinished>, mut commands: Commands) {
    queue_signal(&mut commands, event.splash, Splash::finished);
}

fn on_main_ready(event: On<MainWindowReady>, mut commands: Commands) {
    queue_signal(&mut commands, event.splash, Splash::main_ready);
}

fn queue_signal(
    commands: &mut Commands,
    splash: Entity,
    submit: fn(&mut World, Entity) -> Result<bool, SplashFailure>,
) {
    commands.queue(move |world: &mut World| -> Result {
        if !world
            .get_resource::<Registry>()
            .is_some_and(|registry| registry.sessions.contains_key(&splash))
        {
            return Ok(());
        }
        submit(world, splash)
            .map(|_| ())
            .map_err(|error| error.into_bevy_error())
    });
}

fn sync(world: &mut World) -> Result {
    use bevy_widgetry_diagnostics::bevy::DiagnosticBatch;
    let (shown, completed, mut failures) =
        world.resource_scope(|world, mut registry: Mut<Registry>| {
            let mut shown = Vec::new();
            let mut completed = Vec::new();
            let mut failures = DiagnosticBatch::default();
            for (&splash, session) in &mut registry.sessions {
                if session.halted {
                    continue;
                }
                if let Some(error) = session.poll_failure() {
                    failures.push_shared(error);
                    continue;
                }
                match advance(world, splash, session) {
                    Ok(Advance::Waiting) => {}
                    Ok(Advance::Shown) => shown.push(SplashShown {
                        splash,
                        main: session.main,
                    }),
                    Ok(Advance::Completed) => completed.push(SplashCompleted {
                        splash,
                        main: session.main,
                    }),
                    Err(errors) => {
                        session.halted = true;
                        for error in errors {
                            failures.push_shared(Arc::new(error));
                        }
                    }
                }
            }
            (shown, completed, failures)
        });
    for event in shown {
        world.trigger(event);
    }
    for event in completed {
        if world
            .resource::<Registry>()
            .sessions
            .contains_key(&event.splash)
        {
            match Splash::cancel(world, event.splash) {
                Ok(()) => world.trigger(event),
                Err(error) => failures.push_shared(Arc::new(error)),
            }
        }
    }
    match failures.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

enum Advance {
    Waiting,
    Shown,
    Completed,
}

fn frame_geometry(world: &World, window: Entity) -> Result<FrameGeometry, SplashFailure> {
    let target = world
        .get::<Window>(window)
        .ok_or_else(|| failure(SplashError::WindowMissing { window }, "splash.present"))?;
    Ok(FrameGeometry {
        window,
        size: target.physical_size(),
        scale: target.scale_factor(),
    })
}

pub(super) fn show_window(
    world: &mut World,
    window: Entity,
    focus: bool,
) -> Result<(), SplashFailure> {
    let native_shown = WINIT_WINDOWS.with_borrow(|windows| {
        let Some(native) = windows.get_window(window) else {
            return false;
        };
        native.set_visible(true);
        if focus {
            native.focus_window();
        }
        true
    });
    if !native_shown {
        return Err(failure(
            SplashError::WindowMissing { window },
            "splash.show",
        ));
    }
    if let Some(mut target) = world.get_mut::<Window>(window) {
        target.visible = true;
    }
    Ok(())
}

fn advance(
    world: &mut World,
    splash: Entity,
    session: &mut Session,
) -> Result<Advance, Vec<SplashFailure>> {
    let geometry = frame_geometry(world, splash).map_err(|error| vec![error])?;
    if session.handoff {
        let main = frame_geometry(world, session.main).map_err(|error| vec![error])?;
        prepare_ticket(&mut session.main_ticket, main);
        let visible = WINIT_WINDOWS
            .with_borrow(|windows| {
                windows
                    .get_window(session.main)
                    .map(|native| native.is_visible() == Some(true))
            })
            .ok_or_else(|| {
                failure(
                    SplashError::WindowMissing {
                        window: session.main,
                    },
                    "splash.handoff",
                )
            })
            .map_err(|error| vec![error])?;
        return Ok(
            if visible
                && session.handoff_ready(main)
                && world
                    .get::<Window>(session.main)
                    .is_some_and(|window| window.visible)
            {
                Advance::Completed
            } else {
                Advance::Waiting
            },
        );
    }
    if !session.visible {
        if world
            .get::<Window>(splash)
            .is_some_and(|window| window.visible)
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
            return Ok(Advance::Waiting);
        }
        session.prepare(geometry);
        if session.should_show(geometry) {
            show_window(world, splash, false).map_err(|error| vec![error])?;
            session.visible = true;
            return Ok(Advance::Shown);
        }
    } else if session.finished && session.main_ready {
        let main = frame_geometry(world, session.main).map_err(|error| vec![error])?;
        session.prepare_main(main);
        if session.can_handoff(main) {
            crate::native::show_main(world, session.main)?;
            session.begin_handoff(main);
        }
    }
    Ok(Advance::Waiting)
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
        .filter(|session| session.needs_wake())
    {
        pending.needs_wake = true;
        let ticket = if session.visible {
            &session.main_ticket
        } else {
            &session.ticket
        };
        let Some(ticket) = ticket.as_ref() else {
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
    fn handoff_requires_all_three_signals_and_the_current_main_frame_in_any_order() {
        let main = frame(Entity::from_bits(2), 640);
        let splash = frame(Entity::from_bits(1), 480);
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let mut session = Session::new(main.window);
            for signal in order {
                match signal {
                    0 => {
                        assert!(session.ready());
                    }
                    1 => {
                        assert!(session.finish());
                    }
                    _ => {
                        assert!(session.main_ready());
                    }
                }
                session.prepare_main(main);
                assert!(!session.can_handoff(main));
            }
            session.prepare(splash);
            session
                .ticket
                .as_ref()
                .unwrap()
                .outcome
                .set(Ok(()))
                .unwrap();
            session.visible = true;
            session.prepare_main(main);
            assert!(!session.can_handoff(main));
            session
                .main_ticket
                .as_ref()
                .unwrap()
                .outcome
                .set(Ok(()))
                .unwrap();
            assert!(session.can_handoff(main));
            assert!(!session.ready() && !session.finish() && !session.main_ready());
            assert!(session.can_handoff(main));
            let hidden_frame = session.main_ticket.as_ref().unwrap().clone();
            session.begin_handoff(main);
            assert_eq!(session.phase(), SplashPhase::HandingOff);
            assert!(!session.can_handoff(main));
            assert!(hidden_frame.canceled.load(Ordering::Acquire));
            assert!(!session.handoff_ready(main));
            session
                .main_ticket
                .as_ref()
                .unwrap()
                .outcome
                .set(Ok(()))
                .unwrap();
            assert!(session.handoff_ready(main));
        }
    }

    #[test]
    fn finished_splash_holds_until_main_readiness_and_rejects_stale_frames() {
        let mut session = Session::new(Entity::from_bits(2));
        session.ready();
        session.visible = true;
        assert_eq!(session.phase(), SplashPhase::Visible);
        session.finish();
        assert_eq!(session.phase(), SplashPhase::WaitingForMainContent);
        let main = frame(session.main, 640);
        session.prepare_main(main);
        assert!(session.main_ticket.is_none());
        session.main_ready();
        assert_eq!(session.phase(), SplashPhase::WaitingForMainFrame);
        session.prepare_main(main);
        let old = session.main_ticket.as_ref().unwrap().clone();
        let current = FrameGeometry { scale: 2.0, ..main };
        session.prepare_main(current);
        old.outcome.set(Ok(())).unwrap();
        assert!(old.canceled.load(Ordering::Acquire));
        assert!(!session.can_handoff(main) && !session.can_handoff(current));
        session
            .main_ticket
            .as_ref()
            .unwrap()
            .outcome
            .set(Ok(()))
            .unwrap();
        assert!(session.can_handoff(current));
        session.halted = true;
        assert!(!session.can_handoff(current));
        assert!(session.visible);
    }

    #[test]
    fn main_frame_snapshot_follows_scaling_in_the_same_update() {
        use bevy::window::WindowPosition;
        use bevy_widgetry_ui_scale::{WidgetryUiScale, WidgetryUiScalePlugin};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, WidgetryUiScalePlugin, crate::SplashPlugin));
        let main = app
            .world_mut()
            .spawn(Window {
                resolution: (640, 400).into(),
                position: WindowPosition::At(IVec2::ZERO),
                visible: false,
                ..default()
            })
            .id();
        let splash = Splash::start(app.world_mut(), main, Window::default()).unwrap();
        {
            let mut registry = app.world_mut().resource_mut::<Registry>();
            let session = registry.sessions.get_mut(&splash).unwrap();
            session.ready();
            session.visible = true;
            session.finish();
            session.main_ready();
        }
        app.world_mut()
            .resource_mut::<WidgetryUiScale>()
            .set(2.0)
            .unwrap();
        app.update();
        let geometry = app.world().resource::<Registry>().sessions[&splash]
            .main_ticket
            .as_ref()
            .unwrap()
            .geometry;
        assert_eq!(geometry, frame_geometry(app.world(), main).unwrap());
        assert_eq!(geometry.size, UVec2::new(1280, 800));
        assert_eq!(geometry.scale, 2.0);
        assert!(!app.world().get::<Window>(main).unwrap().visible);
    }

    #[test]
    fn main_frame_failure_keeps_the_splash_and_never_shows_the_main() {
        use bevy_widgetry_test_utils::ErrorCapture;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, crate::SplashPlugin))
            .set_error_handler(ErrorCapture::handler());
        let main = app
            .world_mut()
            .spawn(Window {
                visible: false,
                ..default()
            })
            .id();
        let splash = Splash::start(app.world_mut(), main, Window::default()).unwrap();
        app.world_mut().get_mut::<Window>(splash).unwrap().visible = true;
        let geometry = frame_geometry(app.world(), main).unwrap();
        let original = Arc::new(failure(
            SplashError::Presentation {
                window: main,
                source: winsafe::co::HRESULT::E_FAIL,
            },
            "splash.test.main_frame",
        ));
        {
            let mut registry = app.world_mut().resource_mut::<Registry>();
            let session = registry.sessions.get_mut(&splash).unwrap();
            session.ready();
            session.visible = true;
            session.finish();
            session.main_ready();
            session.prepare_main(geometry);
            session
                .main_ticket
                .as_ref()
                .unwrap()
                .outcome
                .set(Err(Arc::clone(&original)))
                .unwrap();
        }
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0]
                .managed()
                .unwrap()
                .diagnostics()
                .next()
                .unwrap()
                .origin()
                .id(),
            original.origin().id()
        );
        errors.run(|| app.update());
        assert!(errors.take_records().is_empty());
        assert!(!app.world().get::<Window>(main).unwrap().visible);
        assert!(app.world().get::<Window>(splash).unwrap().visible);
        Splash::cancel(app.world_mut(), splash).unwrap();
        assert!(!app.world().get::<Window>(main).unwrap().visible);
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
