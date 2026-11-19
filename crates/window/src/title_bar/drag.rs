use super::{
    bar::TitleBarDragArea,
    double_click::{TitleBarClicks, TitleBarDoubleClick},
};
use crate::{
    action::queue_toggle_maximize,
    window_root::{WindowRoot, find_window_root},
};
use bevy::prelude::Entity;
use bevy::{
    ecs::entity::ContainsEntity,
    ecs::{
        hierarchy::ChildOf,
        observer::On,
        query::{Has, With},
        system::{Commands, Local, NonSendMarker, Query},
    },
    picking::{events::PointerPress, pointer::PointerButton},
    window::Window,
};
use bevy_widgetry_cursor::CursorLease;

pub(super) fn on_title_bar_press(
    event: On<PointerPress>,
    _non_send_marker: NonSendMarker,
    drag_areas: Query<Has<TitleBarDoubleClick>, With<TitleBarDragArea>>,
    parents: Query<&ChildOf>,
    roots: Query<(Entity, &WindowRoot)>,
    mut windows: Query<&mut Window>,
    mut commands: Commands,
    mut clicks: Local<TitleBarClicks>,
) {
    if event.entity != event.original_event_target()
        || event.pointer.id != bevy::picking::pointer::PointerId::Mouse
    {
        return;
    }

    let mut previous_clicks = std::mem::take(&mut *clicks);
    if event.button != PointerButton::Primary {
        return;
    }

    let Ok(double_click) = drag_areas.get(event.entity) else {
        return;
    };

    let Some((root_entity, root)) = find_window_root(event.entity, &parents, &roots) else {
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

    if double_click && window.enabled_buttons.maximize {
        let toggle = previous_clicks.register_press(root.target_window, event.entity);
        *clicks = previous_clicks;
        if toggle {
            queue_toggle_maximize(&mut commands, root_entity);
            return;
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
    fn title_bar_double_click_can_be_omitted_without_disabling_native_drag() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig {
                    double_click_maximize: false, ..default()
                }, WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{})
            })
            .unwrap()
            .id();
        app.update();
        let window = app.world().get::<WindowRoot>(root).unwrap().target_window;
        let area = app
            .world_mut()
            .query_filtered::<Entity, With<TitleBarDragArea>>()
            .single(app.world())
            .unwrap();
        assert!(app.world().get::<TitleBarDoubleClick>(area).is_none());
        for count in [1, 2] {
            let mut event = primary_press(area);
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
            event.count = count;
            app.world_mut().trigger(event);
            let mut native = app.world_mut().get_mut::<Window>(window).unwrap();
            assert!(native.internal.take_move_request());
            assert_eq!(native.internal.take_maximize_request(), None);
        }
    }

    #[test]
    fn paired_native_presses_toggle_maximize_without_requesting_a_second_move() {
        use super::super::double_click::TEST_PRESS_TIME;
        use crate::action::TEST_NATIVE_MAXIMIZED;
        use std::time::{Duration, Instant};

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
        let now = Instant::now();
        for (millis, actual_maximized, moves, request) in [
            (0, false, true, None),
            (100, false, false, Some(true)),
            (200, true, true, None),
            (300, true, false, Some(false)),
        ] {
            let mut event = primary_press(area);
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
            TEST_PRESS_TIME.set(Some(now + Duration::from_millis(millis)));
            if request.is_some() {
                TEST_NATIVE_MAXIMIZED.set(Some((window, actual_maximized)));
            }
            app.world_mut().trigger(event);
            app.world_mut().flush();
            assert!(TEST_PRESS_TIME.take().is_none());
            assert!(TEST_NATIVE_MAXIMIZED.take().is_none());
            let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
            assert_eq!(window.internal.take_move_request(), moves);
            assert_eq!(window.internal.take_maximize_request(), request);
        }
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
        assert!(app.world().get::<TitleBarDoubleClick>(area).is_some());
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
