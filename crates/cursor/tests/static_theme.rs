#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
const TEST_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

use bevy::{
    picking::{
        InteractionPlugin, PickingPlugin,
        backend::{HitData, PointerHits},
        cursor::EntityCursor,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::{CursorIcon, CustomCursor, CustomCursorImage, RequestRedraw, SystemCursorIcon},
};
use bevy_widgetry_cursor::*;

fn app() -> (App, Entity, Entity) {
    let mut app = App::new();
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
    app.world_mut().spawn(PointerId::Mouse);
    (app, window, camera)
}
fn hover(app: &mut App, window: Entity, camera: Entity, target: Entity) {
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                .normalize(None)
                .unwrap(),
            position: Vec2::ONE,
        },
        PointerAction::Move { delta: Vec2::ONE },
    ));
    app.world_mut().write_message(PointerHits::new(
        PointerId::Mouse,
        vec![(target, HitData::new(camera, 0.0, None, None))],
        0.0,
    ));
    app.update();
}
fn pixels() -> Image {
    Image::new_fill(
        Extent3d {
            width: 32,
            height: 24,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255; 4],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

#[test]
fn partial_static_theme_maps_one_semantic_and_preserves_fallbacks_and_explicit_images() {
    let (mut app, window, camera) = app();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let image = CursorImage {
        source: CursorImageSource::Handle(handle.clone()),
        rect: Some(URect::new(4, 2, 16, 18)),
        hotspot: (2, 4),
    };
    let theme = WidgetryCursorTheme::new([(
        CursorSemantic::System(SystemCursorIcon::Pointer),
        CursorStyle::Image(image),
    )])
    .unwrap();
    assert!(WidgetryCursorTheme::set_in_world(
        app.world_mut(),
        theme.clone()
    ));
    assert!(!WidgetryCursorTheme::set_in_world(app.world_mut(), theme));
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    hover(&mut app, window, camera, target);
    let native = app
        .world()
        .get::<CursorIcon>(window)
        .and_then(|cursor| {
            if let CursorIcon::Custom(CustomCursor::Image(image)) = cursor {
                Some(image)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(native.handle, handle);
    assert_eq!(native.hotspot, (2, 4));
    assert_eq!(
        native
            .texture_atlas
            .as_ref()
            .unwrap()
            .texture_rect(app.world().resource::<Assets<TextureAtlasLayout>>()),
        Some(URect::new(4, 2, 16, 18))
    );
    for (intent, expected) in [
        (WidgetryCursorHint::ReadOnly, SystemCursorIcon::Cell),
        (WidgetryCursorHint::Panning, SystemCursorIcon::Grabbing),
    ] {
        app.world_mut()
            .entity_mut(target)
            .remove::<EntityCursor>()
            .insert(intent);
        hover(&mut app, window, camera, target);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(expected))
        );
    }
    let explicit = CustomCursor::Image(CustomCursorImage {
        handle,
        hotspot: (1, 1),
        ..default()
    });
    app.world_mut()
        .entity_mut(target)
        .insert(EntityCursor::Custom(explicit.clone()));
    hover(&mut app, window, camera, target);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::Custom(explicit))
    );
    app.world_mut()
        .entity_mut(target)
        .insert(WidgetryCursorLoading);
    hover(&mut app, window, camera, target);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Wait))
    );
    app.world_mut()
        .entity_mut(target)
        .remove::<WidgetryCursorLoading>()
        .insert(bevy::ui::InteractionDisabled);
    hover(&mut app, window, camera, target);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::NotAllowed))
    );
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    app.update();
    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
}

#[test]
fn invalid_static_definitions_are_rejected_before_changing_the_current_theme() {
    let key = CursorSemantic::System(SystemCursorIcon::Pointer);
    for image in [
        CursorImage {
            source: CursorImageSource::Path(String::new()),
            rect: None,
            hotspot: (0, 0),
        },
        CursorImage {
            source: CursorImageSource::Path("example.png".into()),
            rect: Some(URect::new(2, 2, 2, 8)),
            hotspot: (0, 0),
        },
        CursorImage {
            source: CursorImageSource::Path("example.png".into()),
            rect: Some(URect::new(0, 0, 12, 16)),
            hotspot: (12, 0),
        },
    ] {
        assert!(WidgetryCursorTheme::new([(key, CursorStyle::Image(image))]).is_err());
    }
}

#[test]
fn malformed_asset_paths_are_rejected_without_installing_a_theme() {
    let mut world = World::new();
    world.init_resource::<WidgetryCursorTheme>();
    for path in [
        "pointer.png#",
        "://pointer.png",
        "bad#source://image.png",
        "image.png#bad://label",
    ] {
        let result = WidgetryCursorTheme::new([(
            CursorSemantic::System(SystemCursorIcon::Pointer),
            CursorStyle::Image(CursorImage {
                source: CursorImageSource::Path(path.to_owned()),
                rect: None,
                hotspot: (0, 0),
            }),
        )]);
        assert!(result.is_err(), "invalid asset path accepted: {path}");
        let error = result.unwrap_err();
        assert!(
            matches!(error.kind(), CursorThemeError::InvalidImagePath { path: actual, .. } if actual == path)
        );
        assert!(
            std::error::Error::source(error.kind())
                .unwrap()
                .is::<bevy::asset::ParseAssetPathError>()
        );
        assert!(world.resource::<WidgetryCursorTheme>().is_system());
    }
}

