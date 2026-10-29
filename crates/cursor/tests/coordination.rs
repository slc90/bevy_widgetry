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
use bevy_widgetry_cursor::{
    CursorLease, WidgetryCursorHint, WidgetryCursorPlugin, WidgetryCursorTheme,
};

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
fn loading_wait_overrides_disabled_even_for_a_hit_on_descendant_content() {
    let (mut app, window, camera) = fixture();
    let ancestor = app.world_mut().spawn(bevy::ui::InteractionDisabled).id();
    let owner = app
        .world_mut()
        .spawn((
            ChildOf(ancestor),
            EntityCursor::System(SystemCursorIcon::Pointer),
            bevy_widgetry_cursor::internal::LoadingCursor,
        ))
        .id();
    let child = app
        .world_mut()
        .spawn((
            ChildOf(owner),
            bevy::ui::InteractionDisabled,
            EntityCursor::System(SystemCursorIcon::Text),
        ))
        .id();
    hover(&mut app, window, camera, child);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Wait))
    );
    app.world_mut()
        .entity_mut(owner)
        .remove::<bevy_widgetry_cursor::internal::LoadingCursor>();
    hover(&mut app, window, camera, child);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::NotAllowed))
    );
    app.world_mut()
        .entity_mut(ancestor)
        .remove::<bevy::ui::InteractionDisabled>();
    app.world_mut()
        .entity_mut(child)
        .remove::<bevy::ui::InteractionDisabled>();
    hover(&mut app, window, camera, child);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Text))
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

#[test]
fn extended_hints_use_nearest_declaration_and_entity_cursor_wins_ties() {
    let (mut app, window, camera) = fixture();
    assert_eq!(
        *app.world().resource::<WidgetryCursorTheme>(),
        WidgetryCursorTheme::System
    );
    let parent = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Text))
        .id();
    let child = app
        .world_mut()
        .spawn((ChildOf(parent), WidgetryCursorHint::PanReady))
        .id();
    for (hint, fallback) in [
        (WidgetryCursorHint::AddSelection, SystemCursorIcon::Copy),
        (
            WidgetryCursorHint::RemoveSelection,
            SystemCursorIcon::NotAllowed,
        ),
        (
            WidgetryCursorHint::RangeSelection,
            SystemCursorIcon::Crosshair,
        ),
        (WidgetryCursorHint::DragSort, SystemCursorIcon::NsResize),
        (WidgetryCursorHint::InsertDrop, SystemCursorIcon::Copy),
        (WidgetryCursorHint::ReplaceDrop, SystemCursorIcon::Move),
        (
            WidgetryCursorHint::HorizontalValueAdjust,
            SystemCursorIcon::EwResize,
        ),
        (
            WidgetryCursorHint::VerticalValueAdjust,
            SystemCursorIcon::NsResize,
        ),
        (WidgetryCursorHint::PanReady, SystemCursorIcon::Grab),
        (WidgetryCursorHint::Panning, SystemCursorIcon::Grabbing),
        (WidgetryCursorHint::ReadOnly, SystemCursorIcon::Cell),
    ] {
        for theme in [WidgetryCursorTheme::System, WidgetryCursorTheme::Kamuri] {
            app.world_mut().insert_resource(theme);
            app.world_mut().entity_mut(child).insert(hint);
            hover(&mut app, window, camera, child);
            assert_eq!(
                app.world().get::<CursorIcon>(window),
                Some(&CursorIcon::System(fallback))
            );
        }
    }
    app.world_mut()
        .entity_mut(child)
        .insert(EntityCursor::System(SystemCursorIcon::Help));
    hover(&mut app, window, camera, child);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Help))
    );
    app.world_mut()
        .entity_mut(parent)
        .insert(bevy::ui::InteractionDisabled);
    hover(&mut app, window, camera, child);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::NotAllowed))
    );
}

use bevy_widgetry_asset::{BuiltinCursorAnimation as Animation, BuiltinCursorImage};
use std::time::{Duration, Instant};

