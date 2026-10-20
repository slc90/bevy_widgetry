use crate::{
    title_bar::bar::TitleBarDragArea,
    window_root::{WindowRoot, find_window_root},
};
use bevy::prelude::{Entity, IVec2};
use bevy::{
    ecs::entity::ContainsEntity,
    ecs::{
        hierarchy::ChildOf,
        observer::On,
        query::With,
        system::{Commands, Local, NonSendMarker, Query},
    },
    picking::{events::PointerPress, pointer::PointerButton},
    window::Window,
};
use bevy_widgetry_cursor::CursorLease;
use bevy_widgetry_internal_log::widgetry_warn;
use std::time::{Duration, Instant};

struct TitleBarPress {
    window: Entity,
    area: Entity,
    time: Instant,
    position: IVec2,
}

struct DoubleClickSettings {
    interval: Duration,
    half_extent: IVec2,
}

#[derive(Default)]
pub(super) struct TitleBarClicks {
    previous: Option<TitleBarPress>,
}

impl TitleBarClicks {
    fn is_double_click(&mut self, current: TitleBarPress, settings: DoubleClickSettings) -> bool {
        let matched = self.previous.take().is_some_and(|previous| {
            let dx = i64::from(current.position.x) - i64::from(previous.position.x);
            let dy = i64::from(current.position.y) - i64::from(previous.position.y);
            let half_x = i64::from(settings.half_extent.x);
            let half_y = i64::from(settings.half_extent.y);
            previous.window == current.window
                && previous.area == current.area
                && current.time.saturating_duration_since(previous.time) <= settings.interval
                && (-half_x..half_x).contains(&dx)
                && (-half_y..half_y).contains(&dy)
        });
        if !matched {
            self.previous = Some(current);
        }
        matched
    }
}

impl DoubleClickSettings {
    fn system() -> Self {
        Self {
            interval: Duration::from_millis(u64::from(winsafe::GetDoubleClickTime())),
            // Windows reports the full rectangle dimensions, centered on the first press.
            half_extent: IVec2::new(
                winsafe::GetSystemMetrics(winsafe::co::SM::CXDOUBLECLK) / 2,
                winsafe::GetSystemMetrics(winsafe::co::SM::CYDOUBLECLK) / 2,
            ),
        }
    }
}

