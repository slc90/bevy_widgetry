//! Coverage Model：窗口光标在 Default、Hover、Dragging 与 Native 间切换。
//! stimuli 为真实 PointerInput、backend PointerHits、Disabled 与 owner/window lifecycle。
//! guards 限定 Mouse、有效窗口、有效 owner 与所属按键，capture 优先于可见 hit。
//! invariants 为窗口隔离、最近声明继承、前方遮挡不泄漏、无变化不写入及 Last 同帧消费。
//! 安装冲突通过宿主 Error handler 和统一日志报告，并停止 Widgetry writer。
//! Theme 与素材 readiness 独立于 hover/lease，Pointer/Wait 按 elapsed time 推进，同语义不重启、相互切换从0开始。
//! 34 种系统语义与10种 hint 保持独立静态格，静态同值不写入、不请求 redraw，素材异常保留语义 fallback。
//! 动画与 Native、blur、Disabled、capture、Custom Pointer 和窗口 lifecycle 交错时停播并恢复正确 fallback。

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

fn animation_app() -> App {
    let mut app = App::new();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
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
    app.register_asset_loader(bevy::image::ImageLoader::new(
        bevy::image::CompressedImageFormats::NONE,
    ));
    app.world_mut().insert_resource(WidgetryCursorTheme::Kamuri);
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
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        hover(&mut app, window, camera, target);
        if frame(&app, window).is_some()
            && app
                .world()
                .resource::<AssetServer>()
                .is_loaded_with_dependencies(
                    app.world()
                        .resource::<AssetServer>()
                        .load::<Image>(
                            bevy_widgetry_asset::BuiltinCursorImage::KamuriWaitAtlas.path(),
                        )
                        .id(),
                )
            && app
                .world()
                .resource::<AssetServer>()
                .is_loaded_with_dependencies(
                    app.world()
                        .resource::<AssetServer>()
                        .load::<Image>(
                            bevy_widgetry_asset::BuiltinCursorImage::KamuriStaticAtlas.path(),
                        )
                        .id(),
                )
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Pointer 未通过 embedded AssetServer 加载"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    (app, window, camera, target)
}

fn frame(app: &App, window: Entity) -> Option<usize> {
    animation_frame(
        app,
        window,
        bevy_widgetry_asset::BuiltinCursorImage::KamuriPointerAtlas,
        (22, 54),
    )
}

fn wait_frame(app: &App, window: Entity) -> Option<usize> {
    animation_frame(
        app,
        window,
        bevy_widgetry_asset::BuiltinCursorImage::KamuriWaitAtlas,
        (25, 47),
    )
}

fn animation_frame(
    app: &App,
    window: Entity,
    id: bevy_widgetry_asset::BuiltinCursorImage,
    hotspot: (u16, u16),
) -> Option<usize> {
    match app.world().get::<CursorIcon>(window) {
        Some(CursorIcon::Custom(bevy::window::CustomCursor::Image(image))) => {
            if image.handle.path() != Some(&id.path()) {
                return None;
            }
            assert_eq!(image.hotspot, hotspot);
            image.texture_atlas.as_ref().map(|atlas| atlas.index)
        }
        _ => None,
    }
}

fn tick(app: &mut App, window: Entity, camera: Entity, target: Entity, millis: u64) {
    app.world_mut()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(millis),
        ));
    app.world_mut().write_message(PointerHits::new(
        PointerId::Mouse,
        vec![(target, HitData::new(camera, 0.0, None, None))],
        0.0,
    ));
    app.update();
    assert!(app.world().get::<CursorIcon>(window).is_some());
}

#[test]
fn pointer_stationary_animation_wraps_without_reset_and_stops_waking_on_exit() {
    let (mut app, window, camera, target) = animation_fixture();
    assert_eq!(frame(&app, window), Some(0));
    app.world_mut().clear_trackers();
    tick(&mut app, window, camera, target, 124);
    assert_eq!(frame(&app, window), Some(0));
    assert!(
        !app.world()
            .entity(window)
            .get_ref::<CursorIcon>()
            .unwrap()
            .is_changed()
    );
    tick(&mut app, window, camera, target, 1);
    assert_eq!(frame(&app, window), Some(1));
    for index in 2..=12 {
        tick(&mut app, window, camera, target, 125);
        assert_eq!(frame(&app, window), Some(index % 12));
    }
    tick(&mut app, window, camera, target, 3_125);
    assert_eq!(frame(&app, window), Some(1));
    assert!(
        !app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Text));
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
    tick(&mut app, window, camera, target, 125);
    assert_eq!(frame(&app, window), None);
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Pointer));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(frame(&app, window), Some(0));
}

