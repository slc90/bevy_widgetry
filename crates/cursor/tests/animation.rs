#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    picking::{
        InteractionPlugin, PickingPlugin,
        backend::{HitData, PointerHits},
        cursor::EntityCursor,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    time::TimeUpdateStrategy,
    window::{CursorIcon, CustomCursor, RequestRedraw, SystemCursorIcon},
};
use bevy_widgetry_cursor::*;
use std::time::Duration;

fn app() -> (App, Entity, Entity, Entity) {
    fixture(App::new())
}

fn fixture(mut app: App) -> (App, Entity, Entity, Entity) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        PickingPlugin,
        InteractionPlugin,
        WidgetryCursorPlugin,
    ));
    app.update();
    let window = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn_empty().id();
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    app.world_mut().spawn(PointerId::Mouse);
    (app, window, camera, target)
}

fn pixels() -> Image {
    Image::new_fill(
        Extent3d {
            width: 12,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255; 4],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

fn definition(source: CursorImageSource) -> CursorAnimation {
    CursorAnimation {
        source,
        size: UVec2::new(12, 4),
        frames: [(8, 10), (0, 20), (4, 30)]
            .into_iter()
            .map(|(x, ms)| CursorAnimationFrame {
                rect: URect::new(x, 0, x + 4, 4),
                duration: Duration::from_millis(ms),
            })
            .collect(),
        hotspot: (2, 1),
        fallback: None,
    }
}

fn theme(animation: CursorAnimation) -> WidgetryCursorTheme {
    WidgetryCursorTheme::new([(
        CursorSemantic::System(SystemCursorIcon::Pointer),
        CursorStyle::Animation(animation),
    )])
    .unwrap()
}

fn tick(app: &mut App, window: Entity, camera: Entity, target: Entity, ms: u64) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        ms,
    )));
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                .normalize(None)
                .unwrap(),
            position: Vec2::ONE,
        },
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    app.world_mut().write_message(PointerHits::new(
        PointerId::Mouse,
        vec![(target, HitData::new(camera, 0.0, None, None))],
        0.0,
    ));
    app.update();
}

fn frame(app: &App, window: Entity) -> Option<(Handle<Image>, usize)> {
    let CursorIcon::Custom(CustomCursor::Image(image)) = app.world().get::<CursorIcon>(window)?
    else {
        return None;
    };
    assert_eq!(image.hotspot, (2, 1));
    Some((image.handle.clone(), image.texture_atlas.as_ref()?.index))
}

#[test]
fn external_atlas_uses_real_time_order_and_durations_without_restarting_same_style() {
    let (mut app, window, camera, target) = app();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let animation = definition(CursorImageSource::Handle(handle.clone()));
    assert_eq!(animation.frame_at(Duration::ZERO), Some(0));
    assert_eq!(animation.frame_at(Duration::from_millis(10)), Some(1));
    assert_eq!(animation.frame_at(Duration::from_millis(30)), Some(2));
    assert_eq!(animation.frame_at(Duration::from_millis(60)), Some(0));
    assert!(animation.frame_at(Duration::MAX).unwrap() < 3);
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme(animation.clone()));
    tick(&mut app, window, camera, target, 0);
    assert_eq!(frame(&app, window), Some((handle.clone(), 0)));
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    tick(&mut app, window, camera, target, 10);
    assert_eq!(frame(&app, window), Some((handle.clone(), 1)));
    assert!(!WidgetryCursorTheme::set_in_world(
        app.world_mut(),
        theme(animation)
    ));
    tick(&mut app, window, camera, target, 20);
    assert_eq!(frame(&app, window), Some((handle.clone(), 2)));
    tick(&mut app, window, camera, target, 1030);
    assert_eq!(frame(&app, window), Some((handle, 2)));
}

