//! Coverage Model：窗口光标在 Default、Hover、Dragging 与 Native 间切换。
//! stimuli 为真实 PointerInput、backend PointerHits、Disabled 与 owner/window lifecycle。
//! guards 限定 Mouse、有效窗口、有效 owner 与所属按键，capture 优先于可见 hit。
//! invariants 为窗口隔离、最近声明继承、前方遮挡不泄漏、无变化不写入及 Last 同帧消费。
//! 安装冲突通过宿主 Error handler 和统一日志报告，并停止 Widgetry writer。

// 测试断言需要在 contract 不满足时立即失败。
// 仅在测试 scope 允许 assertion 与 fixture 的 unwrap。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::picking::{cursor::EntityCursor, hover::PointerCaptureMap, pointer::PointerButton};
use bevy::{
    picking::{
        InteractionPlugin, PickingPlugin,
        backend::{HitData, PointerHits},
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    window::{CursorIcon, SystemCursorIcon},
};
use bevy_widgetry_cursor::{CursorLease, WidgetryCursorPlugin};

fn fixture() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        PickingPlugin,
        InteractionPlugin,
        WidgetryCursorPlugin,
    ));
    app.update();
    let window = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn_empty().id();
    app.world_mut().spawn(PointerId::Mouse);
    (app, window, camera)
}

fn location(window: Entity) -> Location {
    Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::splat(20.0),
    }
}

fn hover(app: &mut App, window: Entity, camera: Entity, entity: Entity) {
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Move { delta: Vec2::ONE },
    ));
    app.world_mut().write_message(PointerHits::new(
        PointerId::Mouse,
        vec![(entity, HitData::new(camera, 0.0, None, None))],
        0.0,
    ));
    app.update();
}

#[test]
fn hover_inherits_nearest_declaration_and_explicit_default() {
    let (mut app, window, camera) = fixture();
    let parent = app
        .world_mut()
        .spawn(bevy::picking::cursor::EntityCursor::System(
            SystemCursorIcon::Pointer,
        ))
        .id();
    let child = app.world_mut().spawn(ChildOf(parent)).id();
    hover(&mut app, window, camera, child);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    app.world_mut()
        .entity_mut(child)
        .insert(bevy::picking::cursor::EntityCursor::System(
            SystemCursorIcon::Default,
        ));
    hover(&mut app, window, camera, child);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
}

#[test]
fn disabled_ancestor_above_the_nearest_declaration_is_respected() {
    let (mut app, window, camera) = fixture();
    let ancestor = app.world_mut().spawn(bevy::ui::InteractionDisabled).id();
    let owner = app
        .world_mut()
        .spawn((
            ChildOf(ancestor),
            EntityCursor::System(SystemCursorIcon::Pointer),
        ))
        .id();
    hover(&mut app, window, camera, owner);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::NotAllowed))
    );
}

#[test]
fn layers_depth_passthrough_and_opaque_without_declaration() {
    let (mut app, window, camera) = fixture();
    let lower = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Text))
        .id();
    let top = app
        .world_mut()
        .spawn((
            EntityCursor::System(SystemCursorIcon::Pointer),
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id();
    for reverse in [false, true] {
        let mut hits = vec![
            (lower, HitData::new(camera, 0.0, None, None)),
            (top, HitData::new(camera, 100.0, None, None)),
        ];
        if reverse {
            hits.reverse();
        }
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location(window),
            PointerAction::Move { delta: Vec2::ONE },
        ));
        for (entity, hit) in hits {
            app.world_mut().write_message(PointerHits::new(
                PointerId::Mouse,
                vec![(entity, hit)],
                if entity == top { 10.0 } else { 0.0 },
            ));
        }
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(SystemCursorIcon::Pointer))
        );
    }
    app.world_mut().entity_mut(top).remove::<EntityCursor>();
    app.world_mut().entity_mut(top).insert(Pickable::default());
    app.world_mut().write_message(PointerHits::new(
        PointerId::Mouse,
        vec![
            (top, HitData::new(camera, 0.0, None, None)),
            (lower, HitData::new(camera, 1.0, None, None)),
        ],
        0.0,
    ));
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    app.world_mut()
        .entity_mut(top)
        .insert(EntityCursor::System(SystemCursorIcon::Pointer));
    app.world_mut().entity_mut(top).insert(Pickable {
        should_block_lower: false,
        is_hoverable: true,
    });
    app.world_mut().write_message(PointerHits::new(
        PointerId::Mouse,
        vec![
            (top, HitData::new(camera, 2.0, None, None)),
            (lower, HitData::new(camera, 1.0, None, None)),
        ],
        0.0,
    ));
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Text))
    );
}