const SYSTEM_ANIMATIONS: [(SystemCursorIcon, Animation); 34] = [
    (SystemCursorIcon::Default, Animation::Default),
    (SystemCursorIcon::Pointer, Animation::Pointer),
    (SystemCursorIcon::ContextMenu, Animation::ContextMenu),
    (SystemCursorIcon::Help, Animation::Help),
    (SystemCursorIcon::Text, Animation::Text),
    (SystemCursorIcon::VerticalText, Animation::VerticalText),
    (SystemCursorIcon::Cell, Animation::Cell),
    (SystemCursorIcon::Crosshair, Animation::Crosshair),
    (SystemCursorIcon::Grab, Animation::Grab),
    (SystemCursorIcon::Grabbing, Animation::Grabbing),
    (SystemCursorIcon::Move, Animation::Move),
    (SystemCursorIcon::Copy, Animation::Copy),
    (SystemCursorIcon::Alias, Animation::Alias),
    (SystemCursorIcon::NoDrop, Animation::NoDrop),
    (SystemCursorIcon::EResize, Animation::EResize),
    (SystemCursorIcon::NResize, Animation::NResize),
    (SystemCursorIcon::NeResize, Animation::NeResize),
    (SystemCursorIcon::NwResize, Animation::NwResize),
    (SystemCursorIcon::SResize, Animation::SResize),
    (SystemCursorIcon::SeResize, Animation::SeResize),
    (SystemCursorIcon::SwResize, Animation::SwResize),
    (SystemCursorIcon::WResize, Animation::WResize),
    (SystemCursorIcon::EwResize, Animation::EwResize),
    (SystemCursorIcon::NsResize, Animation::NsResize),
    (SystemCursorIcon::NeswResize, Animation::NeswResize),
    (SystemCursorIcon::NwseResize, Animation::NwseResize),
    (SystemCursorIcon::ColResize, Animation::ColResize),
    (SystemCursorIcon::RowResize, Animation::RowResize),
    (SystemCursorIcon::AllScroll, Animation::AllScroll),
    (SystemCursorIcon::ZoomIn, Animation::ZoomIn),
    (SystemCursorIcon::ZoomOut, Animation::ZoomOut),
    (SystemCursorIcon::Progress, Animation::Progress),
    (SystemCursorIcon::Wait, Animation::Wait),
    (SystemCursorIcon::NotAllowed, Animation::NotAllowed),
];
const HINT_ANIMATIONS: [(WidgetryCursorHint, Animation); 11] = [
    (WidgetryCursorHint::AddSelection, Animation::AddSelection),
    (
        WidgetryCursorHint::RemoveSelection,
        Animation::RemoveSelection,
    ),
    (
        WidgetryCursorHint::RangeSelection,
        Animation::RangeSelection,
    ),
    (WidgetryCursorHint::DragSort, Animation::DragSort),
    (WidgetryCursorHint::InsertDrop, Animation::InsertDrop),
    (WidgetryCursorHint::ReplaceDrop, Animation::ReplaceDrop),
    (
        WidgetryCursorHint::HorizontalValueAdjust,
        Animation::HorizontalValueAdjust,
    ),
    (
        WidgetryCursorHint::VerticalValueAdjust,
        Animation::VerticalValueAdjust,
    ),
    (WidgetryCursorHint::PanReady, Animation::PanReady),
    (WidgetryCursorHint::Panning, Animation::Panning),
    (WidgetryCursorHint::ReadOnly, Animation::ReadOnly),
];

#[derive(Resource)]
struct TestImages {
    _images: Vec<Handle<Image>>,
}

fn animation_app() -> App {
    animation_app_with_loader(true)
}

fn animation_app_with_loader(register_loader: bool) -> App {
    let mut app = App::new();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::ZERO,
    ));
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::image::TextureAtlasPlugin,
        PickingPlugin,
        InteractionPlugin,
        WidgetryCursorPlugin,
    ));
    if register_loader {
        app.register_asset_loader(bevy::image::ImageLoader::new(
            bevy::image::CompressedImageFormats::NONE,
        ));
    }
    app.insert_resource(WidgetryCursorTheme::Kamuri);
    app.world_mut().spawn(PointerId::Mouse);
    app
}