#[test]
fn shared_pixels_with_different_hotspots_have_distinct_native_cache_identities() {
    let (mut app, window, camera) = app();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    let theme = |hotspot| {
        WidgetryCursorTheme::new([(
            CursorSemantic::System(SystemCursorIcon::Pointer),
            CursorStyle::Image(CursorImage {
                source: CursorImageSource::Handle(handle.clone()),
                rect: Some(URect::new(4, 2, 16, 18)),
                hotspot,
            }),
        )])
        .unwrap()
    };
    let native_image = |app: &App| {
        app.world()
            .get::<CursorIcon>(window)
            .and_then(|cursor| {
                if let CursorIcon::Custom(CustomCursor::Image(image)) = cursor {
                    Some(image.clone())
                } else {
                    None
                }
            })
            .unwrap()
    };
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme((2, 4)));
    hover(&mut app, window, camera, target);
    let first = native_image(&app);
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme((3, 5)));
    hover(&mut app, window, camera, target);
    let second = native_image(&app);
    assert_eq!(first.handle, second.handle);
    assert_ne!(
        first.texture_atlas, second.texture_atlas,
        "Winit omits hotspot from its native cache key"
    );
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme((2, 4)));
    hover(&mut app, window, camera, target);
    assert_eq!(native_image(&app), first);
}

#[test]
fn system_mapping_and_unavailable_images_work_without_asset_resources() {
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
    let theme = WidgetryCursorTheme::new([
        (
            CursorSemantic::System(SystemCursorIcon::Pointer),
            CursorStyle::Image(CursorImage {
                source: CursorImageSource::Path("external.png".into()),
                rect: None,
                hotspot: (2, 3),
            }),
        ),
        (
            CursorSemantic::Hint(WidgetryCursorHint::ReadOnly),
            CursorStyle::System(SystemCursorIcon::Help),
        ),
    ])
    .unwrap();
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme);
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    hover(&mut app, window, camera, target);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    app.world_mut()
        .entity_mut(target)
        .remove::<EntityCursor>()
        .insert(WidgetryCursorHint::ReadOnly);
    hover(&mut app, window, camera, target);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Help))
    );
    assert!(!app.world().contains_resource::<AssetServer>());
    assert!(!app.world().contains_resource::<Assets<Image>>());
    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
}

#[test]
fn whole_image_hotspot_and_pixel_failures_fall_back_once_stop_waking_and_recover() {
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
    let (mut app, window, camera) = app();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let definition = CursorImage {
        source: CursorImageSource::Handle(handle.clone()),
        rect: None,
        hotspot: (32, 5),
    };
    let theme = WidgetryCursorTheme::new([(
        CursorSemantic::System(SystemCursorIcon::Pointer),
        CursorStyle::Image(definition),
    )])
    .unwrap();
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme);
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    app.edit_schedule(Last, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let logs = LogCapture::default();
    logs.run(|| {
        hover(&mut app, window, camera, target);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(SystemCursorIcon::Pointer))
        );
        let source = logs
            .records()
            .into_iter()
            .find(|record| record.phase() == Some(DiagnosticPhase::Source))
            .unwrap()
            .fields["error_id"]
            .clone();
        for _ in 0..3 {
            hover(&mut app, window, camera, target);
        }
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.matches_field("error_id", &source)
                    && record.phase() == Some(DiagnosticPhase::Source))
                .count(),
            1
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.matches_field("error_id", &source)
                    && record.phase() == Some(DiagnosticPhase::Handled))
                .count(),
            1
        );
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
        let larger = Image::new_fill(
            Extent3d {
                width: 40,
                height: 24,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255; 4],
            TextureFormat::Rgba8UnormSrgb,
            default(),
        );
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(handle.id(), larger)
            .unwrap();
        hover(&mut app, window, camera, target);
        assert!(matches!(
            app.world().get::<CursorIcon>(window),
            Some(CursorIcon::Custom(_))
        ));
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .get_mut(&handle)
            .unwrap()
            .data = None;
        hover(&mut app, window, camera, target);
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(SystemCursorIcon::Pointer))
        );
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    });
}