#[test]
fn capture_overrides_hits_and_disabled_applies_only_to_declarations() {
    let (mut app, window, camera) = fixture();
    let visible = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    let capture = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Text))
        .id();
    app.world_mut().resource_mut::<PointerCaptureMap>().capture(
        PointerId::Mouse,
        capture,
        HitData::new(camera, 3.0, None, None),
    );
    hover(&mut app, window, camera, visible);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Text))
    );
    app.world_mut()
        .entity_mut(capture)
        .insert(bevy::ui::InteractionDisabled);
    hover(&mut app, window, camera, visible);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::NotAllowed))
    );
    app.world_mut().entity_mut(capture).remove::<EntityCursor>();
    hover(&mut app, window, camera, visible);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
}

#[test]
fn windows_are_isolated_and_non_mouse_never_gets_a_lease() {
    let (mut app, window, camera) = fixture();
    let other = app
        .world_mut()
        .spawn((
            Window::default(),
            CursorIcon::System(SystemCursorIcon::Crosshair),
        ))
        .id();
    let hit = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    hover(&mut app, window, camera, hit);
    assert_eq!(
        app.world().get::<CursorIcon>(other),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    hover(&mut app, other, camera, hit);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    assert_eq!(
        app.world().get::<CursorIcon>(other),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    assert!(
        CursorLease::dragging(
            PointerId::Touch(0),
            &location(window),
            PointerButton::Primary,
            SystemCursorIcon::Grabbing
        )
        .is_none()
    );
    assert!(
        CursorLease::native(
            PointerId::Touch(0),
            &location(window),
            PointerButton::Primary
        )
        .is_none()
    );
    app.world_mut().despawn(window);
    hover(&mut app, window, camera, hit);
    assert_eq!(
        app.world().get::<CursorIcon>(other),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
}

#[test]
fn drag_retains_cursor_outside_owner_and_release_restores_current_hover() {
    let (mut app, window, camera) = fixture();
    let owner = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Grab))
        .id();
    let outside = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Text))
        .id();
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Press(PointerButton::Primary),
    ));
    app.world_mut().entity_mut(owner).insert(
        CursorLease::dragging(
            PointerId::Mouse,
            &location(window),
            PointerButton::Primary,
            SystemCursorIcon::Grabbing,
        )
        .unwrap(),
    );
    hover(&mut app, window, camera, outside);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Grabbing))
    );
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Release(PointerButton::Secondary),
    ));
    hover(&mut app, window, camera, outside);
    assert!(app.world().get::<CursorLease>(owner).is_some());
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Release(PointerButton::Primary),
    ));
    hover(&mut app, window, camera, outside);
    assert!(app.world().get::<CursorLease>(owner).is_none());
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Text))
    );
}

#[test]
fn no_change_does_not_mark_window_cursor_changed() {
    let (mut app, window, camera) = fixture();
    let hit = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    hover(&mut app, window, camera, hit);
    app.world_mut().clear_trackers();
    hover(&mut app, window, camera, hit);
    assert!(
        !app.world()
            .entity(window)
            .get_ref::<CursorIcon>()
            .unwrap()
            .is_changed()
    );
}

#[test]
fn invalid_and_cancelled_leases_cannot_keep_control() {
    for native in [false, true] {
        for terminal in [
            "cancel", "disabled", "ancestor", "owner", "window", "closed", "blur", "pointer",
            "target",
        ] {
            let (mut app, window, camera) = fixture();
            let ancestor = app.world_mut().spawn_empty().id();
            let owner = app
                .world_mut()
                .spawn((
                    ChildOf(ancestor),
                    EntityCursor::System(SystemCursorIcon::Grab),
                ))
                .id();
            app.world_mut().write_message(PointerInput::new(
                PointerId::Mouse,
                location(window),
                PointerAction::Press(PointerButton::Primary),
            ));
            let lease = if native {
                CursorLease::native(PointerId::Mouse, &location(window), PointerButton::Primary)
            } else {
                CursorLease::dragging(
                    PointerId::Mouse,
                    &location(window),
                    PointerButton::Primary,
                    SystemCursorIcon::Grabbing,
                )
            }
            .unwrap();
            app.world_mut().entity_mut(owner).insert(lease);
            app.world_mut()
                .entity_mut(window)
                .insert(CursorIcon::System(SystemCursorIcon::Grabbing));
            hover(&mut app, window, camera, owner);
            assert_eq!(
                app.world().get::<CursorIcon>(window),
                Some(&CursorIcon::System(SystemCursorIcon::Grabbing))
            );
            match terminal {
                "cancel" => {
                    app.world_mut().write_message(PointerInput::new(
                        PointerId::Mouse,
                        location(window),
                        PointerAction::Cancel,
                    ));
                }
                "disabled" => {
                    app.world_mut()
                        .entity_mut(owner)
                        .insert(bevy::ui::InteractionDisabled);
                }
                "ancestor" => {
                    app.world_mut()
                        .entity_mut(ancestor)
                        .insert(bevy::ui::InteractionDisabled);
                }
                "owner" => {
                    app.world_mut().despawn(owner);
                }
                "window" => {
                    app.world_mut().despawn(window);
                }
                "closed" => {
                    app.world_mut()
                        .write_message(bevy::window::WindowClosed { window });
                }
                "blur" => {
                    app.world_mut().write_message(bevy::window::WindowFocused {
                        window,
                        focused: false,
                    });
                }
                "pointer" => {
                    let pointer = app
                        .world_mut()
                        .query_filtered::<Entity, With<PointerId>>()
                        .single(app.world())
                        .unwrap();
                    app.world_mut().despawn(pointer);
                }
                _ => {
                    let other = app.world_mut().spawn(Window::default()).id();
                    app.world_mut().write_message(PointerInput::new(
                        PointerId::Mouse,
                        location(other),
                        PointerAction::Move { delta: Vec2::ONE },
                    ));
                }
            }
            app.update();
            assert!(
                app.world().get::<CursorLease>(owner).is_none(),
                "{terminal}"
            );
            if terminal != "window" {
                assert_ne!(
                    app.world().get::<CursorIcon>(window),
                    Some(&CursorIcon::System(SystemCursorIcon::Grabbing)),
                    "{terminal}"
                );
            }
        }
    }
}