fn animation_fixture() -> (App, Entity, Entity, Entity) {
    let mut app = animation_app();
    let window = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn_empty().id();
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    let server = app.world().resource::<AssetServer>();
    let handles: Vec<Handle<Image>> = Animation::ALL
        .into_iter()
        .map(|a| server.load(a.path()))
        .chain([server.load(BuiltinCursorImage::KamuriStaticAtlas.path())])
        .collect();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        hover(&mut app, window, camera, target);
        if handles
            .iter()
            .all(|h| app.world().resource::<Assets<Image>>().contains(h.id()))
            && animation_frame(&app, window, Animation::Pointer).is_some()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "embedded cursor loading timed out"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    app.insert_resource(TestImages { _images: handles });
    (app, window, camera, target)
}

fn animation_frame(app: &App, window: Entity, animation: Animation) -> Option<usize> {
    let CursorIcon::Custom(bevy::window::CustomCursor::Image(image)) =
        app.world().get::<CursorIcon>(window)?
    else {
        return None;
    };
    if image.handle.path() != Some(&animation.path()) {
        return None;
    }
    assert_eq!(image.hotspot, animation.hotspot());
    let atlas = image.texture_atlas.as_ref().unwrap();
    let layout = app
        .world()
        .resource::<Assets<TextureAtlasLayout>>()
        .get(&atlas.layout)
        .unwrap();
    assert_eq!(layout.textures[atlas.index].size(), UVec2::splat(80));
    Some(atlas.index)
}

fn assert_static_sprite(app: &App, window: Entity, animation: Animation) {
    let image = match app.world().get::<CursorIcon>(window) {
        Some(CursorIcon::Custom(bevy::window::CustomCursor::Image(image))) => Some(image),
        _ => None,
    }
    .unwrap();
    assert_eq!(
        image.handle.path(),
        Some(&BuiltinCursorImage::KamuriStaticAtlas.path())
    );
    let index = Animation::ALL.iter().position(|a| *a == animation).unwrap();
    assert_eq!(image.texture_atlas.as_ref().unwrap().index, index);
    assert_eq!(image.hotspot, animation.hotspot());
}

fn tick(app: &mut App, window: Entity, camera: Entity, target: Entity, millis: u64) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(millis),
    ));
    app.world_mut().write_message(PointerHits::new(
        PointerId::Mouse,
        vec![(target, HitData::new(camera, 0.0, None, None))],
        0.0,
    ));
    app.update();
    assert!(app.world().get::<CursorIcon>(window).is_some());
}

fn clear_redraw(app: &mut App) {
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
}
fn has_redraw(app: &App) -> bool {
    !app.world()
        .resource::<Messages<bevy::window::RequestRedraw>>()
        .is_empty()
}

fn assert_loop(
    app: &mut App,
    window: Entity,
    camera: Entity,
    target: Entity,
    animation: Animation,
) {
    tick(app, window, camera, target, 0);
    assert_eq!(animation_frame(app, window, animation), Some(0));
    let period = animation.frame_duration().as_millis() as u64;
    tick(app, window, camera, target, period - 1);
    assert_eq!(animation_frame(app, window, animation), Some(0));
    assert!(
        !app.world()
            .entity(window)
            .get_ref::<CursorIcon>()
            .unwrap()
            .is_changed()
    );
    tick(app, window, camera, target, 1);
    assert_eq!(animation_frame(app, window, animation), Some(1));
    for i in 2..=animation.frame_count() * 2 {
        tick(app, window, camera, target, period);
        assert_eq!(
            animation_frame(app, window, animation),
            Some(i % animation.frame_count())
        );
    }
    assert!(has_redraw(app));
}

#[test]
fn all_45_semantics_animate_with_independent_timing_and_restart_only_on_semantic_change() {
    let (mut app, window, camera, target) = animation_fixture();
    for (icon, animation) in SYSTEM_ANIMATIONS {
        app.world_mut()
            .entity_mut(target)
            .insert(EntityCursor::System(icon));
        assert_loop(&mut app, window, camera, target, animation);
    }
    app.world_mut().entity_mut(target).remove::<EntityCursor>();
    for (hint, animation) in HINT_ANIMATIONS {
        app.world_mut().entity_mut(target).insert(hint);
        assert_loop(&mut app, window, camera, target, animation);
        app.insert_resource(WidgetryCursorTheme::System);
        tick(&mut app, window, camera, target, 0);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(hint.system_fallback()))
        );
        app.insert_resource(WidgetryCursorTheme::Kamuri);
    }
}