#[test]
fn animation_definition_rejects_invalid_timing_geometry_and_sources() {
    let valid = definition(CursorImageSource::Path("atlas.png".into()));
    let mut cases = Vec::new();
    let mut bad = valid.clone();
    bad.frames.clear();
    cases.push(bad);
    let mut bad = valid.clone();
    bad.frames[0].duration = Duration::ZERO;
    cases.push(bad);
    let mut bad = valid.clone();
    bad.frames[0].duration = Duration::MAX;
    cases.push(bad);
    let mut bad = valid.clone();
    bad.frames[0].rect = URect::new(9, 0, 13, 4);
    cases.push(bad);
    let mut bad = valid.clone();
    bad.frames[0].rect = URect::new(8, 0, 11, 4);
    cases.push(bad);
    let mut bad = valid.clone();
    bad.hotspot = (4, 1);
    cases.push(bad);
    let mut bad = valid.clone();
    bad.source = CursorImageSource::Path("atlas.png#".into());
    cases.push(bad);
    let mut bad = valid.clone();
    bad.fallback = Some(CursorImage {
        source: CursorImageSource::Path("bad.png#".into()),
        rect: None,
        hotspot: (0, 0),
    });
    cases.push(bad);
    for (index, animation) in cases.into_iter().enumerate() {
        let error = WidgetryCursorTheme::new([(
            CursorSemantic::System(SystemCursorIcon::Pointer),
            CursorStyle::Animation(animation),
        )])
        .unwrap_err();
        assert!(match (index, error.kind()) {
            (0, CursorThemeError::EmptyAnimation) => true,
            (1, CursorThemeError::InvalidFrameDuration { frame: 0 }) => true,
            (2, CursorThemeError::AnimationDurationOverflow) => true,
            (3, CursorThemeError::UnavailableRegion { .. }) => true,
            (4, CursorThemeError::InconsistentFrameSize { .. }) => true,
            (
                5,
                CursorThemeError::InvalidHotspot {
                    hotspot: (4, 1), ..
                },
            ) => true,
            (6 | 7, CursorThemeError::InvalidImagePath { .. }) =>
                std::error::Error::source(error.kind())
                    .is_some_and(|source| source.is::<bevy::asset::ParseAssetPathError>()),
            _ => false,
        });
    }
    assert!(
        WidgetryCursorTheme::new([(
            CursorSemantic::System(SystemCursorIcon::Pointer),
            CursorStyle::Animation(valid)
        )])
        .is_ok()
    );
}

#[test]
fn an_atlas_size_failure_is_not_recovered_by_its_same_image_static_fallback() {
    let (mut app, window, camera, target) = app();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let mut animation = definition(CursorImageSource::Handle(handle.clone()));
    animation.size.x = 16;
    animation.fallback = Some(CursorImage {
        source: CursorImageSource::Handle(handle),
        rect: Some(URect::new(8, 0, 12, 4)),
        hotspot: (2, 1),
    });
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme(animation));
    app.edit_schedule(Last, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let logs = bevy_widgetry_test_utils::LogCapture::default();
    logs.run(|| {
        for _ in 0..4 {
            tick(&mut app, window, camera, target, 10);
        }
    });
    let records = logs.records();
    assert_eq!(
        records
            .iter()
            .filter(|r| r.matches_field("phase", "source"))
            .count(),
        1
    );
    assert_eq!(
        records
            .iter()
            .filter(|r| r.matches_field("phase", "handled"))
            .count(),
        1
    );
    assert_eq!(
        records
            .iter()
            .filter(|r| r.matches_field("phase", "recovered"))
            .count(),
        0
    );
}

fn clear_redraw(app: &mut App) {
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
}
fn wakes(app: &App) -> bool {
    !app.world().resource::<Messages<RequestRedraw>>().is_empty()
}

