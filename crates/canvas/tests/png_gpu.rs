#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        pipelined_rendering::PipelinedRenderingPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    window::ExitCondition,
};
use bevy_widgetry_canvas::*;
use std::time::{Duration, Instant};

#[derive(Resource, Default)]
struct Completed(Vec<(Entity, std::result::Result<UVec2, String>)>);

fn frame(app: &mut App) {
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
}

fn finish(app: &mut App, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(35);
    while app.world().resource::<Completed>().0.len() < count {
        assert!(Instant::now() < deadline, "PNG export timed out");
        frame(app);
        std::thread::sleep(Duration::from_millis(1));
    }
    frame(app);
}

fn counts(app: &mut App) -> (usize, usize, usize, usize) {
    (
        app.world().resource::<Assets<Image>>().len(),
        app.world().resource::<Assets<Mesh>>().len(),
        app.world().resource::<Assets<ColorMaterial>>().len(),
        app.world_mut().query::<&Camera>().iter(app.world()).count(),
    )
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_png_rerasterizes_transparent_document_and_preserves_independent_views() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    backends: Some(Backends::DX12),
                    ..default()
                })),
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<PipelinedRenderingPlugin>(),
    );
    app.add_plugins(CanvasPlugin);
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    app.init_resource::<Completed>().add_observer(
        |event: On<CanvasPngExportFinished>,
         mut completed: ResMut<Completed>,
         mut commands: Commands| {
            commands.entity(event.entity).despawn();
            if let Ok(size) = event.result.as_ref() {
                assert_eq!(
                    image::image_dimensions(&event.path).unwrap(),
                    (size.x, size.y)
                );
            }
            completed.0.push((
                event.entity,
                event
                    .result
                    .as_ref()
                    .map(|size| *size)
                    .map_err(ToString::to_string),
            ));
        },
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.plugins_state() != bevy::app::PluginsState::Ready {
        assert!(Instant::now() < deadline, "GPU initialization timed out");
        bevy::tasks::tick_global_task_pools_on_main_thread();
        std::thread::sleep(Duration::from_millis(1));
    }
    app.finish();
    app.cleanup();
    for _ in 0..4 {
        frame(&mut app);
    }
    let document = app.world_mut().spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        size: Vec2::new(120.0, 80.0),
    };
    let transform = Transform::from_xyz(-100.0, -60.0, 0.0);
    let paint = CanvasPaint {
        fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
        stroke: Some(CanvasStroke {
            color: Color::srgba(0.0, 0.0, 1.0, 0.25),
            width: 10.0,
        }),
    };
    let shape =
        CanvasDocument::create_rectangle(app.world_mut(), document, rectangle, transform, paint)
            .unwrap();
    let mut no_plugin = World::new();
    no_plugin.insert_resource(
        app.world()
            .resource::<bevy::render::renderer::RenderDevice>()
            .clone(),
    );
    let unconfigured = no_plugin.spawn(CanvasDocument).id();
    CanvasDocument::create_rectangle(&mut no_plugin, unconfigured, rectangle, transform, paint)
        .unwrap();
    assert!(matches!(
        CanvasDocument::export_png(
            &mut no_plugin,
            unconfigured,
            "unused.png",
            CanvasPngOptions::default()
        ),
        Err(CanvasExportError::RendererUnavailable)
    ));
    let base = counts(&mut app);
    let directory = tempfile::tempdir().unwrap();
    let mut requests = Vec::new();
    for scale in [1_u32, 2, 4] {
        let path = directory.path().join(format!("scale{scale}.png"));
        std::fs::write(&path, b"old file").unwrap();
        requests.push(
            CanvasDocument::export_png(
                app.world_mut(),
                document,
                &path,
                CanvasPngOptions {
                    scale: scale as f32,
                    ..default()
                },
            )
            .unwrap(),
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"old file");
    }
    assert!(app.world().resource::<Completed>().0.is_empty());
    finish(&mut app, 3);
    for (index, scale) in [1_u32, 2, 4].into_iter().enumerate() {
        let result = app
            .world()
            .resource::<Completed>()
            .0
            .iter()
            .find(|r| r.0 == requests[index])
            .unwrap();
        assert_eq!(result.1, Ok(UVec2::new(130 * scale, 90 * scale)));
        let image = image::open(directory.path().join(format!("scale{scale}.png")))
            .unwrap()
            .to_rgba8();
        assert_eq!(image.dimensions(), (130 * scale, 90 * scale));
        for (point, expected) in [
            ((65, 45), [255_u8, 0, 0, 128]),
            ((2, 45), [0, 0, 255, 64]),
            ((7, 45), [203, 0, 170, 159]),
        ] {
            let actual = image.get_pixel(point.0 * scale, point.1 * scale).0;
            for (a, e) in actual.into_iter().zip(expected) {
                assert!(
                    (i16::from(a) - i16::from(e)).abs() <= 3,
                    "pixel {point:?}: {actual:?} expected {expected:?}"
                );
            }
        }
    }
    assert_eq!(counts(&mut app), base);
    let ui_image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            640,
            480,
            bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb,
            None,
        ));
    let ui_camera = app
        .world_mut()
        .spawn((Camera2d, bevy::camera::RenderTarget::Image(ui_image.into())))
        .id();
    let a=app.world_mut().spawn_scene(bsn! { @CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(ui_camera))) Node {width:px(300),height:px(240)} }).unwrap().id();
    let b=app.world_mut().spawn_scene(bsn! { @CanvasView {@document:{Some(document)},@zoom:0.5,@center:Vec2::new(-20.0,30.0)} template(move |_|Ok(UiTargetCamera(ui_camera))) Node {position_type:PositionType::Absolute,left:px(320),width:px(300),height:px(240)} }).unwrap().id();
    for view in [a, b] {
        CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
    }
    for _ in 0..8 {
        frame(&mut app);
    }
    let with_views = counts(&mut app);
    let view_output = directory.path().join("views.png");
    CanvasDocument::export_png(
        app.world_mut(),
        document,
        &view_output,
        CanvasPngOptions::default(),
    )
    .unwrap();
    finish(&mut app, 4);
    assert_eq!(
        image::open(view_output).unwrap().to_rgba8(),
        image::open(directory.path().join("scale1.png"))
            .unwrap()
            .to_rgba8()
    );
    assert_eq!(
        CanvasView::center(app.world(), b).unwrap(),
        Vec2::new(-20.0, 30.0)
    );
    assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 0.5);
    for view in [a, b] {
        assert_eq!(
            CanvasView::selection(app.world(), view).unwrap(),
            Some(shape)
        );
    }
    assert_eq!(counts(&mut app), with_views);
    let protected = directory.path().join("protected.png");
    std::fs::write(&protected, b"preserve").unwrap();
    assert!(matches!(
        CanvasDocument::export_png(
            app.world_mut(),
            document,
            &protected,
            CanvasPngOptions {
                scale: f32::MAX,
                ..default()
            }
        ),
        Err(CanvasExportError::InvalidResolution)
    ));
    assert_eq!(std::fs::read(&protected).unwrap(), b"preserve");
    assert_eq!(counts(&mut app), with_views);
    let outside = directory.path().join("outside.png");
    CanvasDocument::export_png(
        app.world_mut(),
        document,
        &outside,
        CanvasPngOptions {
            region: Some(Rect::from_corners(
                Vec2::splat(1000.0),
                Vec2::new(1013.25, 1009.1),
            )),
            scale: 2.0,
        },
    )
    .unwrap();
    finish(&mut app, 5);
    let transparent = image::open(outside).unwrap().to_rgba8();
    assert_eq!(transparent.dimensions(), (27, 19));
    assert!(transparent.as_raw().iter().all(|v| *v == 0));
    let rotated = transform
        .with_rotation(Quat::from_rotation_z(0.4))
        .with_scale(Vec3::new(-1.2, 0.8, 0.0));
    CanvasDocument::update_rectangle(app.world_mut(), document, shape, rectangle, rotated, paint)
        .unwrap();
    for scale in [1.0, 4.0] {
        CanvasDocument::export_png(
            app.world_mut(),
            document,
            directory.path().join(format!("rotated{scale}.png")),
            CanvasPngOptions { scale, ..default() },
        )
        .unwrap();
    }
    finish(&mut app, 7);
    let low = image::open(directory.path().join("rotated1.png"))
        .unwrap()
        .to_rgba8();
    let high = image::open(directory.path().join("rotated4.png"))
        .unwrap()
        .to_rgba8();
    let bounds = rectangle.bounds(&rotated, &paint).unwrap();
    assert_eq!(
        high.dimensions(),
        (
            (bounds.width() * 4.0).ceil() as u32,
            (bounds.height() * 4.0).ceil() as u32
        )
    );
    assert_eq!(high.get_pixel(0, 0).0, [0, 0, 0, 0]);
    let stretched = image::imageops::resize(
        &low,
        high.width(),
        high.height(),
        image::imageops::FilterType::Nearest,
    );
    assert!(
        high.pixels()
            .zip(stretched.pixels())
            .filter(|(a, b)| a.0 != b.0)
            .count()
            > 100
    );
    CanvasDocument::export_png(
        app.world_mut(),
        document,
        directory.path(),
        CanvasPngOptions::default(),
    )
    .unwrap();
    finish(&mut app, 8);
    assert!(
        app.world()
            .resource::<Completed>()
            .0
            .last()
            .unwrap()
            .1
            .as_ref()
            .unwrap_err()
            .contains("I/O")
    );
    assert_eq!(counts(&mut app), with_views);
    let snapshot_file = directory.path().join("snapshot.png");
    CanvasDocument::update_rectangle(
        app.world_mut(),
        document,
        shape,
        rectangle,
        transform,
        paint,
    )
    .unwrap();
    CanvasDocument::export_png(
        app.world_mut(),
        document,
        &snapshot_file,
        CanvasPngOptions::default(),
    )
    .unwrap();
    CanvasDocument::update_rectangle(
        app.world_mut(),
        document,
        shape,
        rectangle,
        transform,
        CanvasPaint {
            fill: Some(Color::srgb(0.0, 1.0, 0.0)),
            stroke: None,
        },
    )
    .unwrap();
    finish(&mut app, 9);
    assert_eq!(
        image::open(snapshot_file).unwrap().to_rgba8(),
        image::open(directory.path().join("scale1.png"))
            .unwrap()
            .to_rgba8()
    );
    CanvasDocument::remove_rectangle(app.world_mut(), document, shape).unwrap();
    let existing = directory.path().join("scale4.png");
    let original = std::fs::read(&existing).unwrap();
    assert!(matches!(
        CanvasDocument::export_png(
            app.world_mut(),
            document,
            &existing,
            CanvasPngOptions::default()
        ),
        Err(CanvasExportError::EmptyDocument)
    ));
    assert_eq!(std::fs::read(existing).unwrap(), original);
}