#[test]
fn kamuri_all_system_semantics_use_individual_sprites_and_preserve_caller_custom_images() {
    let (mut app, window, camera, target) = animation_fixture();
    use SystemCursorIcon::*;
    for (index, icon) in [
        Default,
        Pointer,
        ContextMenu,
        Help,
        Text,
        VerticalText,
        Cell,
        Crosshair,
        Grab,
        Grabbing,
        Move,
        Copy,
        Alias,
        NoDrop,
        EResize,
        NResize,
        NeResize,
        NwResize,
        SResize,
        SeResize,
        SwResize,
        WResize,
        EwResize,
        NsResize,
        NeswResize,
        NwseResize,
        ColResize,
        RowResize,
        AllScroll,
        ZoomIn,
        ZoomOut,
        Progress,
        Wait,
        NotAllowed,
    ]
    .into_iter()
    .enumerate()
    {
        app.world_mut()
            .entity_mut(target)
            .insert(EntityCursor::System(icon));
        tick(&mut app, window, camera, target, 125);
        if icon == Pointer {
            assert_eq!(frame(&app, window), Some(0));
        } else if icon == Wait {
            assert_eq!(wait_frame(&app, window), Some(0));
        } else {
            assert_static_sprite(&app, window, index);
            let previous = app.world().get::<CursorIcon>(window).unwrap().clone();
            app.world_mut()
                .resource_mut::<Messages<bevy::window::RequestRedraw>>()
                .clear();
            tick(&mut app, window, camera, target, 125);
            assert_eq!(app.world().get::<CursorIcon>(window), Some(&previous));
            assert!(
                !app.world()
                    .entity(window)
                    .get_ref::<CursorIcon>()
                    .unwrap()
                    .is_changed()
            );
            assert!(
                app.world()
                    .resource::<Messages<bevy::window::RequestRedraw>>()
                    .is_empty()
            );
        }
    }
    let custom = bevy::window::CustomCursor::Image(bevy::window::CustomCursorImage {
        handle: Handle::default(),
        hotspot: (2, 3),
        ..default()
    });
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::Custom(custom.clone()));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::Custom(custom))
    );
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
    app.world_mut().insert_resource(WidgetryCursorTheme::System);
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(Pointer));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(Pointer))
    );
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    app.world_mut().insert_resource(WidgetryCursorTheme::Kamuri);
    tick(&mut app, window, camera, target, 125);
    assert_eq!(frame(&app, window), Some(0));
}

#[test]
fn animation_yields_to_native_drag_disabled_capture_and_window_lifecycle() {
    assert_animation_lifecycle(SystemCursorIcon::Pointer);
    assert_animation_lifecycle(SystemCursorIcon::Wait);
}