#[test]
fn replacing_the_animation_restarts_and_focus_native_cancel_and_system_stop_waking() {
    use bevy::picking::pointer::PointerButton;
    let (mut app, window, camera, target) = app();
    let first = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let second = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    WidgetryCursorTheme::set_in_world(
        app.world_mut(),
        theme(definition(CursorImageSource::Handle(first.clone()))),
    );
    tick(&mut app, window, camera, target, 0);
    tick(&mut app, window, camera, target, 30);
    assert_eq!(frame(&app, window), Some((first, 2)));
    let next = theme(definition(CursorImageSource::Handle(second.clone())));
    WidgetryCursorTheme::set_in_world(app.world_mut(), next.clone());
    tick(&mut app, window, camera, target, 20);
    assert_eq!(frame(&app, window), Some((second.clone(), 0)));
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    clear_redraw(&mut app);
    tick(&mut app, window, camera, target, 50);
    assert!(!wakes(&app));
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    tick(&mut app, window, camera, target, 0);
    assert_eq!(frame(&app, window), Some((second.clone(), 0)));
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::ONE,
    };
    app.world_mut()
        .entity_mut(target)
        .insert(CursorLease::native(PointerId::Mouse, &location, PointerButton::Primary).unwrap());
    clear_redraw(&mut app);
    tick(&mut app, window, camera, target, 10);
    assert!(!wakes(&app));
    app.world_mut().entity_mut(target).remove::<CursorLease>();
    tick(&mut app, window, camera, target, 0);
    assert!(wakes(&app));
    clear_redraw(&mut app);
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Cancel,
    ));
    app.update();
    assert!(!wakes(&app));
    WidgetryCursorTheme::set_in_world(app.world_mut(), WidgetryCursorTheme::system());
    clear_redraw(&mut app);
    tick(&mut app, window, camera, target, 20);
    assert!(!wakes(&app));
    WidgetryCursorTheme::set_in_world(app.world_mut(), next);
    tick(&mut app, window, camera, target, 0);
    assert_eq!(frame(&app, window), Some((second, 0)));
    app.world_mut().entity_mut(window).despawn();
    clear_redraw(&mut app);
    app.update();
    assert!(!wakes(&app));
}

#[test]
fn window_entry_starts_its_own_phase_and_a_valid_lease_survives_missing_hover() {
    use bevy::picking::pointer::PointerButton;
    let (mut app, first, camera, target) = app();
    let second = app.world_mut().spawn(Window::default()).id();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    WidgetryCursorTheme::set_in_world(
        app.world_mut(),
        theme(definition(CursorImageSource::Handle(handle.clone()))),
    );
    let mut pressed = ButtonInput::<MouseButton>::default();
    pressed.press(MouseButton::Left);
    app.insert_resource(pressed);
    let lease = |window| {
        CursorLease::dragging(
            PointerId::Mouse,
            &Location {
                target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                    .normalize(None)
                    .unwrap(),
                position: Vec2::ONE,
            },
            PointerButton::Primary,
            SystemCursorIcon::Pointer,
        )
        .unwrap()
    };
    let owner1 = app.world_mut().spawn(lease(first)).id();
    tick(&mut app, first, camera, target, 0);
    tick(&mut app, first, camera, target, 10);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        20,
    )));
    app.update();
    assert_eq!(frame(&app, first), Some((handle.clone(), 2)));
    let owner2 = app.world_mut().spawn(lease(second)).id();
    tick(&mut app, second, camera, target, 0);
    assert!(frame(&app, first).is_none());
    assert_eq!(frame(&app, second), Some((handle.clone(), 0)));
    tick(&mut app, second, camera, target, 10);
    assert_eq!(frame(&app, second), Some((handle, 1)));
    app.world_mut().entity_mut(owner1).despawn();
    app.world_mut().entity_mut(owner2).despawn();
    clear_redraw(&mut app);
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(first))
                .normalize(None)
                .unwrap(),
            position: Vec2::ONE,
        },
        PointerAction::Cancel,
    ));
    app.update();
    assert!(!wakes(&app));
}

#[test]
fn failed_pixels_fall_back_without_waking_and_restart_after_recovery() {
    let (mut app, window, camera, target) = app();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let fallback = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let mut animation = definition(CursorImageSource::Handle(handle.clone()));
    animation.fallback = Some(CursorImage {
        source: CursorImageSource::Handle(fallback.clone()),
        rect: Some(URect::new(0, 0, 4, 4)),
        hotspot: (2, 1),
    });
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme(animation));
    tick(&mut app, window, camera, target, 0);
    for broken in [None, Some(vec![0; 3])] {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .get_mut(&handle)
            .unwrap()
            .data = broken;
        clear_redraw(&mut app);
        tick(&mut app, window, camera, target, 20);
        assert_eq!(frame(&app, window), Some((fallback.clone(), 0)));
        assert!(!wakes(&app));
    }
    let mut bad_format = pixels();
    bad_format.texture_descriptor.format = TextureFormat::Bgra8Unorm;
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(handle.id(), bad_format)
        .unwrap();
    clear_redraw(&mut app);
    tick(&mut app, window, camera, target, 20);
    assert!(!wakes(&app));
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .get_mut(&fallback)
        .unwrap()
        .data = None;
    clear_redraw(&mut app);
    tick(&mut app, window, camera, target, 20);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    assert!(!wakes(&app));
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(handle.id(), pixels())
        .unwrap();
    tick(&mut app, window, camera, target, 100);
    assert_eq!(frame(&app, window), Some((handle, 0)));
    assert!(wakes(&app));
}