#[test]
fn queued_theme_changes_and_static_output_are_isolated_between_windows() {
    let (mut app, first, camera) = app();
    let second = app.world_mut().spawn(Window::default()).id();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(pixels());
    let theme = WidgetryCursorTheme::new([(
        CursorSemantic::System(SystemCursorIcon::Pointer),
        CursorStyle::Image(CursorImage {
            source: CursorImageSource::Handle(handle),
            rect: None,
            hotspot: (7, 9),
        }),
    )])
    .unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryCursorTheme::set(&mut Commands::new(&mut queue, app.world()), theme);
    assert!(app.world().resource::<WidgetryCursorTheme>().is_system());
    queue.apply(app.world_mut());
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    hover(&mut app, first, camera, target);
    assert!(matches!(
        app.world().get::<CursorIcon>(first),
        Some(CursorIcon::Custom(_))
    ));
    assert_eq!(
        app.world().get::<CursorIcon>(second),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    hover(&mut app, second, camera, target);
    assert_eq!(
        app.world().get::<CursorIcon>(first),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    assert!(matches!(
        app.world().get::<CursorIcon>(second),
        Some(CursorIcon::Custom(_))
    ));
    app.world_mut().get_mut::<Window>(second).unwrap().focused = false;
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(second),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
}

#[derive(TypePath)]
struct FixtureLoader;
impl bevy::asset::AssetLoader for FixtureLoader {
    type Asset = Image;
    type Settings = ();
    type Error = std::io::Error;
    async fn load(
        &self,
        _: &mut dyn bevy::asset::io::Reader,
        _: &(),
        context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Image, std::io::Error> {
        if context
            .path()
            .path()
            .file_stem()
            .is_some_and(|name| name == "broken")
        {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "cursor fixture data",
            ))
        } else {
            Ok(pixels())
        }
    }
    fn extensions(&self) -> &[&str] {
        &["cur24"]
    }
}

#[test]
fn external_paths_are_lazy_and_stationary_loading_progresses_to_ready_or_failed() {
    use bevy::asset::{
        LoadState,
        io::{
            AssetSourceBuilder, AssetSourceId,
            memory::{Dir, MemoryAssetReader},
        },
    };
    use std::{path::Path, time::Instant};
    let directory = Dir::default();
    directory.insert_asset_text(Path::new("pointer.cur24"), "pixels");
    directory.insert_asset_text(Path::new("broken.cur24"), "bad");
    let mut app = App::new();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(MemoryAssetReader {
                root: directory.clone(),
            })
        }),
    );
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .preregister_asset_loader::<FixtureLoader>(&["cur24"])
        .add_plugins((PickingPlugin, InteractionPlugin, WidgetryCursorPlugin));
    app.update();
    let window = app.world_mut().spawn(Window::default()).id();
    let camera = app.world_mut().spawn_empty().id();
    app.world_mut().spawn(PointerId::Mouse);
    let target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    let image = |path: &str| {
        CursorStyle::Image(CursorImage {
            source: CursorImageSource::Path(path.into()),
            rect: None,
            hotspot: (3, 5),
        })
    };
    let theme = WidgetryCursorTheme::new([
        (
            CursorSemantic::System(SystemCursorIcon::Pointer),
            image("pointer.cur24"),
        ),
        (
            CursorSemantic::System(SystemCursorIcon::Help),
            image("unused.cur24"),
        ),
    ])
    .unwrap();
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme);
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_handle::<Image>("pointer.cur24")
            .is_none()
    );
    hover(&mut app, window, camera, target);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
    let handle = app
        .world()
        .resource::<AssetServer>()
        .get_handle::<Image>("pointer.cur24")
        .unwrap();
    assert!(matches!(
        app.world()
            .resource::<AssetServer>()
            .load_state(handle.id()),
        LoadState::Loading
    ));
    app.register_asset_loader(FixtureLoader);
    let stay = |app: &mut App| {
        app.world_mut().write_message(PointerHits::new(
            PointerId::Mouse,
            vec![(target, HitData::new(camera, 0.0, None, None))],
            0.0,
        ));
        app.update();
    };
    let deadline = Instant::now() + TEST_WAIT_TIMEOUT;
    while !matches!(
        app.world().get::<CursorIcon>(window),
        Some(CursorIcon::Custom(_))
    ) {
        assert!(Instant::now() < deadline);
        stay(&mut app);
        std::thread::yield_now();
    }
    assert!(
        app.world()
            .resource::<AssetServer>()
            .get_handle::<Image>("unused.cur24")
            .is_none()
    );
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    stay(&mut app);
    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    WidgetryCursorTheme::set_in_world(
        app.world_mut(),
        WidgetryCursorTheme::new([(
            CursorSemantic::System(SystemCursorIcon::Pointer),
            image("broken.cur24"),
        )])
        .unwrap(),
    );
    stay(&mut app);
    let broken = app
        .world()
        .resource::<AssetServer>()
        .get_handle::<Image>("broken.cur24")
        .unwrap();
    let deadline = Instant::now() + TEST_WAIT_TIMEOUT;
    while !matches!(
        app.world()
            .resource::<AssetServer>()
            .load_state(broken.id()),
        LoadState::Failed(_)
    ) {
        assert!(Instant::now() < deadline);
        stay(&mut app);
        std::thread::yield_now();
    }
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    stay(&mut app);
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
}
