use crate::{
    title_bar::bar::TitleBarDragArea,
    window_root::{WindowRoot, find_window_root},
};
use bevy::{
    ecs::entity::ContainsEntity,
    ecs::{
        hierarchy::ChildOf,
        observer::On,
        query::With,
        system::{Commands, Query},
    },
    picking::{events::PointerPress, pointer::PointerButton},
    window::Window,
};
use bevy_widgetry_cursor::CursorLease;

pub(super) fn on_title_bar_press(
    event: On<PointerPress>,
    drag_areas: Query<(), With<TitleBarDragArea>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut windows: Query<&mut Window>,
    mut commands: Commands,
) {
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
