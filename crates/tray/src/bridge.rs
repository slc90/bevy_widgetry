use crate::{TrayError, TrayFailure};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::DiagnosticBatch};
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tray_icon::menu::{MenuEvent, MenuId};
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent, TrayIconId};

const CAPACITY: usize = 64;
type Routes = Mutex<HashMap<TrayIconId, Weak<Bridge>>>;
static ROUTES: OnceLock<Routes> = OnceLock::new();
type MenuRoutes = Mutex<HashMap<MenuId, (Weak<Bridge>, String)>>;
static MENU_ROUTES: OnceLock<MenuRoutes> = OnceLock::new();

#[derive(Message, Clone, Copy, Debug)]
pub struct WidgetryTrayClick;

#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct WidgetryTrayMenuClick {
    pub id: String,
}

pub(crate) struct Bridge {
    id: TrayIconId,
    clicks: Sender<()>,
    menus: Sender<(MenuId, String)>,
    pub(crate) gesture: crate::native::MenuGesture,
    errors: Sender<TrayFailure>,
    wake: Box<dyn Fn() -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
}

#[derive(Resource)]
pub(crate) struct Inbox {
    clicks: Receiver<()>,
    menus: Receiver<(MenuId, String)>,
    errors: Receiver<TrayFailure>,
}

pub(crate) struct Registration(TrayIconId);
pub(crate) struct MenuRegistration(Vec<MenuId>);

pub(crate) fn install() {
    ROUTES.get_or_init(|| {
        TrayIconEvent::set_event_handler(Some(dispatch));
        Mutex::default()
    });
    MENU_ROUTES.get_or_init(|| {
        MenuEvent::set_event_handler(Some(dispatch_menu));
        Mutex::default()
    });
}

impl Bridge {
    pub(crate) fn new(
        id: TrayIconId,
        wake: impl Fn() -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync + 'static,
    ) -> (Arc<Self>, Inbox) {
        let (clicks, click_rx) = bounded(CAPACITY);
        let (menus, menu_rx) = bounded(CAPACITY);
        // Diagnostics must never wait for the event-loop thread that owns the receiver.
        let (errors, error_rx) = unbounded();
        (
            Arc::new(Self {
                id,
                clicks,
                menus,
                gesture: Default::default(),
                errors,
                wake: Box::new(wake),
            }),
            Inbox {
                clicks: click_rx,
                menus: menu_rx,
                errors: error_rx,
            },
        )
    }

    fn notify(&self) {
        match self.clicks.try_send(()) {
            Ok(()) => {}
            Err(TrySendError::Full(())) => {
                self.fail(TrayFailure::new(
                    TrayError::ClickOverflow {
                        id: self.id.clone(),
                        capacity: CAPACITY,
                    },
                    self.frame("enqueue click"),
                ));
                return;
            }
            Err(TrySendError::Disconnected(())) => return,
        }
        if let Err(error) = (self.wake)() {
            self.fail(TrayFailure::new(
                TrayError::Wake {
                    id: self.id.clone(),
                    source: error,
                },
                self.frame("wake event loop"),
            ));
        }
    }

    fn frame(&self, operation: &'static str) -> ErrorFrame {
        ErrorFrame::new(operation).field("tray", format_args!("{:?}", self.id))
    }

    pub(crate) fn fail(&self, error: TrayFailure) {
        if let Err(disconnected) = self.errors.send(error) {
            disconnected
                .0
                .record_handled("停止托盘点击递送", "ECS 收件箱已释放，放弃本次点击操作");
        }
    }

    fn notify_menu(&self, native: MenuId, id: String) {
        match self.menus.try_send((native, id)) {
            Ok(()) => {}
            Err(TrySendError::Full((_, id))) => {
                self.fail(TrayFailure::new(
                    TrayError::MenuOverflow {
                        id,
                        capacity: CAPACITY,
                    },
                    self.frame("queue tray menu activation"),
                ));
                return;
            }
            Err(TrySendError::Disconnected(_)) => return,
        }
        if let Err(source) = (self.wake)() {
            self.fail(TrayFailure::new(
                TrayError::Wake {
                    id: self.id.clone(),
                    source,
                },
                self.frame("wake menu event loop"),
            ));
        }
    }
}