#[derive(TypePath)]
struct AtlasLoader;
impl bevy::asset::AssetLoader for AtlasLoader {
    type Asset = Image;
    type Settings = ();
    type Error = std::io::Error;
    async fn load(
        &self,
        _: &mut dyn bevy::asset::io::Reader,
        _: &(),
        context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        if context
            .path()
            .path()
            .file_stem()
            .is_some_and(|name| name == "broken")
        {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "cursor animation fixture",
            ))
        } else {
            Ok(pixels())
        }
    }
    fn extensions(&self) -> &[&str] {
        &["a25"]
    }
}

#[test]
fn stationary_cold_paths_load_only_active_atlases_and_stop_waking_on_failure() {
    use bevy::asset::{
        LoadState,
        io::{
            AssetSourceBuilder, AssetSourceId,
            memory::{Dir, MemoryAssetReader},
        },
    };
    use std::{path::Path, time::Instant};
    for valid in [true, false] {
        let dir = Dir::default();
        for name in ["active.a25", "broken.a25", "fallback.a25", "unused.a25"] {
            dir.insert_asset_text(Path::new(name), "image");
        }
        let mut initial = App::new();
        initial.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || Box::new(MemoryAssetReader { root: dir.clone() })),
        );
        let (mut app, window, camera, target) = fixture(initial);
        app.preregister_asset_loader::<AtlasLoader>(&["a25"]);
        let path = if valid { "active.a25" } else { "broken.a25" };
        let mut animation = definition(CursorImageSource::Path(path.into()));
        animation.fallback = Some(CursorImage {
            source: CursorImageSource::Path("fallback.a25".into()),
            rect: Some(URect::new(0, 0, 4, 4)),
            hotspot: (2, 1),
        });
        WidgetryCursorTheme::set_in_world(
            app.world_mut(),
            WidgetryCursorTheme::new([
                (
                    CursorSemantic::System(SystemCursorIcon::Pointer),
                    CursorStyle::Animation(animation),
                ),
                (
                    CursorSemantic::System(SystemCursorIcon::Help),
                    CursorStyle::Animation(definition(CursorImageSource::Path(
                        "unused.a25".into(),
                    ))),
                ),
            ])
            .unwrap(),
        );
        assert!(
            app.world()
                .resource::<AssetServer>()
                .get_handle::<Image>(path)
                .is_none()
        );
        tick(&mut app, window, camera, target, 0);
        assert!(wakes(&app));
        let primary = app
            .world()
            .resource::<AssetServer>()
            .get_handle::<Image>(path)
            .unwrap();
        assert!(matches!(
            app.world()
                .resource::<AssetServer>()
                .load_state(primary.id()),
            LoadState::Loading
        ));
        app.register_asset_loader(AtlasLoader);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            clear_redraw(&mut app);
            app.world_mut().write_message(PointerHits::new(
                PointerId::Mouse,
                vec![(target, HitData::new(camera, 0.0, None, None))],
                0.0,
            ));
            app.update();
            let ready = frame(&app, window);
            if valid && ready.as_ref().is_some_and(|(handle, _)| handle == &primary)
                || !valid && ready.is_some() && !wakes(&app)
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(
            app.world()
                .resource::<AssetServer>()
                .get_handle::<Image>("unused.a25")
                .is_none()
        );
        if valid {
            assert!(wakes(&app));
        } else {
            assert!(matches!(
                app.world()
                    .resource::<AssetServer>()
                    .load_state(primary.id()),
                LoadState::Failed(_)
            ));
            for _ in 0..3 {
                clear_redraw(&mut app);
                app.update();
                assert!(!wakes(&app));
            }
        }
    }
}