fn assert_animation_lifecycle(icon: SystemCursorIcon) {
    let (mut app, window, camera, target) = animation_fixture();
    let current_frame = if icon == SystemCursorIcon::Wait {
        wait_frame
    } else {
        frame
    };
    if icon == SystemCursorIcon::Wait {
        app.world_mut()
            .entity_mut(target)
            .insert(EntityCursor::System(icon));
        tick(&mut app, window, camera, target, 0);
    }
    tick(&mut app, window, camera, target, 125);
    assert_eq!(current_frame(&app, window), Some(1));
    app.world_mut().entity_mut(target).insert(
        CursorLease::native(PointerId::Mouse, &location(window), PointerButton::Primary).unwrap(),
    );
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
    app.world_mut().clear_trackers();
    tick(&mut app, window, camera, target, 500);
    assert_eq!(current_frame(&app, window), Some(1));
    assert!(
        !app.world()
            .entity(window)
            .get_ref::<CursorIcon>()
            .unwrap()
            .is_changed()
    );
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Release(PointerButton::Primary),
    ));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(current_frame(&app, window), Some(0));
    app.world_mut().entity_mut(target).insert(
        CursorLease::dragging(
            PointerId::Mouse,
            &location(window),
            PointerButton::Primary,
            SystemCursorIcon::Grabbing,
        )
        .unwrap(),
    );
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 9);
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location(window),
        PointerAction::Cancel,
    ));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(current_frame(&app, window), None);
    assert!(app.world().get::<CursorLease>(target).is_none());
    tick(&mut app, window, camera, target, 125);
    assert_eq!(current_frame(&app, window), Some(0));
    app.world_mut()
        .entity_mut(target)
        .insert(bevy::ui::InteractionDisabled);
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 33);
    app.world_mut()
        .entity_mut(target)
        .remove::<bevy::ui::InteractionDisabled>();
    let captured = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Text))
        .id();
    app.world_mut().resource_mut::<PointerCaptureMap>().capture(
        PointerId::Mouse,
        captured,
        HitData::new(camera, 0.0, None, None),
    );
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 4);
    app.world_mut()
        .resource_mut::<PointerCaptureMap>()
        .release(PointerId::Mouse);
    tick(&mut app, window, camera, target, 125);
    assert_eq!(current_frame(&app, window), Some(0));
    let other = app.world_mut().spawn(Window::default()).id();
    hover(&mut app, other, camera, target);
    assert_eq!(current_frame(&app, other), Some(0));
    assert_eq!(current_frame(&app, window), None);
    hover(&mut app, window, camera, target);
    assert_eq!(current_frame(&app, window), Some(0));
    assert_eq!(current_frame(&app, other), None);
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    tick(&mut app, window, camera, target, 125);
    assert_eq!(current_frame(&app, window), None);
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    tick(&mut app, window, camera, target, 125);
    assert_eq!(current_frame(&app, window), Some(0));
    app.world_mut()
        .write_message(bevy::window::WindowClosed { window });
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
    tick(&mut app, window, camera, target, 125);
    assert_eq!(current_frame(&app, window), None);
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    app.world_mut().despawn(window);
    app.world_mut().despawn(target);
    app.update();
    assert_eq!(current_frame(&app, other), None);
}

#[test]
fn non_mouse_hover_cannot_start_animation_or_acquire_leases() {
    let (mut app, window, camera) = fixture();
    app.world_mut().insert_resource(WidgetryCursorTheme::Kamuri);
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    for pointer in [PointerId::Touch(7), PointerId::Custom(Default::default())] {
        app.world_mut().spawn(pointer);
        app.world_mut().write_message(PointerInput::new(
            pointer,
            location(window),
            PointerAction::Move { delta: Vec2::ONE },
        ));
        app.world_mut().write_message(PointerHits::new(
            pointer,
            vec![(target, HitData::new(camera, 0.0, None, None))],
            0.0,
        ));
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(SystemCursorIcon::Default))
        );
        assert!(
            CursorLease::dragging(
                pointer,
                &location(window),
                PointerButton::Primary,
                SystemCursorIcon::Pointer
            )
            .is_none()
        );
        assert!(CursorLease::native(pointer, &location(window), PointerButton::Primary).is_none());
        assert!(
            app.world()
                .resource::<Messages<bevy::window::RequestRedraw>>()
                .is_empty()
        );
    }
}

#[test]
fn invalid_or_missing_images_use_static_then_system_fallback_and_restart_on_recovery() {
    let (mut app, window, camera, target) = animation_fixture();
    let server = app.world().resource::<AssetServer>();
    let atlas: Handle<Image> =
        server.load(bevy_widgetry_asset::BuiltinCursorImage::KamuriPointerAtlas.path());
    let fallback: Handle<Image> =
        server.load(bevy_widgetry_asset::BuiltinCursorImage::KamuriStaticAtlas.path());
    let saved_atlas = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .remove(atlas.id())
        .unwrap();
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 1);
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    let saved_fallback = app
        .world()
        .resource::<Assets<Image>>()
        .get(&fallback)
        .unwrap()
        .clone();
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .get_mut(&fallback)
        .unwrap()
        .data = None;
    tick(&mut app, window, camera, target, 125);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(atlas.id(), saved_atlas)
        .unwrap();
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(fallback.id(), saved_fallback)
        .unwrap();
    tick(&mut app, window, camera, target, 125);
    assert_eq!(frame(&app, window), Some(0));
}