impl MenuRegistration {
    pub(crate) fn new(items: &[(MenuId, String)], bridge: &Arc<Bridge>) -> Self {
        install();
        with_menu_routes(|routes| {
            for (native, id) in items {
                routes.insert(native.clone(), (Arc::downgrade(bridge), id.clone()));
            }
        });
        Self(items.iter().map(|(id, _)| id.clone()).collect())
    }
}

impl Drop for MenuRegistration {
    fn drop(&mut self) {
        with_menu_routes(|routes| {
            for id in &self.0 {
                routes.remove(id);
            }
        });
    }
}

fn with_menu_routes<T>(
    action: impl FnOnce(&mut HashMap<MenuId, (Weak<Bridge>, String)>) -> T,
) -> T {
    install();
    let routes = MENU_ROUTES.get_or_init(Mutex::default);
    let (mut guard, failure) = match routes.lock() {
        Ok(guard) => (guard, None),
        Err(poisoned) => (
            poisoned.into_inner(),
            Some(TrayFailure::new(
                TrayError::RouteLock,
                ErrorFrame::new("access menu routes"),
            )),
        ),
    };
    let result = action(&mut guard);
    if let Some(failure) = failure {
        routes.clear_poison();
        failure.record_handled("恢复托盘菜单路由锁访问", "已完成本次路由操作并清除中毒标记");
    }
    result
}

fn dispatch_menu(event: MenuEvent) {
    let route = with_menu_routes(|routes| {
        routes
            .get(&event.id)
            .and_then(|(bridge, id)| Some((bridge.upgrade()?, id.clone())))
    });
    if let Some((bridge, id)) = route {
        bridge.notify_menu(event.id, id);
    }
}

impl Registration {
    pub(crate) fn new(id: TrayIconId, bridge: &Arc<Bridge>) -> Self {
        install();
        if let Some(routes) = ROUTES.get() {
            with_routes(routes, &id, "register route", |routes| {
                routes.insert(id.clone(), Arc::downgrade(bridge));
            });
        }
        Self(id)
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(routes) = ROUTES.get() {
            with_routes(routes, &self.0, "remove route", |routes| {
                routes.remove(&self.0);
            });
        }
    }
}

fn with_routes<T>(
    routes: &Routes,
    id: &TrayIconId,
    operation: &'static str,
    action: impl FnOnce(&mut HashMap<TrayIconId, Weak<Bridge>>) -> T,
) -> T {
    let (mut guard, failure) = match routes.lock() {
        Ok(guard) => (guard, None),
        Err(poisoned) => (
            poisoned.into_inner(),
            Some(TrayFailure::new(
                TrayError::RouteLock,
                ErrorFrame::new(operation).field("tray", format_args!("{id:?}")),
            )),
        ),
    };
    // HashMap remains valid after unwinding. No user callbacks execute while it is locked.
    let result = action(&mut guard);
    if let Some(failure) = failure {
        routes.clear_poison();
        failure.record_handled("恢复托盘路由锁访问", "已完成本次路由操作并清除中毒标记");
    }
    result
}

fn dispatch(event: TrayIconEvent) {
    let (id, state) = match event {
        TrayIconEvent::Click {
            id,
            button: MouseButton::Left,
            button_state,
            ..
        } => (id, button_state),
        TrayIconEvent::DoubleClick {
            id,
            button: MouseButton::Left,
            ..
        } => (id, MouseButtonState::Down),
        _ => return,
    };
    let Some(routes) = ROUTES.get() else {
        return;
    };
    let bridge = with_routes(routes, &id, "dispatch click", |routes| {
        routes.get(&id).and_then(Weak::upgrade)
    });
    if let Some(bridge) = bridge
        && bridge.gesture.tray_input(state)
    {
        bridge.notify();
    }
}

pub(crate) fn forward_clicks(
    inbox: Option<Res<Inbox>>,
    mut clicks: MessageWriter<WidgetryTrayClick>,
    mut menus: MessageWriter<WidgetryTrayMenuClick>,
) -> Result {
    let Some(inbox) = inbox else {
        return Ok(());
    };
    for () in inbox.clicks.try_iter().take(CAPACITY) {
        clicks.write(WidgetryTrayClick);
    }
    for (native, id) in inbox.menus.try_iter().take(CAPACITY) {
        if with_menu_routes(|routes| routes.contains_key(&native)) {
            menus.write(WidgetryTrayMenuClick { id });
        }
    }
    forward_errors(&inbox)
}