#[test]
fn native_yields_through_pointer_out_and_resized_then_resumes_on_activity() {
    let (mut app, window, camera) = fixture();
    let owner = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::NeResize))
        .id();
    hover(&mut app, window, camera, owner);
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Press(PointerButton::Primary),
    ));
    app.world_mut().entity_mut(owner).insert(
        CursorLease::native(PointerId::Mouse, &location(window), PointerButton::Primary).unwrap(),
    );
    app.world_mut()
        .entity_mut(window)
        .insert(CursorIcon::System(SystemCursorIcon::Crosshair));
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Crosshair))
    );
    let pointer = app
        .world_mut()
        .query_filtered::<Entity, With<PointerId>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<bevy::picking::pointer::PointerLocation>(pointer)
        .unwrap()
        .location = None;
    app.add_message::<bevy::window::WindowResized>();
    app.world_mut().write_message(bevy::window::WindowResized {
        window,
        width: 800.0,
        height: 600.0,
    });
    app.update();
    assert!(app.world().get::<CursorLease>(owner).is_some());
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Crosshair))
    );
    hover(&mut app, window, camera, owner);
    assert!(app.world().get::<CursorLease>(owner).is_none());
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::NeResize))
    );
}

#[test]
fn native_lease_does_not_survive_mouse_entity_destruction() {
    let (mut app, window, camera) = fixture();
    let owner = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::NeResize))
        .id();
    app.world_mut().entity_mut(owner).insert(
        CursorLease::native(PointerId::Mouse, &location(window), PointerButton::Primary).unwrap(),
    );
    hover(&mut app, window, camera, owner);
    let pointer = app
        .world_mut()
        .query_filtered::<Entity, With<PointerId>>()
        .single(app.world())
        .unwrap();
    app.world_mut().despawn(pointer);
    app.update();
    assert!(app.world().get::<CursorLease>(owner).is_none());
}

#[derive(Resource, Default)]
struct BackendCursor(Option<CursorIcon>);

#[test]
fn post_update_commands_are_seen_by_the_cursor_backend_in_last() {
    let (mut app, window, camera) = fixture();
    let owner = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    app.init_resource::<BackendCursor>();
    app.add_systems(PostUpdate, move |mut commands: Commands| {
        commands.entity(owner).insert(bevy::ui::InteractionDisabled);
    });
    app.add_systems(
        Last,
        (move |windows: Query<&CursorIcon>, mut result: ResMut<BackendCursor>| {
            result.0 = windows.get(window).ok().cloned();
        })
        .in_set(bevy::window::CursorSystems::Update),
    );
    hover(&mut app, window, camera, owner);
    assert_eq!(
        app.world().resource::<BackendCursor>().0,
        Some(CursorIcon::System(SystemCursorIcon::NotAllowed))
    );
}

#[test]
fn duplicate_cursor_writer_is_reported_in_both_plugin_orders() {
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    for widgetry_first in [false, true] {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(Startup, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        if widgetry_first {
            app.add_plugins((
                WidgetryCursorPlugin,
                bevy::picking::cursor::CursorIconPlugin,
            ));
        } else {
            app.add_plugins((
                bevy::picking::cursor::CursorIconPlugin,
                WidgetryCursorPlugin,
            ));
        }
        let window = app.world_mut().spawn(Window::default()).id();
        let owner = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(owner).insert(
            CursorLease::native(PointerId::Mouse, &location(window), PointerButton::Primary)
                .unwrap(),
        );
        app.finish();
        app.cleanup();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        errors.run(|| logs.run(|| app.update()));
        let errors = errors.take();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
        assert!(errors[0].to_string().contains("CursorIconPlugin"));
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::Level::ERROR)
                .count(),
            1
        );
        assert!(app.world().get::<CursorLease>(owner).is_some());
    }
}