#[test]
fn failed_png_decodes_warn_once_and_keep_system_pointer_without_waking() {
    let mut app = animation_app();
    app.edit_schedule(Last, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let registry = app
        .world()
        .resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>();
    for id in [
        bevy_widgetry_asset::BuiltinCursorImage::KamuriPointerAtlas,
        bevy_widgetry_asset::BuiltinCursorImage::KamuriStaticAtlas,
    ] {
        registry.insert_asset(
            Default::default(),
            id.path().path(),
            b"invalid PNG".to_vec(),
        );
    }
    let window = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn_empty().id();
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    let atlas: Handle<Image> = app
        .world()
        .resource::<AssetServer>()
        .load(bevy_widgetry_asset::BuiltinCursorImage::KamuriPointerAtlas.path());
    let fallback: Handle<Image> = app
        .world()
        .resource::<AssetServer>()
        .load(bevy_widgetry_asset::BuiltinCursorImage::KamuriStaticAtlas.path());
    let logs = bevy_widgetry_test_utils::LogCapture::default();
    logs.run(|| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            hover(&mut app, window, camera, target);
            let server = app.world().resource::<AssetServer>();
            if [atlas.id(), fallback.id()].into_iter().all(|id| {
                matches!(
                    server.get_load_state(id),
                    Some(bevy::asset::LoadState::Failed(_))
                )
            }) {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        for _ in 0..4 {
            hover(&mut app, window, camera, target);
        }
    });
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    assert_eq!(logs.records().iter().filter(|entry| entry.target == "bevy_widgetry" && entry.level == bevy::log::Level::WARN).count(), 2);
}

#[test]
fn selecting_kamuri_preloads_without_a_mouse_hover_or_redraw_loop() {
    let mut app = animation_app();
    app.world_mut().insert_resource(WidgetryCursorTheme::System);
    app.update();
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_handle::<Image>(bevy_widgetry_asset::BuiltinCursorImage::KamuriPointerAtlas.path())
            .is_none()
    );
    app.world_mut().insert_resource(WidgetryCursorTheme::Kamuri);
    app.update();
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_handle::<Image>(bevy_widgetry_asset::BuiltinCursorImage::KamuriPointerAtlas.path())
            .is_some()
    );
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_handle::<Image>(bevy_widgetry_asset::BuiltinCursorImage::KamuriWaitAtlas.path())
            .is_some()
    );
}

#[test]
fn wait_stationary_loop_and_pointer_transitions_use_independent_animation_origins() {
    let (mut app, window, camera, target) = animation_fixture();
    tick(&mut app, window, camera, target, 125);
    assert_eq!(frame(&app, window), Some(1));
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Wait));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(wait_frame(&app, window), Some(0));
    tick(&mut app, window, camera, target, 124);
    assert_eq!(wait_frame(&app, window), Some(0));
    assert!(
        !app.world()
            .entity(window)
            .get_ref::<CursorIcon>()
            .unwrap()
            .is_changed()
    );
    tick(&mut app, window, camera, target, 1);
    assert_eq!(wait_frame(&app, window), Some(1));
    for i in 2..=24 {
        tick(&mut app, window, camera, target, 125);
        assert_eq!(wait_frame(&app, window), Some(i % 24));
    }
    tick(&mut app, window, camera, target, 3125);
    assert_eq!(wait_frame(&app, window), Some(1));
    assert!(
        !app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Pointer));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(frame(&app, window), Some(0));
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Wait));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(wait_frame(&app, window), Some(0));
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Text));
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 4);
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
}

