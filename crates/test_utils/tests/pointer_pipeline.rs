//! State：Mouse / Custom 的位置、press、drag、有效 / 注销与 window target。
//! Stimuli：PointerInput 经真实 UI backend 命中，Press / Release 跨 frame。
//! Guards：disabled 不 activation，父子命中只触发一次业务动作。
//! Invariants：fixture 不写 HoverMap、不 trigger 目标 event，保留原始 PointerId 与 target。
//! 基线差异由显式断言记录，后续生产适配落地时升级对应期望。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    camera::NormalizedRenderTarget,
    input::{mouse::MouseScrollUnit, touch::TouchPhase},
    picking::{
        events::*,
        hover::{HoverMap, Hovered},
        pointer::*,
    },
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::{Activate, Button, ButtonPlugin},
};
use bevy_widgetry_test_utils::{
    picking_app, pointer_event, pointer_ids, queue_pointer, spawn_picking_camera,
};

#[derive(Resource, Default)]
struct Seen {
    clicks: Vec<(PointerId, u8)>,
    activated: usize,
    drag_ends: Vec<Vec2>,
    scroll: Vec<f32>,
    outs: usize,
    cancels: usize,
    hover_changes: usize,
}

fn fixture(id: PointerId) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    app.add_plugins(ButtonPlugin)
        .init_resource::<Seen>()
        .add_systems(
            Update,
            |changed: Query<(), Changed<Hovered>>, mut seen: ResMut<Seen>| {
                seen.hover_changes += changed.iter().count()
            },
        );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 400).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            Node { width: px(100), height: px(100) }
            template(move |_| Ok(UiTargetCamera(camera)))
            Button
            Hovered(false)
            Children [Node { width: px(100), height: px(100) }]
        })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(root)
        .observe(|e: On<Pointer<Click>>, mut s: ResMut<Seen>| {
            s.clicks.push((e.pointer_id, e.count))
        })
        .observe(|_: On<Activate>, mut s: ResMut<Seen>| s.activated += 1)
        .observe(|e: On<Pointer<DragEnd>>, mut s: ResMut<Seen>| s.drag_ends.push(e.distance))
        .observe(|e: On<Pointer<Scroll>>, mut s: ResMut<Seen>| s.scroll.push(e.y))
        .observe(|_: On<Pointer<Out>>, mut s: ResMut<Seen>| s.outs += 1)
        .observe(|_: On<Pointer<Cancel>>, mut s: ResMut<Seen>| s.cancels += 1);
    if !id.is_mouse() {
        app.world_mut().spawn(id);
    }
    app.update();
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(Some(window))
            .unwrap(),
        position: Vec2::splat(30.0),
    };
    (app, root, window, location)
}

fn input(app: &mut App, id: PointerId, location: &Location, action: PointerAction) {
    queue_pointer(app, id, location.clone(), action);
    app.update();
}

#[test]
fn event_constructor_preserves_identity_location_and_payload() {
    for id in pointer_ids() {
        let location = Location {
            target: NormalizedRenderTarget::None {
                width: 900,
                height: 600,
            },
            position: Vec2::new(23.0, 17.0),
        };
        let entity = Entity::PLACEHOLDER;
        let event = pointer_event(
            id,
            location.clone(),
            entity,
            DragEnd {
                button: PointerButton::Secondary,
                distance: Vec2::new(3.0, 4.0),
            },
        );
        assert_eq!(event.pointer_id, id);
        assert_eq!(event.pointer_location, location);
        assert_eq!(event.entity, entity);
        assert_eq!(event.distance, Vec2::new(3.0, 4.0));
        assert_eq!(event.button, PointerButton::Secondary);
    }
}

#[test]
fn first_move_then_stationary_click_and_double_click_share_assertions() {
    for id in pointer_ids() {
        let (mut app, root, _, location) = fixture(id);
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move {
                delta: location.position,
            },
        );
        let child = app.world().get::<Children>(root).unwrap()[0];
        assert!(app.world().resource::<HoverMap>()[&id].contains_key(&child));
        // scene_app 自动装配共享 writer，Custom 的 hover 必须与 Mouse 保持相同结果。
        assert!(app.world().get::<Hovered>(root).unwrap().0);
        let changes = app.world().resource::<Seen>().hover_changes;
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().resource::<Seen>().hover_changes, changes);
        for _ in 0..2 {
            input(
                &mut app,
                id,
                &location,
                PointerAction::Press(PointerButton::Primary),
            );
            assert!(app.world().get::<Pressed>(root).is_some());
            input(
                &mut app,
                id,
                &location,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(app.world().get::<Pressed>(root).is_none());
        }
        let seen = app.world().resource::<Seen>();
        assert_eq!(seen.clicks, vec![(id, 1), (id, 2)]);
        assert_eq!(seen.activated, 2);
        input(
            &mut app,
            id,
            &location,
            PointerAction::Scroll {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: 2.0,
                phase: TouchPhase::Moved,
            },
        );
        assert_eq!(app.world().resource::<Seen>().scroll, vec![2.0]);
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        input(
            &mut app,
            id,
            &location,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            id,
            &location,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(app.world().resource::<Seen>().activated, 2);
    }
}