#[test]
fn stationary_animation_does_not_wake_when_mouse_leaves_or_only_custom_pointer_remains() {
    let (mut app, window, camera, target) = animation_fixture();
    let other = app.world_mut().spawn(Window::default()).id();
    tick(&mut app, window, camera, target, 100);
    assert_eq!(
        app.world().get::<CursorIcon>(other),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    let mouse = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::picking::pointer::PointerLocation>>()
        .iter(app.world())
        .find(|e| app.world().get::<PointerId>(*e) == Some(&PointerId::Mouse))
        .unwrap();
    app.world_mut()
        .get_mut::<bevy::picking::pointer::PointerLocation>(mouse)
        .unwrap()
        .location = None;
    clear_redraw(&mut app);
    tick(&mut app, window, camera, target, 100);
    assert!(!has_redraw(&app));
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    let custom = PointerId::Custom(Default::default());
    app.world_mut().spawn(custom);
    app.world_mut().write_message(PointerInput::new(
        custom,
        location(window),
        PointerAction::Move { delta: Vec2::ONE },
    ));
    tick(&mut app, window, camera, target, 100);
    assert!(!has_redraw(&app));
    hover(&mut app, window, camera, target);
    assert_eq!(animation_frame(&app, window, Animation::Pointer), Some(0));
}

#[test]
fn system_and_custom_cursors_never_start_kamuri_playback_or_change_caller_images() {
    let (mut app, window, camera, target) = animation_fixture();
    let custom = bevy::window::CustomCursor::Image(bevy::window::CustomCursorImage {
        handle: Handle::default(),
        hotspot: (2, 3),
        ..default()
    });
    for theme in [WidgetryCursorTheme::Kamuri, WidgetryCursorTheme::System] {
        app.insert_resource(theme);
        app.world_mut()
            .entity_mut(target)
            .insert(EntityCursor::Custom(custom.clone()));
        clear_redraw(&mut app);
        tick(&mut app, window, camera, target, 100);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::Custom(custom.clone()))
        );
        assert!(!has_redraw(&app));
    }
    for (icon, _) in SYSTEM_ANIMATIONS {
        app.world_mut()
            .entity_mut(target)
            .insert(EntityCursor::System(icon));
        tick(&mut app, window, camera, target, 100);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(icon))
        );
        assert!(!has_redraw(&app));
    }
}