pub(super) fn on_title_bar_press(
    event: On<PointerPress>,
    _non_send_marker: NonSendMarker,
    drag_areas: Query<(), With<TitleBarDragArea>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut windows: Query<&mut Window>,
    mut commands: Commands,
    mut clicks: Local<TitleBarClicks>,
) {
    if event.entity != event.original_event_target()
        || event.pointer.id != bevy::picking::pointer::PointerId::Mouse
    {
        return;
    }

    let previous = clicks.previous.take();
    if event.button != PointerButton::Primary {
        return;
    }

    if drag_areas.get(event.entity).is_err() {
        return;
    }

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };
    let Some(lease) =
        CursorLease::native(event.pointer.id, &event.pointer.location(), event.button)
    else {
        return;
    };
    if !matches!(event.pointer.target, bevy::camera::NormalizedRenderTarget::Window(target) if target.entity() == root.target_window)
    {
        return;
    }

    let Ok(mut window) = windows.get_mut(root.target_window) else {
        return;
    };

    if window.enabled_buttons.maximize {
        match winsafe::GetCursorPos() {
            Ok(position) => {
                clicks.previous = previous;
                if clicks.is_double_click(
                    TitleBarPress {
                        window: root.target_window,
                        area: event.entity,
                        time: Instant::now(),
                        position: IVec2::new(position.x, position.y),
                    },
                    DoubleClickSettings::system(),
                ) {
                    super::maximize::toggle_maximize(root.target_window, &mut window);
                    return;
                }
            }
            Err(error) => {
                widgetry_warn!(entity = ?root.target_window, error = %error, "无法读取鼠标屏幕坐标，本次标题栏按下仅执行拖动");
            }
        }
    }

    window.start_drag_move();
    commands.entity(event.entity).insert(lease);
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {

    use super::*;
    use crate::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
        owned_widgetry_window,
    };
    use bevy::{
        picking::{
            events::Pointer,
            pointer::{Location, PointerId},
        },
        prelude::*,
    };
    use bevy_widgetry_test_utils::{primary_press, scene_app};

    fn press(window: Entity, area: Entity, time: Instant, x: i32, y: i32) -> TitleBarPress {
        TitleBarPress {
            window,
            area,
            time,
            position: IVec2::new(x, y),
        }
    }

    fn settings() -> DoubleClickSettings {
        DoubleClickSettings {
            interval: Duration::from_millis(500),
            half_extent: IVec2::new(2, 3),
        }
    }

    #[test]
    fn double_click_consumes_pairs_and_uses_screen_coordinates() {
        let window = Entity::from_bits(1);
        let area = Entity::from_bits(2);
        let now = Instant::now();
        let mut clicks = TitleBarClicks::default();
        for (millis, expected) in [(0, false), (100, true), (200, false), (300, true)] {
            assert_eq!(
                clicks.is_double_click(
                    press(window, area, now + Duration::from_millis(millis), -100, -50),
                    settings()
                ),
                expected
            );
        }
    }

    #[test]
    fn double_click_obeys_windows_time_and_rectangle_boundaries() {
        let window = Entity::from_bits(1);
        let area = Entity::from_bits(2);
        let now = Instant::now();
        for (millis, x, y, expected) in [
            (500, 0, 0, true),
            (501, 0, 0, false),
            (100, -2, -3, true),
            (100, 1, 2, true),
            (100, 2, 0, false),
            (100, -3, 0, false),
            (100, 0, 3, false),
            (100, 0, -4, false),
            (100, i32::MAX, i32::MIN, false),
        ] {
            let mut clicks = TitleBarClicks::default();
            assert!(!clicks.is_double_click(press(window, area, now, 0, 0), settings()));
            assert_eq!(
                clicks.is_double_click(
                    press(window, area, now + Duration::from_millis(millis), x, y),
                    settings()
                ),
                expected,
                "{millis}ms ({x}, {y})"
            );
        }
    }

    #[test]
    fn double_click_requires_the_same_window_and_drag_area() {
        let window = Entity::from_bits(1);
        let area = Entity::from_bits(2);
        let other = Entity::from_bits(3);
        let now = Instant::now();
        for (second_window, second_area) in [(other, area), (window, other)] {
            let mut clicks = TitleBarClicks::default();
            assert!(!clicks.is_double_click(press(window, area, now, 0, 0), settings()));
            assert!(!clicks.is_double_click(
                press(
                    second_window,
                    second_area,
                    now + Duration::from_millis(100),
                    0,
                    0
                ),
                settings()
            ));
            assert!(clicks.is_double_click(
                press(
                    second_window,
                    second_area,
                    now + Duration::from_millis(200),
                    0,
                    0
                ),
                settings()
            ));
        }
    }

    #[test]
    fn disabling_maximize_preserves_drag_and_custom_controls_keep_their_input() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app.world_mut().spawn_scene(bsn! {
            @owned_widgetry_window(Window { enabled_buttons: bevy::window::EnabledButtons { maximize: false, ..default() }, ..default() }, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{Name::new("CustomTitleControl") Node::default()}, bsn_list!{})
        }).unwrap().id();
        app.update();
        let window = app.world().get::<WindowRoot>(root).unwrap().target_window;
        let area = app
            .world_mut()
            .query_filtered::<Entity, With<TitleBarDragArea>>()
            .single(app.world())
            .unwrap();
        let control = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "CustomTitleControl")
            .unwrap()
            .0;
        let presses = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = presses.clone();
        app.world_mut()
            .entity_mut(control)
            .observe(move |_: On<PointerPress>| {
                observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            });
        for (entity, button, count, moves) in [
            (area, PointerButton::Primary, 1, true),
            (area, PointerButton::Primary, 2, true),
            (area, PointerButton::Primary, 3, true),
            (area, PointerButton::Secondary, 1, false),
            (control, PointerButton::Primary, 1, false),
            (control, PointerButton::Primary, 2, false),
        ] {
            let mut event = primary_press(entity);
            event.pointer = Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(
                        window,
                    ))
                    .normalize(None)
                    .unwrap(),
                    position: Vec2::ONE,
                },
            );
            event.button = button;
            event.count = count;
            app.world_mut().trigger(event);
            let mut native = app.world_mut().get_mut::<Window>(window).unwrap();
            assert_eq!(native.internal.take_move_request(), moves);
            assert_eq!(native.internal.take_maximize_request(), None);
        }
        assert_eq!(presses.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert!(app.world().get::<CursorLease>(control).is_none());
    }

    #[test]
    fn native_move_uses_the_mouse_and_bound_window() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app.world_mut().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{})
        }).unwrap().id();
        app.update();
        let window = app.world().get::<WindowRoot>(root).unwrap().target_window;
        let area = app
            .world_mut()
            .query_filtered::<Entity, With<TitleBarDragArea>>()
            .single(app.world())
            .unwrap();
        let other = app.world_mut().spawn(Window::default()).id();
        for (id, target, accepted) in [
            (bevy_widgetry_test_utils::pointer_ids()[1], window, false),
            (PointerId::Mouse, other, false),
            (PointerId::Mouse, window, true),
        ] {
            let mut event = primary_press(area);
            event.pointer = Pointer::new(
                id,
                Location {
                    target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(
                        target,
                    ))
                    .normalize(None)
                    .unwrap(),
                    position: Vec2::ONE,
                },
            );
            app.world_mut().trigger(event);
            app.world_mut().flush();
            assert_eq!(
                app.world_mut()
                    .get_mut::<Window>(window)
                    .unwrap()
                    .internal
                    .take_move_request(),
                accepted
            );
            assert_eq!(app.world().get::<CursorLease>(area).is_some(), accepted);
        }
    }
}
