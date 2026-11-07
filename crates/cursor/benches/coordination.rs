use bevy::{
    picking::{
        InteractionPlugin, PickingPlugin,
        backend::{HitData, PointerHits},
        cursor::EntityCursor,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    window::{CursorIcon, SystemCursorIcon, WindowRef},
};
use bevy_widgetry_cursor::{
    CursorAnimation, CursorAnimationFrame, CursorImageSource, CursorSemantic, CursorStyle,
    WidgetryCursorPlugin, WidgetryCursorTheme,
};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run};
use std::hint::black_box;

struct Fixture {
    app: App,
    windows: Vec<Entity>,
    camera: Entity,
    candidates: Vec<Entity>,
    animation_image: Option<Handle<Image>>,
}

fn fixture(windows: usize, candidates: usize, depth: usize, animated: bool) -> Result<Fixture> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut animation_image = None;
    if animated {
        app.add_plugins((
            AssetPlugin::default(),
            bevy::image::ImagePlugin::default(),
            bevy::image::TextureAtlasPlugin,
        ));
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                bevy::render::render_resource::Extent3d {
                    width: 16,
                    height: 16,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                &[255; 4],
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                default(),
            ));
        WidgetryCursorTheme::set_in_world(
            app.world_mut(),
            WidgetryCursorTheme::new(
                [
                    (SystemCursorIcon::Pointer, 2, 70),
                    (SystemCursorIcon::Wait, 4, 65),
                ]
                .into_iter()
                .map(|(icon, count, ms)| {
                    (
                        CursorSemantic::System(icon),
                        CursorStyle::Animation(CursorAnimation {
                            source: CursorImageSource::Handle(image.clone()),
                            size: UVec2::splat(16),
                            frames: (0..count)
                                .map(|index| CursorAnimationFrame {
                                    rect: URect::new(
                                        index % 2 * 8,
                                        index / 2 * 8,
                                        index % 2 * 8 + 8,
                                        index / 2 * 8 + 8,
                                    ),
                                    duration: std::time::Duration::from_millis(ms),
                                })
                                .collect(),
                            hotspot: (2, 3),
                            fallback: None,
                        }),
                    )
                }),
            )
            .map_err(|error| error.into_bevy_error())?,
        );
        animation_image = Some(image);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(125),
        ));
    }
    app.add_plugins((PickingPlugin, InteractionPlugin, WidgetryCursorPlugin));
    app.world_mut().spawn(PointerId::Mouse);
    let windows = (0..windows)
        .map(|_| app.world_mut().spawn(Window::default()).id())
        .collect();
    let camera = app.world_mut().spawn_empty().id();
    let mut entities = Vec::new();
    for _ in 0..candidates {
        let mut parent = app
            .world_mut()
            .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
            .id();
        for _ in 1..depth {
            parent = app.world_mut().spawn(ChildOf(parent)).id();
        }
        app.world_mut().entity_mut(parent).insert(Pickable {
            should_block_lower: false,
            is_hoverable: true,
        });
        entities.push(parent);
    }
    app.update();
    Ok(Fixture {
        app,
        windows,
        camera,
        candidates: entities,
        animation_image,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("cursor-criterion")?;
    for (windows, candidates, depth) in [
        (1, 1, 1),
        (2, 1, 1),
        (8, 1, 1),
        (1, 32, 1),
        (1, 256, 1),
        (1, 1, 16),
        (1, 1, 64),
        (8, 256, 64),
    ] {
        run(
            &mut harness,
            &format!("cursor/w{windows}_h{candidates}_d{depth}/mouse_update"),
            false,
            || fixture(windows, candidates, depth, false),
            |fixture, index| {
                let window = fixture.windows[index % fixture.windows.len()];
                let location = Location {
                    target: bevy::camera::RenderTarget::Window(WindowRef::Entity(window))
                        .normalize(None)
                        .ok_or_else(|| missing("window render target"))?,
                    position: Vec2::splat(20.0),
                };
                fixture.app.world_mut().write_message(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Move { delta: Vec2::ONE },
                ));
                fixture.app.world_mut().write_message(PointerHits::new(
                    PointerId::Mouse,
                    fixture
                        .candidates
                        .iter()
                        .enumerate()
                        .map(|(depth, entity)| {
                            (
                                *entity,
                                HitData::new(fixture.camera, depth as f32, None, None),
                            )
                        })
                        .collect(),
                    0.0,
                ));
                fixture.app.update();
                let cursor = fixture
                    .app
                    .world()
                    .get::<CursorIcon>(window)
                    .ok_or_else(|| missing("window cursor"))?;
                if *cursor != CursorIcon::System(SystemCursorIcon::Pointer) {
                    return Err(BevyError::error(
                        "benchmark Mouse hover did not resolve cursor",
                    ));
                }
                black_box(cursor);
                Ok(())
            },
            |fixture| Ok(fixture.app.world().entities().len()),
        )?;
    }
    for icon in [SystemCursorIcon::Pointer, SystemCursorIcon::Wait] {
        let group = if icon == SystemCursorIcon::Pointer {
            "cursor_animation"
        } else {
            "cursor_wait_animation"
        };
        for windows in [1, 2, 8] {
            for active in [true, false] {
                run(
                    &mut harness,
                    &format!(
                        "{group}/w{windows}/{state}",
                        state = if active { "stationary_frame" } else { "idle" }
                    ),
                    false,
                    || {
                        let mut fixture = fixture(windows, 1, 1, true)?;
                        fixture
                            .app
                            .world_mut()
                            .entity_mut(fixture.candidates[0])
                            .insert(EntityCursor::System(icon));
                        let window = fixture.windows[0];
                        let location = Location {
                            target: bevy::camera::RenderTarget::Window(WindowRef::Entity(window))
                                .normalize(None)
                                .ok_or_else(|| missing("window render target"))?,
                            position: Vec2::splat(20.0),
                        };
                        fixture.app.world_mut().write_message(PointerInput::new(
                            PointerId::Mouse,
                            location,
                            PointerAction::Move { delta: Vec2::ONE },
                        ));
                        let deadline =
                            std::time::Instant::now() + std::time::Duration::from_secs(10);
                        loop {
                            animation_update(&mut fixture)?;
                            if matches!(fixture.app.world().get::<CursorIcon>(window), Some(CursorIcon::Custom(bevy::window::CustomCursor::Image(image))) if image.texture_atlas.is_some() && fixture.animation_image.as_ref() == Some(&image.handle))
                            {
                                break;
                            }
                            if std::time::Instant::now() > deadline {
                                return Err(missing("loaded animation atlas"));
                            }
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        if !active {
                            WidgetryCursorTheme::set_in_world(
                                fixture.app.world_mut(),
                                WidgetryCursorTheme::system(),
                            );
                            animation_update(&mut fixture)?;
                        }
                        Ok(fixture)
                    },
                    |fixture, _| {
                        animation_update(fixture)?;
                        let cursor = fixture
                            .app
                            .world()
                            .get::<CursorIcon>(fixture.windows[0])
                            .ok_or_else(|| missing("window cursor"))?;
                        let animated = matches!(cursor, CursorIcon::Custom(bevy::window::CustomCursor::Image(image)) if image.texture_atlas.is_some() && fixture.animation_image.as_ref() == Some(&image.handle));
                        if animated != active {
                            return Err(missing("expected animation state"));
                        }
                        if !active && *cursor != CursorIcon::System(icon) {
                            return Err(missing("System cursor without animation work"));
                        }
                        black_box(cursor);
                        Ok(())
                    },
                    |fixture| Ok(fixture.app.world().entities().len()),
                )?;
            }
        }
    }
    harness.finish()
}

fn animation_update(fixture: &mut Fixture) -> Result {
    fixture.app.world_mut().write_message(PointerHits::new(
        PointerId::Mouse,
        vec![(
            fixture.candidates[0],
            HitData::new(fixture.camera, 0.0, None, None),
        )],
        0.0,
    ));
    fixture.app.update();
    Ok(())
}