#[test]
fn animation_obeys_native_lease_drag_capture_disabled_loading_and_window_lifecycle() {
    let (mut app, window, camera, target) = animation_fixture();
    tick(&mut app, window, camera, target, 70);
    assert_eq!(animation_frame(&app, window, Animation::Pointer), Some(1));
    app.world_mut().entity_mut(target).insert(
        CursorLease::native(PointerId::Mouse, &location(window), PointerButton::Primary).unwrap(),
    );
    clear_redraw(&mut app);
    tick(&mut app, window, camera, target, 500);
    assert_eq!(animation_frame(&app, window, Animation::Pointer), Some(1));
    assert!(
        !app.world()
            .entity(window)
            .get_ref::<CursorIcon>()
            .unwrap()
            .is_changed()
    );
    assert!(!has_redraw(&app));
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Release(PointerButton::Primary),
    ));
    tick(&mut app, window, camera, target, 0);
    assert_eq!(animation_frame(&app, window, Animation::Pointer), Some(0));
    app.world_mut().entity_mut(target).insert(
        CursorLease::dragging(
            PointerId::Mouse,
            &location(window),
            PointerButton::Primary,
            SystemCursorIcon::Grabbing,
        )
        .unwrap(),
    );
    tick(&mut app, window, camera, target, 60);
    assert_eq!(animation_frame(&app, window, Animation::Grabbing), Some(0));
    app.world_mut()
        .resource_mut::<bevy::picking::hover::HoverMap>()
        .clear();
    tick(&mut app, window, camera, target, 60);
    assert_eq!(animation_frame(&app, window, Animation::Grabbing), Some(1));
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Cancel,
    ));
    tick(&mut app, window, camera, target, 0);
    assert!(app.world().get::<CursorLease>(target).is_none());
    hover(&mut app, window, camera, target);
    app.world_mut()
        .entity_mut(target)
        .insert(bevy::ui::InteractionDisabled);
    tick(&mut app, window, camera, target, 0);
    assert_eq!(
        animation_frame(&app, window, Animation::NotAllowed),
        Some(0)
    );
    app.world_mut()
        .entity_mut(target)
        .insert(bevy_widgetry_cursor::internal::LoadingCursor);
    tick(&mut app, window, camera, target, 0);
    assert_eq!(animation_frame(&app, window, Animation::Wait), Some(0));
    app.world_mut().entity_mut(target).remove::<(
        bevy::ui::InteractionDisabled,
        bevy_widgetry_cursor::internal::LoadingCursor,
    )>();
    let captured = app.world_mut().spawn(WidgetryCursorHint::ReadOnly).id();
    app.world_mut().resource_mut::<PointerCaptureMap>().capture(
        PointerId::Mouse,
        captured,
        HitData::new(camera, 0.0, None, None),
    );
    tick(&mut app, window, camera, target, 0);
    assert_eq!(animation_frame(&app, window, Animation::ReadOnly), Some(0));
    tick(&mut app, window, camera, target, 80);
    assert_eq!(animation_frame(&app, window, Animation::ReadOnly), Some(1));
    app.world_mut()
        .resource_mut::<PointerCaptureMap>()
        .release(PointerId::Mouse);
    tick(&mut app, window, camera, target, 0);
    assert_eq!(animation_frame(&app, window, Animation::Pointer), Some(0));
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    clear_redraw(&mut app);
    tick(&mut app, window, camera, target, 100);
    assert!(!has_redraw(&app));
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    tick(&mut app, window, camera, target, 0);
    assert_eq!(animation_frame(&app, window, Animation::Pointer), Some(0));
    let other = app.world_mut().spawn(Window::default()).id();
    hover(&mut app, other, camera, target);
    assert_eq!(animation_frame(&app, other, Animation::Pointer), Some(0));
    assert_eq!(animation_frame(&app, window, Animation::Pointer), None);
    app.world_mut()
        .write_message(bevy::window::WindowClosed { window: other });
    clear_redraw(&mut app);
    tick(&mut app, other, camera, target, 0);
    assert!(!has_redraw(&app));
    app.world_mut().despawn(other);
    app.world_mut().despawn(target);
    app.update();
}

#[test]
fn missing_and_invalid_frames_use_static_then_system_and_restart_on_recovery() {
    let (mut app, window, camera, target) = animation_fixture();
    for (animation, icon) in [
        (Animation::Pointer, SystemCursorIcon::Pointer),
        (Animation::Wait, SystemCursorIcon::Wait),
        (Animation::Text, SystemCursorIcon::Text),
    ] {
        app.world_mut()
            .entity_mut(target)
            .insert(EntityCursor::System(icon));
        tick(&mut app, window, camera, target, 0);
        let atlas: Handle<Image> = app.world().resource::<AssetServer>().load(animation.path());
        let fallback: Handle<Image> = app
            .world()
            .resource::<AssetServer>()
            .load(BuiltinCursorImage::KamuriStaticAtlas.path());
        let saved = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .remove(atlas.id())
            .unwrap();
        clear_redraw(&mut app);
        tick(&mut app, window, camera, target, 100);
        assert_static_sprite(&app, window, animation);
        assert!(!has_redraw(&app));
        let saved_static = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .remove(fallback.id())
            .unwrap();
        tick(&mut app, window, camera, target, 100);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(icon))
        );
        let mut invalid = saved.clone();
        invalid.data = None;
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(atlas.id(), invalid)
            .unwrap();
        tick(&mut app, window, camera, target, 100);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(icon))
        );
        assert!(!has_redraw(&app));
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(fallback.id(), saved_static)
            .unwrap();
        tick(&mut app, window, camera, target, 100);
        assert_static_sprite(&app, window, animation);
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(atlas.id(), saved)
            .unwrap();
        tick(&mut app, window, camera, target, 100);
        assert_eq!(animation_frame(&app, window, animation), Some(0));
    }
}

