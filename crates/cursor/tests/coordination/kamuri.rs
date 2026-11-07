use super::*;
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
    WidgetryCursorTheme::set_in_world(app.world_mut(), WidgetryCursorTheme::kamuri());
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
        WidgetryCursorTheme::set_in_world(app.world_mut(), WidgetryCursorTheme::system());
        tick(&mut app, window, camera, target, 0);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(hint.system_fallback()))
        );
        WidgetryCursorTheme::set_in_world(app.world_mut(), WidgetryCursorTheme::kamuri());
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
    for theme in [WidgetryCursorTheme::kamuri(), WidgetryCursorTheme::system()] {
        WidgetryCursorTheme::set_in_world(app.world_mut(), theme);
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
        .insert(bevy_widgetry_cursor::WidgetryCursorLoading);
    tick(&mut app, window, camera, target, 0);
    assert_eq!(animation_frame(&app, window, Animation::Wait), Some(0));
    app.world_mut().entity_mut(target).remove::<(
        bevy::ui::InteractionDisabled,
        bevy_widgetry_cursor::WidgetryCursorLoading,
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
fn failed_png_decodes_keep_correlated_source_and_actual_fallback_without_waking() {
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
            .filter(|r| r.matches_field("phase", "source") && r.level == bevy::log::Level::ERROR)
            .count(),
        2
    );
    let records = logs.records();
    let sources: Vec<_> = records
        .iter()
        .filter(|record| record.matches_field("phase", "source"))
        .collect();
    for source in sources {
        let id = source.fields.get("error_id").unwrap();
        assert_eq!(
            records
                .iter()
                .filter(|record| record.fields.get("error_id") == Some(id)
                    && record.matches_field("phase", "handled"))
                .count(),
            1
        );
    }
}

#[test]
fn shared_invalid_animation_reports_one_source_and_only_records_an_actual_static_choice() {
    let (mut app, window, camera, target) = animation_fixture();
    app.edit_schedule(Last, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let other = app.world_mut().spawn(Window::default()).id();
    let atlas: Handle<Image> = app
        .world()
        .resource::<AssetServer>()
        .load(Animation::Pointer.path());
    let saved = app
        .world()
        .resource::<Assets<Image>>()
        .get(&atlas)
        .unwrap()
        .clone();
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .get_mut(&atlas)
        .unwrap()
        .data = Some(vec![0; 3]);
    let logs = bevy_widgetry_test_utils::LogCapture::default();
    logs.run(|| {
        for current in [window, other, window, other] {
            clear_redraw(&mut app);
            hover(&mut app, current, camera, target);
            assert_static_sprite(&app, current, Animation::Pointer);
            assert!(!has_redraw(&app));
        }
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(atlas.id(), saved)
            .unwrap();
        hover(&mut app, window, camera, target);
        assert!(animation_frame(&app, window, Animation::Pointer).is_some());
    });
    let records = logs.records();
    let sources: Vec<_> = records
        .iter()
        .filter(|record| record.matches_field("phase", "source"))
        .collect();
    assert_eq!(sources.len(), 1);
    let source = sources[0];
    let context = source.fields.get("context").unwrap();
    assert!(
        context.contains("expected")
            && context.contains("actual")
            && context.contains("format")
            && context.contains("bytes")
    );
    let id = source.fields.get("error_id").unwrap();
    let handled: Vec<_> = records
        .iter()
        .filter(|record| {
            record.fields.get("error_id") == Some(id) && record.matches_field("phase", "handled")
        })
        .collect();
    assert_eq!(handled.len(), 1);
    assert!(
        handled[0]
            .fields
            .get("outcome")
            .unwrap()
            .contains("static atlas=")
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| record.fields.get("error_id") == Some(id)
                && record.matches_field("phase", "recovered"))
            .count(),
        1
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
        WidgetryCursorTheme::set_in_world(app.world_mut(), WidgetryCursorTheme::system());
        tick(&mut app, window, camera, target, 0);
        assert!(!has_redraw(&app));
        WidgetryCursorTheme::set_in_world(app.world_mut(), WidgetryCursorTheme::kamuri());
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
