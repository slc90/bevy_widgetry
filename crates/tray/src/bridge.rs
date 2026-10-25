use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent, TrayIconId};

const CAPACITY: usize = 64;
type Routes = Mutex<HashMap<TrayIconId, Weak<Bridge>>>;
static ROUTES: OnceLock<Routes> = OnceLock::new();

#[derive(Message, Clone, Copy, Debug)]
pub struct WidgetryTrayClick;

pub(crate) struct Bridge {
    clicks: Sender<()>,
    errors: Sender<BevyError>,
    wake: Box<dyn Fn() -> Result + Send + Sync>,
}

#[derive(Resource)]
pub(crate) struct Inbox {
    clicks: Receiver<()>,
    errors: Receiver<BevyError>,
}

pub(crate) struct Registration(TrayIconId);

pub(crate) fn install() {
    ROUTES.get_or_init(|| {
        TrayIconEvent::set_event_handler(Some(dispatch));
        Mutex::default()
    });
}

impl Bridge {
    pub(crate) fn new(wake: impl Fn() -> Result + Send + Sync + 'static) -> (Arc<Self>, Inbox) {
        let (clicks, click_rx) = bounded(CAPACITY);
        let (errors, error_rx) = bounded(1);
        (
            Arc::new(Self {
                clicks,
                errors,
                wake: Box::new(wake),
            }),
            Inbox {
                clicks: click_rx,
                errors: error_rx,
            },
        )
    }

    fn notify(&self) {
        match self.clicks.try_send(()) {
            Ok(()) => {}
            Err(TrySendError::Full(())) => {
                self.fail(BevyError::error("托盘点击队列已满，拒绝本次点击"));
                return;
            }
            Err(TrySendError::Disconnected(())) => return,
        }
        if let Err(error) = (self.wake)() {
            self.fail(error);
        }
    }

    fn fail(&self, error: BevyError) {
        let message = error.to_string();
        if self.errors.try_send(error).is_ok() {
            widgetry_error!(error = %message, "托盘点击桥接失败");
        }
    }
}

impl Registration {
    pub(crate) fn new(id: TrayIconId, bridge: &Arc<Bridge>) -> Result<Self> {
        install();
        if let Some(routes) = ROUTES.get() {
            routes
                .lock()
                .map_err(route_error)?
                .insert(id.clone(), Arc::downgrade(bridge));
        }
        Ok(Self(id))
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(routes) = ROUTES.get() {
            match routes.lock() {
                Ok(mut routes) => {
                    routes.remove(&self.0);
                }
                Err(error) => {
                    route_error(error);
                }
            }
        }
    }
}

fn route_error(error: impl std::fmt::Display) -> BevyError {
    widgetry_error!(error = %error, "托盘事件路由锁失败");
    BevyError::error(error.to_string())
}

fn dispatch(event: TrayIconEvent) {
    let TrayIconEvent::Click {
        id,
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    else {
        return;
    };
    let Some(routes) = ROUTES.get() else {
        return;
    };
    let bridge = match routes.lock() {
        Ok(routes) => routes.get(&id).and_then(Weak::upgrade),
        Err(error) => {
            route_error(error);
            return;
        }
    };
    if let Some(bridge) = bridge {
        bridge.notify();
    }
}

pub(crate) fn forward_clicks(
    inbox: Option<Res<Inbox>>,
    mut clicks: MessageWriter<WidgetryTrayClick>,
) -> Result {
    let Some(inbox) = inbox else {
        return Ok(());
    };
    for () in inbox.clicks.try_iter() {
        clicks.write(WidgetryTrayClick);
    }
    if let Ok(error) = inbox.errors.try_recv() {
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        let (bridge, inbox) = Bridge::new(move || {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        let id = TrayIconId::from(std::thread::current().name().unwrap_or("tray-test"));
        let registration = Registration::new(id.clone(), &bridge).unwrap();
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
        let (bridge, inbox) = Bridge::new(|| Ok(()));
        let mut app = App::new();
        app.add_message::<WidgetryTrayClick>()
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
    fn wake_failure_reaches_ecs_and_weak_route_does_not_retain_sender() {
        let (bridge, inbox) = Bridge::new(|| Err(BevyError::error("event loop closed")));
        let id = TrayIconId::from(std::thread::current().name().unwrap_or("tray-test"));
        let registration = Registration::new(id.clone(), &bridge).unwrap();
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