#[test]
fn failed_png_decodes_warn_once_and_keep_system_pointer_without_waking() {
    let mut app = animation_app();
    app.edit_schedule(Last, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let paths = [
        Animation::Pointer.path(),
        BuiltinCursorImage::KamuriStaticAtlas.path(),
    ];
    for path in &paths {
        app.world()
            .resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
            .insert_asset(Default::default(), path.path(), b"invalid PNG".to_vec());
    }
    let handles: Vec<Handle<Image>> = paths
        .into_iter()
        .map(|p| app.world().resource::<AssetServer>().load(p))
        .collect();
    let window = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn_empty().id();
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    let logs = bevy_widgetry_test_utils::LogCapture::default();
    logs.run(|| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            hover(&mut app, window, camera, target);
            if handles.iter().all(|h| {
                matches!(
                    app.world().resource::<AssetServer>().get_load_state(h.id()),
                    Some(bevy::asset::LoadState::Failed(_))
                )
            }) {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        for _ in 0..4 {
            clear_redraw(&mut app);
            hover(&mut app, window, camera, target);
            assert!(!has_redraw(&app));
        }
    });
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    assert!(!has_redraw(&app));
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.target == "bevy_widgetry" && r.level == bevy::log::Level::WARN)
            .count(),
        2
    );
}

#[test]
fn theme_selection_preloads_only_static_fallback_and_active_semantics_are_lazy() {
    let mut app = animation_app();
    app.update();
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_handle::<Image>(BuiltinCursorImage::KamuriStaticAtlas.path())
            .is_some()
    );
    for animation in Animation::ALL {
        assert!(
            app.world()
                .resource::<AssetServer>()
                .get_handle::<Image>(animation.path())
                .is_none()
        );
    }
    assert!(!has_redraw(&app));
    let window = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn_empty().id();
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Text))
        .id();
    hover(&mut app, window, camera, target);
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_handle::<Image>(Animation::Text.path())
            .is_some()
    );
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_handle::<Image>(Animation::Wait.path())
            .is_none()
    );
}

#[test]
fn cold_stationary_load_keeps_updates_until_ready_or_failed_and_obeys_focus() {
    for valid in [true, false] {
        // Keep the preregistered PNG loader unavailable until after the first stationary updates.
        let mut app = animation_app_with_loader(false);
        if !valid {
            for path in [
                Animation::Wait.path(),
                BuiltinCursorImage::KamuriStaticAtlas.path(),
            ] {
                app.world()
                    .resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
                    .insert_asset(Default::default(), path.path(), b"invalid PNG".to_vec());
            }
        }
        let window = app.world_mut().spawn(Window::default()).id();
        let camera = app.world_mut().spawn_empty().id();
        let target = app
            .world_mut()
            .spawn(EntityCursor::System(SystemCursorIcon::Wait))
            .id();
        hover(&mut app, window, camera, target);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(SystemCursorIcon::Wait))
        );
        assert!(
            has_redraw(&app),
            "a cold load must schedule another update for a stationary mouse"
        );
        clear_redraw(&mut app);
        tick(&mut app, window, camera, target, 0);
        assert!(has_redraw(&app));
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        clear_redraw(&mut app);
        tick(&mut app, window, camera, target, 0);
        assert!(!has_redraw(&app));
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        app.insert_resource(WidgetryCursorTheme::System);
        tick(&mut app, window, camera, target, 0);
        assert!(!has_redraw(&app));
        app.insert_resource(WidgetryCursorTheme::Kamuri);
        tick(&mut app, window, camera, target, 0);
        assert!(has_redraw(&app));
        app.register_asset_loader(bevy::image::ImageLoader::new(
            bevy::image::CompressedImageFormats::NONE,
        ));
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(
                has_redraw(&app),
                "loading must not sleep before a terminal state"
            );
            clear_redraw(&mut app);
            tick(&mut app, window, camera, target, 0);
            if valid && animation_frame(&app, window, Animation::Wait).is_some()
                || !valid && !has_redraw(&app)
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        if valid {
            assert_eq!(animation_frame(&app, window, Animation::Wait), Some(0));
            tick(&mut app, window, camera, target, 65);
            assert_eq!(animation_frame(&app, window, Animation::Wait), Some(1));
        } else {
            assert_eq!(
                app.world().get::<CursorIcon>(window),
                Some(&CursorIcon::System(SystemCursorIcon::Wait))
            );
            for _ in 0..4 {
                clear_redraw(&mut app);
                tick(&mut app, window, camera, target, 65);
                assert!(
                    !has_redraw(&app),
                    "failed loads must not keep waking the app"
                );
            }
        }
    }
}