pub(crate) fn forward_errors(inbox: &Inbox) -> Result {
    let mut errors = DiagnosticBatch::default();
    // Bound this drain to the current backlog so concurrent callbacks cannot starve ECS.
    for error in inbox.errors.try_iter().take(inbox.errors.len()) {
        errors.push(error);
    }
    errors.into_bevy_error().map_or(Ok(()), Err)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn callback_thread_preserves_every_origin_and_source_through_host_boundary() {
        // Isolate callsite-interest updates from other tests' thread-local dispatchers.
        const CHILD: &str = "WIDGETRY_TRAY_CALLBACK_CAPTURE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "bridge::tests::callback_thread_preserves_every_origin_and_source_through_host_boundary",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        use bevy::ecs::schedule::SingleThreadedExecutor;
        use std::error::Error;
        let logs = LogCapture::default();
        let dispatcher = logs.dispatcher();
        let (bridge, inbox) = Bridge::new(TrayIconId::from("thread"), || Ok(()));
        let origins = std::thread::spawn(move || {
            bevy::log::tracing::dispatcher::with_default(&dispatcher, || {
                (0..8)
                    .map(|_| {
                        let failure = TrayFailure::new(
                            TrayError::Wake {
                                id: bridge.id.clone(),
                                source: Box::new(std::io::Error::from_raw_os_error(5)),
                            },
                            bridge.frame("callback failure"),
                        );
                        let origin = failure.origin().clone();
                        bridge.fail(failure);
                        origin
                    })
                    .collect::<Vec<_>>()
            })
        })
        .join()
        .unwrap();
        let mut app = App::new();
        app.add_message::<WidgetryTrayClick>()
            .add_message::<WidgetryTrayMenuClick>()
            .insert_resource(inbox)
            .add_systems(PreUpdate, forward_clicks);
        app.edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::default());
        });
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        errors.run(|| logs.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostics: Vec<_> = records[0].managed().unwrap().diagnostics().collect();
        assert_eq!(diagnostics.len(), origins.len());
        for (diagnostic, origin) in diagnostics.into_iter().zip(origins) {
            assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
            assert_eq!(
                origin.backtrace().status(),
                std::backtrace::BacktraceStatus::Captured
            );
            let kind = diagnostic
                .source()
                .unwrap()
                .downcast_ref::<TrayError>()
                .unwrap();
            assert_eq!(
                kind.source()
                    .unwrap()
                    .downcast_ref::<std::io::Error>()
                    .unwrap()
                    .raw_os_error(),
                Some(5)
            );
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
        errors.run(|| app.update());
        assert!(errors.take_records().is_empty());
    }

    #[test]
    fn overloaded_callback_thread_retains_each_rejected_operation() {
        let (bridge, inbox) = Bridge::new(TrayIconId::from("overloaded"), || Ok(()));
        std::thread::spawn(move || {
            for _ in 0..CAPACITY + 128 {
                bridge.notify();
            }
        })
        .join()
        .unwrap();
        assert_eq!(inbox.clicks.len(), CAPACITY);
        let failures: Vec<_> = inbox.errors.try_iter().collect();
        assert_eq!(failures.len(), 128);
        let ids: std::collections::HashSet<_> = failures
            .iter()
            .map(|failure| failure.origin().id())
            .collect();
        assert_eq!(ids.len(), 128);
        assert!(failures.iter().all(|failure| matches!(
            failure.kind(),
            TrayError::ClickOverflow {
                capacity: CAPACITY,
                ..
            }
        )));
    }

    #[test]
    fn poisoned_route_is_recovered_and_only_then_marked_handled() {
        let routes = Arc::new(Routes::default());
        let poisoned = routes.clone();
        let poisoning = std::thread::spawn(move || {
            let _guard = poisoned.lock().unwrap();
            panic!("poison local routes");
        })
        .join();
        assert!(poisoning.is_err());
        let logs = LogCapture::default();
        let id = TrayIconId::from("poisoned");
        logs.run(|| {
            assert_eq!(
                with_routes(&routes, &id, "recover test", |routes| routes.len()),
                0
            );
        });
        assert!(!routes.is_poisoned());
        let records = logs.records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].phase(), Some(DiagnosticPhase::Source));
        assert_eq!(records[1].phase(), Some(DiagnosticPhase::Handled));
        assert_eq!(records[0].fields["error_id"], records[1].fields["error_id"]);
        logs.run(|| {
            with_routes(&routes, &id, "healthy", |_| ());
        });
        assert_eq!(logs.records().len(), 2);
    }

    #[test]
    fn consecutive_wake_failures_are_retained_without_waiting_for_ecs() {
        let (bridge, inbox) = Bridge::new(TrayIconId::from("failures"), || {
            Err(Box::new(std::io::Error::other("event loop closed")))
        });
        for _ in 0..8 {
            bridge.notify();
        }
        assert_eq!(inbox.clicks.len(), 8);
        assert_eq!(inbox.errors.len(), 8);
    }

    fn click(
        id: &TrayIconId,
        button: MouseButton,
        button_state: MouseButtonState,
    ) -> TrayIconEvent {
        TrayIconEvent::Click {
            id: id.clone(),
            position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
            rect: tray_icon::Rect {
                position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
                size: tray_icon::dpi::PhysicalSize::new(16, 16),
            },
            button,
            button_state,
        }
    }

    #[test]
    fn routes_only_own_left_release_and_drops_expired_routes() {
        let wakes = Arc::new(AtomicUsize::new(0));
        let calls = wakes.clone();
        let (bridge, inbox) = Bridge::new(TrayIconId::from("healthy"), move || {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        let id = TrayIconId::from(std::thread::current().name().unwrap_or("tray-test"));
        let registration = Registration::new(id.clone(), &bridge);
        dispatch(click(&id, MouseButton::Left, MouseButtonState::Down));
        dispatch(click(&id, MouseButton::Right, MouseButtonState::Up));
        dispatch(TrayIconEvent::DoubleClick {
            id: id.clone(),
            position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
            rect: tray_icon::Rect {
                position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
                size: tray_icon::dpi::PhysicalSize::new(16, 16),
            },
            button: MouseButton::Left,
        });
        dispatch(click(
            &TrayIconId::from("other-icon"),
            MouseButton::Left,
            MouseButtonState::Up,
        ));
        assert!(inbox.clicks.is_empty());
        dispatch(click(&id, MouseButton::Left, MouseButtonState::Up));
        assert_eq!(inbox.clicks.len(), 1);
        assert_eq!(wakes.load(Ordering::Relaxed), 1);
        drop(registration);
        dispatch(click(&id, MouseButton::Left, MouseButtonState::Up));
        assert_eq!(inbox.clicks.len(), 1);
    }

    #[test]
    fn bounded_clicks_forward_once_without_subscribers_and_report_overflow() {
        let (bridge, inbox) = Bridge::new(TrayIconId::from("overflow"), || Ok(()));
        let mut app = App::new();
        app.add_message::<WidgetryTrayClick>()
            .add_message::<WidgetryTrayMenuClick>()
            .insert_resource(inbox)
            .add_systems(PreUpdate, forward_clicks);
        for _ in 0..CAPACITY {
            bridge.notify();
        }
        bridge.notify();
        let mut cursor = app
            .world()
            .resource::<Messages<WidgetryTrayClick>>()
            .get_cursor();
        use bevy::ecs::system::RunSystemOnce;
        let result = app
            .world_mut()
            .run_system_once::<_, Result, _>(forward_clicks)
            .unwrap();
        assert!(result.is_err());
        assert_eq!(
            cursor
                .read(app.world().resource::<Messages<WidgetryTrayClick>>())
                .count(),
            CAPACITY
        );
        assert!(
            app.world_mut()
                .run_system_once::<_, Result, _>(forward_clicks)
                .unwrap()
                .is_ok()
        );
        assert_eq!(
            cursor
                .read(app.world().resource::<Messages<WidgetryTrayClick>>())
                .count(),
            0
        );
        bridge.notify();
        assert!(
            app.world_mut()
                .run_system_once::<_, Result, _>(forward_clicks)
                .unwrap()
                .is_ok()
        );
        assert_eq!(
            cursor
                .read(app.world().resource::<Messages<WidgetryTrayClick>>())
                .count(),
            1
        );
    }

    #[test]
    fn menu_routes_isolate_instances_and_discard_retired_queued_events() {
        use bevy::ecs::system::RunSystemOnce;
        let wakes = Arc::new(AtomicUsize::new(0));
        let calls = wakes.clone();
        let (bridge, inbox) = Bridge::new(TrayIconId::from("menu-a"), move || {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        let (other, other_inbox) = Bridge::new(TrayIconId::from("menu-b"), || Ok(()));
        let first = tray_icon::menu::MenuItem::new("first", true, None)
            .id()
            .clone();
        let second = tray_icon::menu::MenuItem::new("second", true, None)
            .id()
            .clone();
        let registration = MenuRegistration::new(&[(first.clone(), "shared".into())], &bridge);
        let _other_registration =
            MenuRegistration::new(&[(second.clone(), "shared".into())], &other);
        dispatch_menu(MenuEvent { id: first.clone() });
        dispatch_menu(MenuEvent { id: first.clone() });
        dispatch_menu(MenuEvent {
            id: MenuId::new("unknown"),
        });
        assert_eq!(wakes.load(Ordering::Relaxed), 2);
        assert!(other_inbox.menus.is_empty());
        let mut app = App::new();
        app.add_message::<WidgetryTrayClick>()
            .add_message::<WidgetryTrayMenuClick>()
            .insert_resource(inbox);
        let mut cursor = app
            .world()
            .resource::<Messages<WidgetryTrayMenuClick>>()
            .get_cursor();
        app.world_mut()
            .run_system_once::<_, Result, _>(forward_clicks)
            .unwrap()
            .unwrap();
        let messages = app.world().resource::<Messages<WidgetryTrayMenuClick>>();
        assert_eq!(
            cursor
                .read(messages)
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            ["shared", "shared"]
        );
        dispatch_menu(MenuEvent { id: first.clone() });
        drop(registration);
        dispatch_menu(MenuEvent { id: first });
        app.world_mut()
            .run_system_once::<_, Result, _>(forward_clicks)
            .unwrap()
            .unwrap();
        assert_eq!(
            cursor
                .read(app.world().resource::<Messages<WidgetryTrayMenuClick>>())
                .count(),
            0
        );
        dispatch_menu(MenuEvent { id: second });
        assert_eq!(other_inbox.menus.len(), 1);
    }

    #[test]
    fn menu_overload_and_wake_errors_preserve_each_failure() {
        let (bridge, inbox) = Bridge::new(TrayIconId::from("menu-errors"), || {
            Err(Box::new(std::io::Error::from_raw_os_error(5)))
        });
        for _ in 0..CAPACITY + 3 {
            bridge.notify_menu(MenuId::new("native"), "action".into());
        }
        assert_eq!(inbox.menus.len(), CAPACITY);
        let errors: Vec<_> = inbox.errors.try_iter().collect();
        assert_eq!(errors.len(), CAPACITY + 3);
        assert_eq!(
            errors
                .iter()
                .filter(|failure| matches!(failure.kind(), TrayError::MenuOverflow { .. }))
                .count(),
            3
        );
        use std::error::Error;
        assert_eq!(
            errors[0]
                .kind()
                .source()
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .raw_os_error(),
            Some(5)
        );
    }

    #[test]
    fn wake_failure_reaches_ecs_and_weak_route_does_not_retain_sender() {
        let (bridge, inbox) = Bridge::new(TrayIconId::from("wake"), || {
            Err(Box::new(std::io::Error::other("event loop closed")))
        });
        let id = TrayIconId::from(std::thread::current().name().unwrap_or("tray-test"));
        let registration = Registration::new(id.clone(), &bridge);
        dispatch(click(&id, MouseButton::Left, MouseButtonState::Up));
        assert_eq!(inbox.clicks.len(), 1);
        assert!(
            inbox
                .errors
                .try_recv()
                .unwrap()
                .to_string()
                .contains("event loop closed")
        );
        let weak = Arc::downgrade(&bridge);
        drop(bridge);
        assert!(weak.upgrade().is_none());
        dispatch(click(&id, MouseButton::Left, MouseButtonState::Up));
        assert_eq!(inbox.clicks.len(), 1);
        drop(registration);
    }
}
