#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    camera::{NormalizedRenderTarget, RenderTarget},
    ecs::entity::EntityHashMap,
    picking::{
        backend::HitData,
        hover::{DirectlyHovered, HoverMap, Hovered},
        pointer::*,
    },
    prelude::*,
    window::WindowRef,
};
use bevy_widgetry_core::pointer::WidgetryPointerPlugin;
use bevy_widgetry_test_utils::{pointer_ids, scene_app};

#[derive(Resource, Default)]
struct Changes(usize);

fn count_changes(
    changed: Query<(), Or<(Changed<Hovered>, Changed<DirectlyHovered>)>>,
    mut count: ResMut<Changes>,
) {
    count.0 += changed.iter().count();
}

#[test]
fn shared_hover_projects_live_hits_and_is_quiet_until_context_changes() {
    for id in pointer_ids() {
        let mut app = scene_app();
        if !app.is_plugin_added::<WidgetryPointerPlugin>() {
            app.add_plugins(WidgetryPointerPlugin);
        }
        app.init_resource::<HoverMap>()
            .init_resource::<Changes>()
            .add_systems(Update, count_changes);
        let window = app.world_mut().spawn(Window::default()).id();
        let location = Location {
            target: RenderTarget::Window(WindowRef::Entity(window))
                .normalize(None)
                .unwrap(),
            position: Vec2::ONE,
        };
        let pointer = app
            .world_mut()
            .spawn((id, PointerLocation::new(location.clone())))
            .id();
        let parent = app
            .world_mut()
            .spawn((Hovered(false), DirectlyHovered(false)))
            .id();
        let other = app
            .world_mut()
            .spawn((Hovered(false), DirectlyHovered(false)))
            .id();
        let child = app
            .world_mut()
            .spawn((Hovered(false), DirectlyHovered(false), ChildOf(parent)))
            .id();
        let mut hits = EntityHashMap::default();
        hits.insert(child, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
        app.world_mut().resource_mut::<HoverMap>().insert(id, hits);
        app.update();
        assert!(app.world().get::<Hovered>(parent).unwrap().0);
        assert!(!app.world().get::<DirectlyHovered>(parent).unwrap().0);
        assert!(app.world().get::<DirectlyHovered>(child).unwrap().0);
        let changes = app.world().resource::<Changes>().0;
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().resource::<Changes>().0, changes);
        app.world_mut().entity_mut(child).insert(ChildOf(other));
        app.update();
        assert!(!app.world().get::<Hovered>(parent).unwrap().0);
        assert!(app.world().get::<Hovered>(other).unwrap().0);
        app.world_mut()
            .get_mut::<PointerLocation>(pointer)
            .unwrap()
            .location = None;
        app.update();
        assert!(!app.world().get::<Hovered>(other).unwrap().0);
        assert!(!app.world().get::<DirectlyHovered>(child).unwrap().0);
        app.world_mut()
            .get_mut::<PointerLocation>(pointer)
            .unwrap()
            .location = Some(location);
        app.update();
        assert!(app.world().get::<Hovered>(other).unwrap().0);
        app.world_mut().despawn(window);
        app.update();
        assert!(!app.world().get::<Hovered>(other).unwrap().0);
        app.world_mut()
            .get_mut::<PointerLocation>(pointer)
            .unwrap()
            .location = Some(Location {
            target: NormalizedRenderTarget::None {
                width: 100,
                height: 100,
            },
            position: Vec2::ZERO,
        });
        app.update();
        assert!(app.world().get::<Hovered>(other).unwrap().0);
        app.world_mut().despawn(pointer);
        app.update();
        assert!(!app.world().get::<Hovered>(other).unwrap().0);
    }
}

#[test]
fn concurrent_valid_hits_form_union_and_stale_entities_do_not_hover_ancestors() {
    let mut app = scene_app();
    if !app.is_plugin_added::<WidgetryPointerPlugin>() {
        app.add_plugins(WidgetryPointerPlugin);
    }
    app.init_resource::<HoverMap>();
    let root = app
        .world_mut()
        .spawn((Hovered(false), DirectlyHovered(false)))
        .id();
    let child = app.world_mut().spawn(ChildOf(root)).id();
    let pointers: Vec<_> = pointer_ids()
        .into_iter()
        .map(|id| {
            let pointer = app
                .world_mut()
                .spawn((
                    id,
                    PointerLocation::new(Location {
                        target: NormalizedRenderTarget::None {
                            width: 1,
                            height: 1,
                        },
                        position: Vec2::ZERO,
                    }),
                ))
                .id();
            let mut hits = EntityHashMap::default();
            hits.insert(child, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
            app.world_mut().resource_mut::<HoverMap>().insert(id, hits);
            pointer
        })
        .collect();
    app.update();
    assert!(app.world().get::<Hovered>(root).unwrap().0);
    app.world_mut().despawn(pointers[0]);
    app.update();
    assert!(app.world().get::<Hovered>(root).unwrap().0);
    app.world_mut().despawn(child);
    app.update();
    assert!(!app.world().get::<Hovered>(root).unwrap().0);
}

#[test]
fn native_focus_cancel_never_turns_into_a_release_button_activation() {
    use bevy::{
        input::{ButtonState, mouse::MouseButtonInput},
        ui_widgets::{Activate, Button, ButtonPlugin},
        window::{CursorMoved, WindowEvent, WindowFocused},
    };
    let mut app = bevy_widgetry_test_utils::picking_app();
    app.insert_resource(bevy::picking::input::PointerInputSettings {
        is_mouse_enabled: true,
        is_touch_enabled: false,
    });
    if !app.is_plugin_added::<WidgetryPointerPlugin>() {
        app.add_plugins(WidgetryPointerPlugin);
    }
    app.add_plugins(ButtonPlugin).init_resource::<Changes>();
    app.add_observer(|_: On<Activate>, mut count: ResMut<Changes>| count.0 += 1);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera =
        bevy_widgetry_test_utils::spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    app.world_mut().spawn_scene(bsn! {
        Button bevy_widgetry_core::pointer::WidgetryPointerPressed
        Node{position_type:PositionType::Absolute,left:px(40),top:px(40),width:px(200),height:px(40)}
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap();
    for _ in 0..4 {
        app.update();
    }
    app.world_mut()
        .write_message(WindowEvent::CursorMoved(CursorMoved {
            window,
            position: Vec2::new(80.0, 60.0),
            delta: None,
        }));
    app.update();
    for (index, button) in [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
        .into_iter()
        .enumerate()
    {
        app.world_mut()
            .write_message(WindowEvent::MouseButtonInput(MouseButtonInput {
                window,
                button,
                state: ButtonState::Pressed,
            }));
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, index);
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        app.world_mut()
            .write_message(WindowEvent::WindowFocused(WindowFocused {
                window,
                focused: false,
            }));
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, index);
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        app.world_mut().write_message_batch([
            WindowEvent::WindowFocused(WindowFocused {
                window,
                focused: true,
            }),
            WindowEvent::MouseButtonInput(MouseButtonInput {
                window,
                button,
                state: ButtonState::Released,
            }),
        ]);
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, index);
        app.world_mut()
            .write_message(WindowEvent::MouseButtonInput(MouseButtonInput {
                window,
                button,
                state: ButtonState::Pressed,
            }));
        app.update();
        app.world_mut()
            .write_message(WindowEvent::MouseButtonInput(MouseButtonInput {
                window,
                button,
                state: ButtonState::Released,
            }));
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, index + 1);
    }
}