#[test]
fn drag_endpoint_and_leave_are_resolved_by_backend() {
    for id in pointer_ids() {
        let (mut app, root, _, mut location) = fixture(id);
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move {
                delta: location.position,
            },
        );
        input(
            &mut app,
            id,
            &location,
            PointerAction::Press(PointerButton::Primary),
        );
        location.position.x += 10.0;
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move {
                delta: Vec2::new(10.0, 0.0),
            },
        );
        location.position.x += 120.0;
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move {
                delta: Vec2::new(120.0, 0.0),
            },
        );
        input(
            &mut app,
            id,
            &location,
            PointerAction::Release(PointerButton::Primary),
        );
        let seen = app.world().resource::<Seen>();
        assert_eq!(seen.drag_ends, vec![Vec2::new(130.0, 0.0)]);
        assert!(seen.outs > 0);
        assert_eq!(seen.activated, 0);
        assert!(app.world().get::<Pressed>(root).is_none());
    }
}

#[test]
fn cancellation_and_pointer_invalidation_capture_terminal_gap() {
    for id in pointer_ids() {
        for invalidation in 0..3 {
            let (mut app, root, _, mut location) = fixture(id);
            input(
                &mut app,
                id,
                &location,
                PointerAction::Move {
                    delta: location.position,
                },
            );
            input(
                &mut app,
                id,
                &location,
                PointerAction::Press(PointerButton::Primary),
            );
            if invalidation != 0 {
                let pointer = app.world().resource::<PointerMap>().get_entity(id).unwrap();
                if invalidation == 1 {
                    app.world_mut().despawn(pointer);
                } else {
                    app.world_mut()
                        .get_mut::<PointerLocation>(pointer)
                        .unwrap()
                        .location = None;
                }
                app.update();
            } else {
                location.position = Vec2::splat(200.0);
                input(
                    &mut app,
                    id,
                    &location,
                    PointerAction::Move {
                        delta: Vec2::splat(170.0),
                    },
                );
                input(&mut app, id, &location, PointerAction::Cancel);
            }
            assert_eq!(app.world().resource::<Seen>().activated, 0);
            assert_eq!(app.world().resource::<Seen>().cancels, 0);
            // 未收到高层 Cancel 时官方 Button 的 Pressed 残留，后续 ownership 清理需修复。
            assert!(app.world().get::<Pressed>(root).is_some());
        }
    }
}

#[test]
fn target_window_identity_prevents_cross_window_hits_and_despawn_is_safe() {
    for id in pointer_ids() {
        let (mut app, root, window, mut location) = fixture(id);
        let other = app.world_mut().spawn(Window::default()).id();
        location.target =
            bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(other))
                .normalize(Some(window))
                .unwrap();
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move {
                delta: location.position,
            },
        );
        let child = app.world().get::<Children>(root).unwrap()[0];
        assert!(!app.world().resource::<HoverMap>()[&id].contains_key(&child));
        location.target =
            bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                .normalize(Some(window))
                .unwrap();
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move { delta: Vec2::ZERO },
        );
        input(
            &mut app,
            id,
            &location,
            PointerAction::Press(PointerButton::Primary),
        );
        app.world_mut().despawn(root);
        input(
            &mut app,
            id,
            &location,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(app.world().resource::<Seen>().activated, 0);
    }
}

#[test]
fn high_level_cancel_clears_pressed_but_queue_does_not_advance_app() {
    for id in pointer_ids() {
        let (mut app, root, _, location) = fixture(id);
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move {
                delta: location.position,
            },
        );
        queue_pointer(
            &mut app,
            id,
            location.clone(),
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().get::<Pressed>(root).is_none());
        app.update();
        assert!(app.world().get::<Pressed>(root).is_some());
        app.world_mut().trigger(pointer_event(
            id,
            location,
            root,
            Cancel {
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            },
        ));
        app.world_mut().flush();
        assert!(app.world().get::<Pressed>(root).is_none());
        assert_eq!(app.world().resource::<Seen>().activated, 0);
    }
}

#[test]
fn ignore_and_occlusion_preserve_backend_hover_authority() {
    for id in pointer_ids() {
        let (mut app, root, _, location) = fixture(id);
        let child = app.world().get::<Children>(root).unwrap()[0];
        app.world_mut()
            .entity_mut(child)
            .insert(bevy::picking::Pickable::IGNORE);
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move {
                delta: location.position,
            },
        );
        assert!(app.world().get::<Hovered>(root).unwrap().0);
        assert!(app.world().resource::<HoverMap>()[&id].contains_key(&root));
        app.world_mut()
            .entity_mut(root)
            .insert(bevy::picking::Pickable::IGNORE);
        app.update();
        assert!(!app.world().get::<Hovered>(root).unwrap().0);
        app.world_mut()
            .entity_mut(child)
            .remove::<bevy::picking::Pickable>();
        app.world_mut()
            .entity_mut(root)
            .remove::<bevy::picking::Pickable>();
        let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
        let blocker = app.world_mut().spawn_scene(bsn! {
            Node { width: px(100), height: px(100), position_type: PositionType::Absolute, left: px(0), top: px(0) }
            template(move |_| Ok(UiTargetCamera(camera)))
            ZIndex(1)
        }).unwrap().id();
        app.update();
        app.update();
        assert!(!app.world().get::<Hovered>(root).unwrap().0);
        assert!(app.world().resource::<HoverMap>()[&id].contains_key(&blocker));
        app.world_mut().despawn(blocker);
        app.update();
        app.update();
        assert!(app.world().get::<Hovered>(root).unwrap().0);
    }
}