#[test]
fn unavailable_wait_animation_falls_back_without_affecting_pointer_and_restarts_on_recovery() {
    let (mut app, window, camera, target) = animation_fixture();
    let atlas: Handle<Image> = app
        .world()
        .resource::<AssetServer>()
        .load(bevy_widgetry_asset::BuiltinCursorImage::KamuriWaitAtlas.path());
    let saved = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .remove(atlas.id())
        .unwrap();
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Wait));
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 32);
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
    let fallback: Handle<Image> = app
        .world()
        .resource::<AssetServer>()
        .load(bevy_widgetry_asset::BuiltinCursorImage::KamuriStaticAtlas.path());
    let static_image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .remove(fallback.id())
        .unwrap();
    tick(&mut app, window, camera, target, 125);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Wait))
    );
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Pointer));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(frame(&app, window), Some(0));
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(fallback.id(), static_image)
        .unwrap();
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Wait));
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 32);
    let mut invalid = saved.clone();
    invalid.data = None;
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(atlas.id(), invalid)
        .unwrap();
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 32);
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(atlas.id(), saved)
        .unwrap();
    tick(&mut app, window, camera, target, 125);
    assert_eq!(wait_frame(&app, window), Some(0));
}

fn assert_static_sprite(app: &App, window: Entity, expected_index: usize) {
    let image = app
        .world()
        .get::<CursorIcon>(window)
        .and_then(|cursor| match cursor {
            CursorIcon::Custom(bevy::window::CustomCursor::Image(image)) => Some(image),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        image.handle.path(),
        Some(&bevy_widgetry_asset::BuiltinCursorImage::KamuriStaticAtlas.path())
    );
    assert_eq!(image.texture_atlas.as_ref().unwrap().index, expected_index);
    let size = if matches!(expected_index, 1 | 32) {
        64
    } else {
        48
    };
    assert!(image.hotspot.0 < size && image.hotspot.1 < size);
    let atlas = image.texture_atlas.as_ref().unwrap();
    let layout = app
        .world()
        .resource::<Assets<TextureAtlasLayout>>()
        .get(&atlas.layout)
        .unwrap();
    assert_eq!(
        layout.textures[expected_index].size(),
        UVec2::splat(u32::from(size))
    );
}

#[test]
fn kamuri_extended_semantics_remain_distinct_from_their_system_fallbacks() {
    let (mut app, window, camera, target) = animation_fixture();
    app.world_mut().entity_mut(target).remove::<EntityCursor>();
    use bevy_widgetry_cursor::WidgetryCursorHint::*;
    for (offset, hint) in [
        AddSelection,
        RemoveSelection,
        RangeSelection,
        DragSort,
        InsertDrop,
        ReplaceDrop,
        HorizontalValueAdjust,
        VerticalValueAdjust,
        PanReady,
        Panning,
    ]
    .into_iter()
    .enumerate()
    {
        app.world_mut().entity_mut(target).insert(hint);
        tick(&mut app, window, camera, target, 125);
        assert_static_sprite(&app, window, 34 + offset);
        app.world_mut().insert_resource(WidgetryCursorTheme::System);
        tick(&mut app, window, camera, target, 125);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(hint.system_fallback()))
        );
        app.world_mut().insert_resource(WidgetryCursorTheme::Kamuri);
    }
    let parent = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Help))
        .id();
    app.world_mut()
        .entity_mut(target)
        .insert((ChildOf(parent), AddSelection));
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 34);
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Copy));
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 11);
    app.world_mut()
        .entity_mut(parent)
        .insert(bevy::ui::InteractionDisabled);
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 33);
    app.world_mut()
        .resource_mut::<Messages<bevy::window::RequestRedraw>>()
        .clear();
    tick(&mut app, window, camera, target, 125);
    assert!(
        app.world()
            .resource::<Messages<bevy::window::RequestRedraw>>()
            .is_empty()
    );
}

#[test]
fn unavailable_static_atlas_preserves_the_semantic_system_fallback() {
    let (mut app, window, camera, target) = animation_fixture();
    let handle: Handle<Image> = app
        .world()
        .resource::<AssetServer>()
        .load(bevy_widgetry_asset::BuiltinCursorImage::KamuriStaticAtlas.path());
    let saved = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .remove(handle.id())
        .unwrap();
    app.world_mut()
        .entity_mut(target)
        .remove::<EntityCursor>()
        .insert(bevy_widgetry_cursor::WidgetryCursorHint::AddSelection);
    tick(&mut app, window, camera, target, 125);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Copy))
    );
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::System(SystemCursorIcon::Text));
    tick(&mut app, window, camera, target, 125);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Text))
    );
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(handle.id(), saved)
        .unwrap();
    tick(&mut app, window, camera, target, 125);
    assert_static_sprite(&app, window, 4);
}
