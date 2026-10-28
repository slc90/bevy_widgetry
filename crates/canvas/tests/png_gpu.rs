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
fn gpu_high_resolution_tiles_keep_request_snapshots_and_release_all_resources() {
    let mut app = gpu_app();
    let directory = tempfile::tempdir().unwrap();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        size: Vec2::new(1920.0, 1080.0),
        corner_radius: 0.0,
    };
    let transform = Transform::from_xyz(960.0, 540.0, 0.0);
    let paint = CanvasPaint {
        fill: Some(Color::srgba(0.2, 0.5, 0.8, 0.5)),
        ..default()
    };
    let shape =
        CanvasDocument::create_rectangle(app.world_mut(), document, rectangle, transform, paint)
            .unwrap();
    for (index, scale) in [1.0, 2.0, 4.0].into_iter().enumerate() {
        CanvasDocument::update_rectangle(
            app.world_mut(),
            document,
            shape,
            rectangle,
            transform,
            paint,
        )
        .unwrap();
        frame(&mut app);
        let baseline = counts(&mut app);
        let path = directory.path().join(format!("{scale}x.png"));
        let started = Instant::now();
        CanvasDocument::export_png(
            app.world_mut(),
            document,
            &path,
            CanvasPngOptions {
                region: Some(Rect::from_corners(Vec2::ZERO, rectangle.size)),
                scale,
            },
        )
        .unwrap();
        CanvasDocument::update_rectangle(
            app.world_mut(),
            document,
            shape,
            rectangle,
            transform,
            CanvasPaint {
                fill: Some(Color::WHITE),
                ..default()
            },
        )
        .unwrap();
        finish(&mut app, index + 1);
        let size = UVec2::new((1920.0 * scale) as u32, (1080.0 * scale) as u32);
        assert_eq!(app.world().resource::<Completed>().0[index].1, Ok(size));
        let png = image::open(&path).unwrap().into_rgba8();
        for (x, y) in [
            (50, 50),
            (size.x / 2, size.y / 2),
            (size.x - 50, size.y - 50),
            (size.x.min(2050) - 1, 511),
        ] {
            let p = png.get_pixel(x, y).0;
            assert!((126..=129).contains(&p[3]));
            assert!(p[2] > p[1] && p[1] > p[0]);
        }
        assert_eq!(counts(&mut app), baseline);
        eprintln!(
            "DX12 {}x{} request-to-notification {:?}; render assets reclaimed",
            size.x,
            size.y,
            started.elapsed()
        );
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_empty_paints_clear_views_and_export_transparent_regions_without_waiting_for_meshes() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let directory = tempfile::tempdir().unwrap();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let empty = CanvasPath {
        commands: vec![MoveTo(Vec2::ZERO)],
    };
    let square = CanvasPath {
        commands: vec![
            MoveTo(Vec2::splat(-30.0)),
            LineTo(Vec2::new(30.0, -30.0)),
            LineTo(Vec2::splat(30.0)),
            LineTo(Vec2::new(-30.0, 30.0)),
            Close,
        ],
    };
    let line = CanvasPath {
        commands: vec![MoveTo(Vec2::new(-30.0, 0.0)), LineTo(Vec2::new(30.0, 0.0))],
    };
    let paint = CanvasPaint {
        fill: Some(Color::WHITE),
        stroke: Some(CanvasStroke {
            width: 6.0,
            ..default()
        }),
        ..default()
    };
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        empty.clone(),
        Transform::default(),
        paint,
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    for (step, path) in [square.clone(), empty.clone(), square, line, empty]
        .into_iter()
        .enumerate()
    {
        CanvasDocument::update_path(
            app.world_mut(),
            document,
            shape,
            path,
            Transform::default(),
            paint,
        )
        .unwrap();
        frame(&mut app);
        let view = capture(&mut app, camera);
        let path = directory.path().join(format!("empty-{step}.png"));
        CanvasDocument::export_png(
            app.world_mut(),
            document,
            &path,
            CanvasPngOptions {
                region: Some(Rect::from_corners(Vec2::splat(-128.0), Vec2::splat(128.0))),
                scale: 1.0,
            },
        )
        .unwrap();
        finish(&mut app, step + 1);
        assert!(app.world().resource::<Completed>().0[step].1.is_ok());
        let png = image::open(path).unwrap().to_rgba8();
        if step == 1 || step == 4 {
            assert!(view.pixels().all(|p| p.0[3] == 0));
            assert!(png.pixels().all(|p| p.0[3] == 0));
        } else {
            assert_eq!(view.get_pixel(128, 128).0[3], 255);
            assert_eq!(png.get_pixel(128, 128), view.get_pixel(128, 128));
            assert_eq!(png.get_pixel(128, 140), view.get_pixel(128, 140));
            assert_eq!(
                png.get_pixel(128, 140).0[3],
                if step == 3 { 0 } else { 255 }
            );
        }
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_mixed_exports_match_views_and_svg_keep_holes_and_own_their_snapshots() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let directory = tempfile::tempdir().unwrap();
    let region = Rect::from_corners(Vec2::splat(-128.0), Vec2::splat(128.0));
    for (case, rule) in [CanvasFillRule::EvenOdd, CanvasFillRule::NonZero]
        .into_iter()
        .enumerate()
    {
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            CanvasRectangle {
                size: Vec2::new(35.0, 25.0),
                corner_radius: 8.0,
            },
            Transform::from_xyz(-80.0, -70.0, 9000.0).with_rotation(Quat::from_rotation_z(0.4)),
            CanvasPaint {
                fill: Some(Color::WHITE),
                ..default()
            },
        )
        .unwrap();
        let ellipse = CanvasDocument::create_ellipse(
            app.world_mut(),
            document,
            CanvasEllipse {
                radii: Vec2::new(65.0, 55.0),
            },
            Transform::from_xyz(0.0, 0.0, 9000.0),
            CanvasPaint {
                fill: Some(Color::srgb(1.0, 0.5, 0.0)),
                stroke: Some(CanvasStroke {
                    width: 3.0,
                    ..default()
                }),
                ..default()
            },
        )
        .unwrap();
        let path = CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    MoveTo(Vec2::new(-45.0, 0.0)),
                    ArcTo {
                        radii: Vec2::splat(45.0),
                        rotation: 0.0,
                        large_arc: false,
                        sweep: true,
                        to: Vec2::new(45.0, 0.0),
                    },
                    ArcTo {
                        radii: Vec2::splat(45.0),
                        rotation: 0.0,
                        large_arc: false,
                        sweep: true,
                        to: Vec2::new(-45.0, 0.0),
                    },
                    Close,
                    MoveTo(Vec2::new(-20.0, 0.0)),
                    ArcTo {
                        radii: Vec2::splat(20.0),
                        rotation: 0.0,
                        large_arc: false,
                        sweep: true,
                        to: Vec2::new(20.0, 0.0),
                    },
                    ArcTo {
                        radii: Vec2::splat(20.0),
                        rotation: 0.0,
                        large_arc: false,
                        sweep: true,
                        to: Vec2::new(-20.0, 0.0),
                    },
                    Close,
                    MoveTo(Vec2::new(-80.0, 85.0)),
                    LineTo(Vec2::new(-60.0, 85.0)),
                    QuadTo {
                        control: Vec2::new(-40.0, 45.0),
                        to: Vec2::new(-20.0, 85.0),
                    },
                    CubicTo {
                        control1: Vec2::new(0.0, 115.0),
                        control2: Vec2::new(20.0, 55.0),
                        to: Vec2::new(40.0, 85.0),
                    },
                    ArcTo {
                        radii: Vec2::new(20.0, 10.0),
                        rotation: 0.4,
                        large_arc: true,
                        sweep: false,
                        to: Vec2::new(80.0, 85.0),
                    },
                ],
            },
            Transform::from_xyz(0.0, 0.0, -9000.0).with_scale(Vec3::new(-1.0, 1.0, 0.0)),
            CanvasPaint {
                fill: Some(Color::srgba(0.0, 0.0, 1.0, 0.5)),
                fill_rule: rule,
                stroke: Some(CanvasStroke {
                    color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                    width: 4.0,
                    cap: CanvasLineCap::Round,
                    join: CanvasLineJoin::Bevel,
                }),
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let secondary = view_camera(&mut app, document);
        let view = capture(&mut app, camera);
        let roots = app
            .world_mut()
            .query::<(Entity, &ViewportNode)>()
            .iter(app.world())
            .filter_map(|(e, n)| {
                [Some(camera), Some(secondary)]
                    .contains(&n.camera)
                    .then_some(e)
            })
            .collect::<Vec<_>>();
        assert_eq!(roots.len(), 2);
        CanvasView::set_selection_in_world(app.world_mut(), roots[0], [ellipse, path]).unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), roots[1], Some(path)).unwrap();
        CanvasView::set_center_in_world(app.world_mut(), roots[1], Vec2::new(30.0, -20.0)).unwrap();
        CanvasView::set_zoom_in_world(app.world_mut(), roots[1], 2.0).unwrap();
        for _ in 0..3 {
            frame(&mut app);
        }
        let selections = roots
            .iter()
            .map(|root| CanvasView::selection(app.world(), *root).unwrap())
            .collect::<Vec<_>>();
        let base = counts(&mut app);
        let svg = directory.path().join("混合.svg");
        CanvasDocument::export_svg(app.world_mut(), document, &svg, Some(region)).unwrap();
        let tree = resvg::usvg::Tree::from_data(
            &std::fs::read(svg).unwrap(),
            &resvg::usvg::Options::default(),
        )
        .unwrap();
        let mut raster = resvg::tiny_skia::Pixmap::new(256, 256).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut raster.as_mut(),
        );
        let before = app.world().resource::<Completed>().0.len();
        for scale in [1.0, 2.0] {
            CanvasDocument::export_png(
                app.world_mut(),
                document,
                directory.path().join(format!("混合😀-{case}-{scale}.png")),
                CanvasPngOptions {
                    region: Some(region),
                    scale,
                },
            )
            .unwrap();
        }
        CanvasDocument::update_transform(
            app.world_mut(),
            document,
            path,
            Transform::from_xyz(500.0, 500.0, 1.0),
        )
        .unwrap();
        CanvasDocument::reorder_shape(app.world_mut(), document, ellipse, CanvasOrder::Front)
            .unwrap();
        finish(&mut app, before + 2);
        for scale in [1_u32, 2] {
            let exported = image::open(directory.path().join(format!("混合😀-{case}-{scale}.png")))
                .unwrap()
                .to_rgba8();
            assert_eq!(exported.dimensions(), (256 * scale, 256 * scale));
            for (x, y) in [
                (128, 128),
                (160, 128),
                (188, 128),
                (48, 58),
                (8, 8),
                (188, 213),
                (198, 213),
            ] {
                let actual = exported.get_pixel(x * scale, y * scale).0;
                let expected = view.get_pixel(x, y).0;
                assert!(
                    actual
                        .into_iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(b) <= 2),
                    "{case} {scale} ({x},{y}): {actual:?} != {expected:?}"
                );
                assert!(actual[3].abs_diff(raster.pixel(x, y).unwrap().alpha()) <= 2);
            }
            if rule == CanvasFillRule::EvenOdd {
                assert_eq!(
                    exported.get_pixel(128 * scale, 128 * scale).0,
                    [255, 128, 0, 255]
                );
            } else {
                assert!(exported.get_pixel(128 * scale, 128 * scale).0[2] > 170);
            }
        }
        assert_eq!(counts(&mut app), base);
        assert_eq!(
            CanvasView::center(app.world(), roots[1]).unwrap(),
            Vec2::new(30.0, -20.0)
        );
        assert_eq!(CanvasView::zoom(app.world(), roots[1]).unwrap(), 2.0);
        assert_eq!(
            roots
                .iter()
                .map(|root| CanvasView::selection(app.world(), *root).unwrap())
                .collect::<Vec<_>>(),
            selections
        );
        let reordered = directory.path().join(format!("reordered-{case}.png"));
        CanvasDocument::update_transform(
            app.world_mut(),
            document,
            path,
            Transform::from_xyz(0.0, 0.0, -9000.0).with_scale(Vec3::new(-1.0, 1.0, 0.0)),
        )
        .unwrap();
        CanvasDocument::export_png(
            app.world_mut(),
            document,
            &reordered,
            CanvasPngOptions {
                region: Some(region),
                scale: 1.0,
            },
        )
        .unwrap();
        finish(&mut app, before + 3);
        assert_eq!(
            image::open(reordered)
                .unwrap()
                .to_rgba8()
                .get_pixel(160, 128)
                .0,
            [255, 128, 0, 255]
        );
        for root in roots {
            app.world_mut().despawn(root);
        }
        frame(&mut app);
    }
    let document = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::splat(-20.0)),
                LineTo(Vec2::new(20.0, -20.0)),
                LineTo(Vec2::splat(20.0)),
                LineTo(Vec2::new(-20.0, 20.0)),
                Close,
                MoveTo(Vec2::splat(1e12)),
                LineTo(Vec2::new(1e12, 1e12 + 1e6)),
            ],
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let base = counts(&mut app);
    let before = app.world().resource::<Completed>().0.len();
    let file = directory.path().join("no-view.png");
    CanvasDocument::export_png(
        app.world_mut(),
        document,
        &file,
        CanvasPngOptions {
            region: Some(region),
            scale: 1.0,
        },
    )
    .unwrap();
    finish(&mut app, before + 1);
    assert_eq!(
        image::open(file).unwrap().to_rgba8().get_pixel(128, 128).0,
        [255; 4]
    );
    assert_eq!(counts(&mut app), base);
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_png_rerasterizes_transparent_document_and_preserves_independent_views() {
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        corner_radius: 0.0,
        size: Vec2::new(120.0, 80.0),
    };
    let transform = Transform::from_xyz(-100.0, -60.0, 0.0);
    let paint = CanvasPaint {
        fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
        stroke: Some(CanvasStroke {
            color: Color::srgba(0.0, 0.0, 1.0, 0.25),
            width: 10.0,
            ..Default::default()
        }),
        ..Default::default()
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
    let invisible = CanvasDocument::create_rectangle(
        app.world_mut(),
        document,
        CanvasRectangle {
            size: Vec2::splat(20.0),
            corner_radius: 0.0,
        },
        transform,
        CanvasPaint {
            fill: None,
            stroke: None,
            ..default()
        },
    )
    .unwrap();
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
    CanvasView::set_selection_in_world(app.world_mut(), a, [shape, invisible]).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), b, [shape]).unwrap();
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
    let mut selected = vec![shape, invisible];
    selected.sort_unstable();
    assert_eq!(CanvasView::selection(app.world(), a).unwrap(), selected);
    assert_eq!(CanvasView::selection(app.world(), b).unwrap(), [shape]);
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
            ..Default::default()
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
    CanvasDocument::remove_shape(app.world_mut(), document, shape).unwrap();
    CanvasDocument::remove_shape(app.world_mut(), document, invisible).unwrap();
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

    let ordered = app.world_mut().spawn(CanvasDocument).id();
    let back = CanvasDocument::create_rectangle(
        app.world_mut(),
        ordered,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::from_xyz(0.0, 0.0, 9000.0),
        CanvasPaint {
            fill: Some(Color::srgb(1.0, 0.0, 0.0)),
            stroke: Some(CanvasStroke {
                color: Color::srgb(0.0, 1.0, 0.0),
                width: 4.0,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let front = CanvasDocument::create_rectangle(
        app.world_mut(),
        ordered,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(30.0),
        },
        Transform::from_xyz(0.0, 0.0, -9000.0),
        CanvasPaint {
            fill: Some(Color::srgb(0.0, 0.0, 1.0)),
            stroke: None,
            ..Default::default()
        },
    )
    .unwrap();
    let completed = app.world().resource::<Completed>().0.len();
    for step in 0..2 {
        let path = directory.path().join(format!("order{step}.png"));
        let region = Rect::from_corners(Vec2::splat(-20.0), Vec2::splat(20.0));
        CanvasDocument::export_png(
            app.world_mut(),
            ordered,
            &path,
            CanvasPngOptions {
                region: Some(region),
                scale: 1.0,
            },
        )
        .unwrap();
        CanvasDocument::export_svg(
            app.world_mut(),
            ordered,
            path.with_extension("svg"),
            Some(region),
        )
        .unwrap();
        // Each request must keep its captured order even when the live document is reordered immediately.
        CanvasDocument::reorder_shape(
            app.world_mut(),
            ordered,
            front,
            if step == 0 {
                CanvasOrder::Back
            } else {
                CanvasOrder::Front
            },
        )
        .unwrap();
        finish(&mut app, completed + step + 1);
        let image = image::open(&path).unwrap().to_rgba8();
        let expected = if step == 0 {
            [0, 0, 255, 255]
        } else {
            [0, 255, 0, 255]
        };
        assert_eq!(image.get_pixel(29, 20).0, expected);
        let svg = std::fs::read(path.with_extension("svg")).unwrap();
        let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default()).unwrap();
        let mut pixels = resvg::tiny_skia::Pixmap::new(40, 40).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        let pixel = pixels.pixel(29, 20).unwrap();
        assert_eq!(
            [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()],
            expected
        );
    }
    assert_eq!(
        app.world().get::<Transform>(back).unwrap().translation.z,
        9000.0
    );
    assert_eq!(
        app.world().get::<Transform>(front).unwrap().translation.z,
        -9000.0
    );
}

fn gpu_app() -> App {
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
         manager: Res<bevy_widgetry_job::JobManager>,
         mut completed: ResMut<Completed>,
         mut commands: Commands| {
            commands.entity(event.entity).despawn();
            if let Ok(size) = event.result.as_ref() {
                assert_eq!(
                    manager.outcome(event.job_id).unwrap(),
                    Some(bevy_widgetry_job::JobOutcome::Completed)
                );
                assert_eq!(
                    image::image_dimensions(&event.path).unwrap(),
                    (size.x, size.y)
                );
            } else {
                assert!(matches!(
                    manager.outcome(event.job_id).unwrap(),
                    Some(bevy_widgetry_job::JobOutcome::Failed(_))
                ));
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
    app
}

#[derive(Resource, Default)]
struct Captured(Vec<Image>);

fn ellipse_view(app: &mut App, radii: Vec2, scale: f32, width: f32, fill: Option<Color>) -> Entity {
    ellipse_view_scaled(app, radii, Vec2::splat(scale), width, fill)
}

fn ellipse_view_scaled(
    app: &mut App,
    radii: Vec2,
    scale: Vec2,
    width: f32,
    fill: Option<Color>,
) -> Entity {
    let document = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_ellipse(
        app.world_mut(),
        document,
        CanvasEllipse { radii },
        Transform::from_scale(scale.extend(1.0)),
        CanvasPaint {
            fill,
            stroke: Some(CanvasStroke {
                color: Color::srgba(0.0, 0.0, 1.0, 0.5),
                width,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    view_camera(app, document)
}

fn view_camera(app: &mut App, document: Entity) -> Entity {
    use bevy::camera::RenderTarget;
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            256,
            256,
            bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb,
            None,
        ));
    let ui_camera = app
        .world_mut()
        .spawn((Camera2d, RenderTarget::Image(image.into())))
        .id();
    let view = app.world_mut().spawn_scene(bsn!{ @CanvasView { @document: {Some(document)} } template(move |_|Ok(UiTargetCamera(ui_camera))) Node {width:px(256),height:px(256)} }).unwrap().id();
    for _ in 0..20 {
        frame(app);
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    while app
        .sub_app(bevy::render::RenderApp)
        .world()
        .resource::<bevy::render::render_resource::PipelineCache>()
        .waiting_pipelines()
        .next()
        .is_some()
    {
        assert!(
            Instant::now() < deadline,
            "Canvas View pipeline compilation timed out"
        );
        frame(app);
        std::thread::sleep(Duration::from_millis(1));
    }
    frame(app);
    app.world()
        .get::<ViewportNode>(view)
        .unwrap()
        .camera
        .unwrap()
}

fn capture(app: &mut App, camera: Entity) -> image::RgbaImage {
    use bevy::{
        camera::RenderTarget,
        render::view::screenshot::{Screenshot, ScreenshotCaptured},
    };
    // A formerly empty scene may create its first mesh pipeline only after an update.
    frame(app);
    let pipeline_deadline = Instant::now() + Duration::from_secs(30);
    while app
        .sub_app(bevy::render::RenderApp)
        .world()
        .resource::<bevy::render::render_resource::PipelineCache>()
        .waiting_pipelines()
        .next()
        .is_some()
    {
        assert!(
            Instant::now() < pipeline_deadline,
            "Canvas capture pipeline compilation timed out"
        );
        frame(app);
        std::thread::sleep(Duration::from_millis(1));
    }
    app.init_resource::<Captured>();
    let target = app
        .world()
        .get::<RenderTarget>(camera)
        .unwrap()
        .as_image()
        .unwrap()
        .clone();
    app.world_mut().spawn(Screenshot::image(target)).observe(
        |event: On<ScreenshotCaptured>, mut images: ResMut<Captured>| {
            images.0.push(event.image.clone())
        },
    );
    let deadline = Instant::now() + Duration::from_secs(35);
    while app.world().resource::<Captured>().0.is_empty() {
        assert!(Instant::now() < deadline, "Canvas View readback timed out");
        frame(app);
        std::thread::sleep(Duration::from_millis(1));
    }
    app.world_mut()
        .resource_mut::<Captured>()
        .0
        .remove(0)
        .try_into_dynamic()
        .unwrap()
        .to_rgba8()
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_fill_rule_changes_holes_without_rewriting_commands() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let mut path = CanvasPath { commands: vec![] };
    for size in [80.0, 30.0] {
        path.commands.extend([
            MoveTo(Vec2::splat(-size)),
            LineTo(Vec2::new(size, -size)),
            LineTo(Vec2::splat(size)),
            LineTo(Vec2::new(-size, size)),
            Close,
        ]);
    }
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        path.clone(),
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    for rule in [
        CanvasFillRule::NonZero,
        CanvasFillRule::EvenOdd,
        CanvasFillRule::NonZero,
    ] {
        CanvasDocument::update_path(
            app.world_mut(),
            document,
            shape,
            path.clone(),
            Transform::default(),
            CanvasPaint {
                fill_rule: rule,
                ..default()
            },
        )
        .unwrap();
        for _ in 0..3 {
            frame(&mut app);
        }
        let image = capture(&mut app, camera);
        assert_eq!(
            image.get_pixel(128, 128).0[3],
            if rule == CanvasFillRule::EvenOdd {
                0
            } else {
                255
            },
            "{rule:?}"
        );
        assert_eq!(image.get_pixel(185, 128).0[3], 255);
        assert_eq!(
            app.world().get::<CanvasGeometry>(shape),
            Some(&CanvasGeometry::Path(path.clone()))
        );
    }

    let mut reversed = path.clone();
    reversed.commands[6..9].reverse();
    let star = CanvasPath {
        commands: (0..5)
            .flat_map(|i| {
                let angle =
                    std::f32::consts::TAU * (i * 2) as f32 / 5.0 - std::f32::consts::FRAC_PI_2;
                let p = Vec2::new(angle.cos(), angle.sin()) * 80.0;
                if i == 0 {
                    vec![MoveTo(p)]
                } else {
                    vec![LineTo(p)]
                }
            })
            .chain([Close])
            .collect(),
    };
    for (path, nonzero_center) in [(reversed, false), (star, true)] {
        for rule in [CanvasFillRule::NonZero, CanvasFillRule::EvenOdd] {
            CanvasDocument::update_path(
                app.world_mut(),
                document,
                shape,
                path.clone(),
                Transform::default(),
                CanvasPaint {
                    fill_rule: rule,
                    ..default()
                },
            )
            .unwrap();
            for _ in 0..3 {
                frame(&mut app);
            }
            let image = capture(&mut app, camera);
            assert_eq!(
                image.get_pixel(128, 128).0[3],
                if nonzero_center && rule == CanvasFillRule::NonZero {
                    255
                } else {
                    0
                },
                "{rule:?} {path:?}"
            );
        }
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_zero_length_and_closed_subpaths_apply_svg_caps() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let initial = CanvasPath {
        commands: vec![MoveTo(Vec2::ZERO)],
    };
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        initial,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    for tail in [
        vec![],
        vec![LineTo(Vec2::ZERO)],
        vec![Close],
        vec![LineTo(Vec2::ZERO), Close],
    ] {
        for cap in [
            CanvasLineCap::Butt,
            CanvasLineCap::Round,
            CanvasLineCap::Square,
        ] {
            let path = CanvasPath {
                commands: std::iter::once(MoveTo(Vec2::ZERO))
                    .chain(tail.iter().copied())
                    .collect(),
            };
            let paint = CanvasPaint {
                fill: None,
                stroke: Some(CanvasStroke {
                    width: 20.0,
                    cap,
                    ..default()
                }),
                ..default()
            };
            CanvasDocument::update_path(
                app.world_mut(),
                document,
                shape,
                path,
                Transform::default(),
                paint,
            )
            .unwrap();
            for _ in 0..3 {
                frame(&mut app);
            }
            let image = capture(&mut app, camera);
            for (point, inside) in [
                ((128, 128), cap != CanvasLineCap::Butt),
                ((136, 136), cap == CanvasLineCap::Square),
            ] {
                assert_eq!(
                    image.get_pixel(point.0, point.1).0[3],
                    if !tail.is_empty() && inside { 255 } else { 0 },
                    "{tail:?} {cap:?} {point:?}"
                );
            }
        }
    }
    let mut expected = None;
    for cap in [
        CanvasLineCap::Butt,
        CanvasLineCap::Round,
        CanvasLineCap::Square,
    ] {
        let path = CanvasPath {
            commands: vec![
                MoveTo(Vec2::splat(-30.0)),
                LineTo(Vec2::new(30.0, -30.0)),
                LineTo(Vec2::splat(30.0)),
                Close,
            ],
        };
        CanvasDocument::update_path(
            app.world_mut(),
            document,
            shape,
            path,
            Transform::default(),
            CanvasPaint {
                fill: None,
                stroke: Some(CanvasStroke {
                    width: 20.0,
                    cap,
                    join: CanvasLineJoin::Round,
                    ..default()
                }),
                ..default()
            },
        )
        .unwrap();
        for _ in 0..3 {
            frame(&mut app);
        }
        let image = capture(&mut app, camera);
        if let Some(expected) = &expected {
            assert_eq!(&image, expected, "closed cap {cap:?}");
        } else {
            expected = Some(image);
        }
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_rectangle_join_matches_svg_and_png_export() {
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        corner_radius: 0.0,
        size: Vec2::splat(60.0),
    };
    let shape = CanvasDocument::create_rectangle(
        app.world_mut(),
        document,
        rectangle,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let directory = tempfile::tempdir().unwrap();
    let region = Rect::from_corners(Vec2::splat(-128.0), Vec2::splat(128.0));
    for (i, join) in [
        CanvasLineJoin::Miter,
        CanvasLineJoin::Round,
        CanvasLineJoin::Bevel,
        CanvasLineJoin::Miter,
    ]
    .into_iter()
    .enumerate()
    {
        let rectangle = CanvasRectangle {
            corner_radius: if i == 3 { 25.0 } else { 0.0 },
            ..rectangle
        };
        let paint = CanvasPaint {
            fill: None,
            fill_rule: CanvasFillRule::EvenOdd,
            stroke: Some(CanvasStroke {
                width: 20.0,
                cap: CanvasLineCap::Square,
                join,
                ..default()
            }),
        };
        CanvasDocument::update_rectangle(
            app.world_mut(),
            document,
            shape,
            rectangle,
            Transform::default(),
            paint,
        )
        .unwrap();
        for _ in 0..3 {
            frame(&mut app);
        }
        let view = capture(&mut app, camera);
        let svg = directory.path().join("paint.svg");
        CanvasDocument::export_svg(app.world_mut(), document, &svg, Some(region)).unwrap();
        let tree = resvg::usvg::Tree::from_data(
            &std::fs::read(svg).unwrap(),
            &resvg::usvg::Options::default(),
        )
        .unwrap();
        let mut raster = resvg::tiny_skia::Pixmap::new(256, 256).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut raster.as_mut(),
        );
        let png = directory.path().join(format!("paint{i}.png"));
        CanvasDocument::export_png(
            app.world_mut(),
            document,
            &png,
            CanvasPngOptions {
                region: Some(region),
                ..default()
            },
        )
        .unwrap();
        finish(&mut app, i + 1);
        let exported = image::open(png).unwrap().to_rgba8();
        for point in [(166, 90), (164, 92), (158, 128), (128, 128)] {
            let expected = raster.pixel(point.0, point.1).unwrap().alpha();
            assert_eq!(
                view.get_pixel(point.0, point.1).0[3],
                expected,
                "View {join:?} {point:?}"
            );
            assert_eq!(
                exported.get_pixel(point.0, point.1).0[3],
                expected,
                "PNG {join:?} {point:?}"
            );
        }
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_caps_and_joins_change_coverage_without_double_blending() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let line = CanvasPath {
        commands: vec![MoveTo(Vec2::new(-30.0, 0.0)), LineTo(Vec2::new(30.0, 0.0))],
    };
    let mut paint = CanvasPaint {
        fill: None,
        fill_rule: CanvasFillRule::EvenOdd,
        stroke: Some(CanvasStroke {
            color: Color::srgba(0.0, 0.0, 1.0, 0.5),
            width: 20.0,
            ..default()
        }),
    };
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        line.clone(),
        Transform::default(),
        paint,
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let directory = tempfile::tempdir().unwrap();
    let export = |app: &mut App| {
        let before = app.world().resource::<Completed>().0.len();
        let file = directory.path().join("path-style.png");
        CanvasDocument::export_png(
            app.world_mut(),
            document,
            &file,
            CanvasPngOptions {
                region: Some(Rect::from_corners(Vec2::splat(-128.0), Vec2::splat(128.0))),
                scale: 1.0,
            },
        )
        .unwrap();
        finish(app, before + 1);
        image::open(file).unwrap().to_rgba8()
    };
    for cap in [
        CanvasLineCap::Butt,
        CanvasLineCap::Round,
        CanvasLineCap::Square,
    ] {
        paint.stroke.as_mut().unwrap().cap = cap;
        CanvasDocument::update_path(
            app.world_mut(),
            document,
            shape,
            line.clone(),
            Transform::default(),
            paint,
        )
        .unwrap();
        for _ in 0..3 {
            frame(&mut app);
        }
        let image = capture(&mut app, camera);
        let exported = export(&mut app);
        for (point, inside) in [
            ((166, 128), cap != CanvasLineCap::Butt),
            ((166, 136), cap == CanvasLineCap::Square),
            ((128, 128), true),
        ] {
            let alpha = image.get_pixel(point.0, point.1).0[3];
            assert_eq!(
                exported.get_pixel(point.0, point.1).0,
                image.get_pixel(point.0, point.1).0
            );
            assert!(
                alpha.abs_diff(if inside { 128 } else { 0 }) <= 3,
                "{cap:?} {point:?}: {alpha}"
            );
        }
    }
    let corner = CanvasPath {
        commands: vec![
            MoveTo(Vec2::new(-30.0, 0.0)),
            LineTo(Vec2::ZERO),
            LineTo(Vec2::new(0.0, 30.0)),
        ],
    };
    for join in [
        CanvasLineJoin::Miter,
        CanvasLineJoin::Round,
        CanvasLineJoin::Bevel,
    ] {
        paint.stroke.as_mut().unwrap().join = join;
        CanvasDocument::update_path(
            app.world_mut(),
            document,
            shape,
            corner.clone(),
            Transform::default(),
            paint,
        )
        .unwrap();
        for _ in 0..3 {
            frame(&mut app);
        }
        let image = capture(&mut app, camera);
        let exported = export(&mut app);
        for (point, inside) in [
            ((136, 120), join == CanvasLineJoin::Miter),
            ((134, 122), join != CanvasLineJoin::Bevel),
            ((124, 132), true),
        ] {
            let alpha = image.get_pixel(point.0, point.1).0[3];
            assert_eq!(
                exported.get_pixel(point.0, point.1).0,
                image.get_pixel(point.0, point.1).0
            );
            assert!(
                alpha.abs_diff(if inside { 128 } else { 0 }) <= 3,
                "{join:?} {point:?}: {alpha}"
            );
        }
    }
    paint.stroke.as_mut().unwrap().cap = CanvasLineCap::Butt;
    paint.stroke.as_mut().unwrap().join = CanvasLineJoin::Round;
    CanvasDocument::update_path(
        app.world_mut(),
        document,
        shape,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(-2.0, 0.0)),
                LineTo(Vec2::ZERO),
                LineTo(Vec2::new(-1.0, 0.0)),
            ],
        },
        Transform::default(),
        paint,
    )
    .unwrap();
    for _ in 0..3 {
        frame(&mut app);
    }
    let image = capture(&mut app, camera);
    assert_eq!(
        image.get_pixel(120, 128).0[3],
        0,
        "round cusp must not add the inward half-circle beyond both short segments"
    );
    assert!(image.get_pixel(136, 128).0[3].abs_diff(128) <= 3);
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_ellipse_uniform_scale_preserves_fill_and_stroke_at_tiny_and_huge_local_radii() {
    let mut app = gpu_app();
    let camera = ellipse_view(
        &mut app,
        Vec2::splat(100.0),
        1.0,
        20.0,
        Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
    );
    let expected = capture(&mut app, camera);
    assert!(expected.get_pixel(128, 128).0[3].abs_diff(128) <= 2);
    for radius in [1e-5_f32, 1e20] {
        let camera = ellipse_view(
            &mut app,
            Vec2::splat(radius),
            100.0 / radius,
            radius * 0.2,
            Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
        );
        let actual = capture(&mut app, camera);
        for point in [(128, 128), (233, 128), (128, 233)] {
            let a = actual.get_pixel(point.0, point.1).0;
            let e = expected.get_pixel(point.0, point.1).0;
            assert!(
                a.into_iter()
                    .zip(e)
                    .all(|(a, e)| (i16::from(a) - i16::from(e)).abs() <= 3),
                "radius {radius} at {point:?}: {a:?} vs {e:?}"
            );
        }
        let delta = actual
            .pixels()
            .zip(expected.pixels())
            .map(|(a, e)| {
                a.0.into_iter()
                    .zip(e.0)
                    .map(|(a, e)| u64::from(a.abs_diff(e)))
                    .sum::<u64>()
            })
            .sum::<u64>();
        let signal = expected
            .pixels()
            .map(|p| p.0.into_iter().map(u64::from).sum::<u64>())
            .sum::<u64>();
        assert!(
            delta * 100 < signal,
            "scaled equivalent ellipse differs by over one percent: radius={radius}, delta={delta}, signal={signal}"
        );
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_wide_transparent_ellipse_stroke_covers_each_pixel_once() {
    let mut app = gpu_app();
    for radius in [10.0, 1e-5] {
        let camera = ellipse_view(&mut app, Vec2::splat(radius), 1.0, 40.0, None);
        let image = capture(&mut app, camera);
        for point in [(128, 128), (127, 128), (128, 127), (140, 128)] {
            let alpha = image.get_pixel(point.0, point.1).0[3];
            assert!(
                alpha.abs_diff(128) <= 2,
                "stroke alpha at {point:?} is {alpha}, expected one coverage at 128"
            );
        }
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_ellipse_fill_survives_a_much_wider_translucent_stroke() {
    let mut app = gpu_app();
    let camera = ellipse_view(
        &mut app,
        Vec2::splat(100.0),
        1.0,
        1e6,
        Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
    );
    let image = capture(&mut app, camera);
    assert!(image.get_pixel(128, 128).0[3].abs_diff(191) <= 3);
    assert!(image.get_pixel(220, 160).0[3].abs_diff(191) <= 3);
    assert!(image.get_pixel(245, 128).0[3].abs_diff(128) <= 3);
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_thin_transparent_ellipse_stroke_keeps_the_hole_and_visible_edge() {
    let mut app = gpu_app();
    let camera = ellipse_view(&mut app, Vec2::splat(100.0), 1.0, 1.0, None);
    let image = capture(&mut app, camera);
    assert_eq!(image.get_pixel(128, 128).0[3], 0);
    let alpha = image.get_pixel(228, 128).0[3];
    assert!(
        alpha > 0 && alpha <= 130,
        "thin edge alpha should retain partial single coverage: {alpha}"
    );
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_eccentric_ellipse_wide_stroke_preserves_the_rounded_end() {
    let mut app = gpu_app();
    for radii in [Vec2::new(60.0, 0.6), Vec2::new(0.6, 60.0)] {
        let camera = ellipse_view(&mut app, radii, 1.0, 60.0, None);
        let image = capture(&mut app, camera);
        // These points lie well inside the radius-30 disk centered on the ellipse's tip.
        // A fixed parameter-angle polygon cuts through that disk for eccentric ellipses.
        let points = if radii.x > radii.y {
            [(208, 148), (48, 148), (208, 108), (48, 108)]
        } else {
            [(148, 208), (148, 48), (108, 208), (108, 48)]
        };
        for point in points {
            let alpha = image.get_pixel(point.0, point.1).0[3];
            assert!(
                alpha.abs_diff(128) <= 2,
                "eccentric {radii:?} stroke alpha at {point:?} is {alpha}, expected 128"
            );
        }
        assert_eq!(image.get_pixel(220, 128).0[3], 0);
        assert_eq!(image.get_pixel(128, 220).0[3], 0);
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_eccentric_ellipse_thin_stroke_survives_nonuniform_scale() {
    let mut app = gpu_app();
    for (radii, scale, sides) in [
        (
            Vec2::new(1.0, 1e8),
            Vec2::new(60.0, 6e-7),
            [(188, 128), (68, 128)],
        ),
        (
            Vec2::new(1e8, 1.0),
            Vec2::new(6e-7, 60.0),
            [(128, 188), (128, 68)],
        ),
    ] {
        let camera = ellipse_view_scaled(&mut app, radii, scale, 1.0, None);
        let image = capture(&mut app, camera);
        for point in sides {
            let alpha = image.get_pixel(point.0, point.1).0[3];
            assert!(
                alpha.abs_diff(128) <= 2,
                "nonuniform {radii:?} stroke alpha at {point:?} is {alpha}, expected 128"
            );
        }
        assert_eq!(image.get_pixel(128, 128).0[3], 0);
        assert_eq!(image.get_pixel(225, 128).0[3], 0);
        assert_eq!(image.get_pixel(128, 225).0[3], 0);
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_subpaths_curves_and_default_stroke_render_as_one_shape() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let path = CanvasPath {
        commands: vec![
            MoveTo(Vec2::new(-90.0, -60.0)),
            LineTo(Vec2::new(-10.0, -60.0)),
            LineTo(Vec2::new(-10.0, 20.0)),
            LineTo(Vec2::new(-90.0, 20.0)),
            Close,
            MoveTo(Vec2::new(20.0, 20.0)),
            QuadTo {
                control: Vec2::new(20.0, -60.0),
                to: Vec2::new(60.0, -60.0),
            },
            CubicTo {
                control1: Vec2::new(100.0, -60.0),
                control2: Vec2::new(100.0, 20.0),
                to: Vec2::new(60.0, 20.0),
            },
            Close,
            MoveTo(Vec2::new(-70.0, 60.0)),
            ArcTo {
                radii: Vec2::splat(20.0),
                rotation: 0.0,
                large_arc: false,
                sweep: true,
                to: Vec2::new(-30.0, 60.0),
            },
            MoveTo(Vec2::new(10.0, 60.0)),
            LineTo(Vec2::new(80.0, 60.0)),
        ],
    };
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        path,
        Transform::default(),
        CanvasPaint {
            fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
            stroke: Some(CanvasStroke {
                color: Color::srgba(0.0, 0.0, 1.0, 0.5),
                width: 10.0,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    for point in [(78, 108), (188, 108), (170, 188)] {
        assert!(
            image.get_pixel(point.0, point.1).0[3].abs_diff(128) <= 3,
            "interior/open stroke {point:?}"
        );
    }
    assert!(image.get_pixel(38, 68).0[3].abs_diff(191) <= 3);
    assert!(image.get_pixel(78, 168).0[3] > 100);
    for point in [(128, 128), (132, 188), (215, 188)] {
        assert_eq!(
            image.get_pixel(point.0, point.1).0[3],
            0,
            "unconnected subpaths/butt cap {point:?}"
        );
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_miter_join_extends_only_the_outer_side() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(-2.0, 0.0)),
                LineTo(Vec2::ZERO),
                LineTo(Vec2::new(-1.0, 3.0_f32.sqrt())),
            ],
        },
        Transform::default(),
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                color: Color::srgba(0.0, 0.0, 1.0, 0.5),
                width: 20.0,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    assert_eq!(
        image.get_pixel(113, 136).0[3],
        0,
        "inner miter must not extend past short segments"
    );
    assert_eq!(
        image.get_pixel(123, 128).0[3],
        0,
        "the inner bevel must not fill outside both short segments"
    );
    assert!(
        image.get_pixel(140, 120).0[3].abs_diff(128) <= 3,
        "outer miter remains visible exactly once"
    );
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_distant_subpath_preserves_small_contour_precision() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let mut expected = None;
    for distant in [false, true] {
        let mut commands = vec![
            MoveTo(Vec2::splat(-0.5)),
            LineTo(Vec2::new(0.5, -0.5)),
            LineTo(Vec2::splat(0.5)),
            LineTo(Vec2::new(-0.5, 0.5)),
            Close,
        ];
        if distant {
            commands.extend([
                MoveTo(Vec2::splat(1e8)),
                LineTo(Vec2::new(1e8 + 16.0, 1e8)),
                LineTo(Vec2::splat(1e8 + 16.0)),
                LineTo(Vec2::new(1e8, 1e8 + 16.0)),
                Close,
            ]);
        }
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath { commands },
            Transform::from_scale(Vec3::new(10.0, 10.0, 1.0)),
            CanvasPaint {
                fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
                stroke: Some(CanvasStroke {
                    color: Color::srgba(0.0, 0.0, 1.0, 0.5),
                    width: 0.1,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let image = capture(&mut app, camera);
        assert!(
            image.get_pixel(128, 128).0[3].abs_diff(128) <= 3,
            "distant={distant} must preserve local fill"
        );
        if let Some(expected) = &expected {
            assert_eq!(
                &image, expected,
                "a disconnected remote contour must not alter local pixels"
            );
        } else {
            expected = Some(image);
        }
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_overlapping_subpaths_keep_nonzero_winding_and_single_stroke_coverage() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    for same_winding in [false, true] {
        let mut commands = vec![
            MoveTo(Vec2::splat(-50.0)),
            LineTo(Vec2::new(50.0, -50.0)),
            LineTo(Vec2::splat(50.0)),
            LineTo(Vec2::new(-50.0, 50.0)),
            Close,
        ];
        let mut inner = [
            Vec2::splat(-20.0),
            Vec2::new(20.0, -20.0),
            Vec2::splat(20.0),
            Vec2::new(-20.0, 20.0),
        ];
        if !same_winding {
            inner.reverse();
        }
        commands.push(MoveTo(inner[0]));
        commands.extend(inner[1..].iter().copied().map(LineTo));
        commands.push(Close);
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath { commands },
            Transform::default(),
            CanvasPaint {
                fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
                stroke: None,
                ..Default::default()
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let image = capture(&mut app, camera);
        let alpha = image.get_pixel(128, 128).0[3];
        assert!(alpha.abs_diff(if same_winding { 128 } else { 0 }) <= 3);
        assert!(image.get_pixel(165, 128).0[3].abs_diff(128) <= 3);
    }
    let document = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(-30.0, 0.0)),
                LineTo(Vec2::new(30.0, 0.0)),
                MoveTo(Vec2::new(0.0, -30.0)),
                LineTo(Vec2::new(0.0, 30.0)),
            ],
        },
        Transform::default(),
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                color: Color::srgba(0.0, 0.0, 1.0, 0.5),
                width: 10.0,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    assert!(
        image.get_pixel(128, 128).0[3].abs_diff(128) <= 3,
        "crossing strokes in one Path are painted once"
    );
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_crossing_remote_subpath_does_not_erase_small_fill() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    for distance in [1e8, 1e12] {
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    MoveTo(Vec2::splat(-0.5)),
                    LineTo(Vec2::new(0.5, -0.5)),
                    LineTo(Vec2::splat(0.5)),
                    LineTo(Vec2::new(-0.5, 0.5)),
                    Close,
                    MoveTo(Vec2::ZERO),
                    LineTo(Vec2::splat(distance)),
                ],
            },
            Transform::from_scale(Vec3::new(10.0, 10.0, 1.0)),
            CanvasPaint {
                fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
                stroke: Some(CanvasStroke {
                    color: Color::srgba(0.0, 0.0, 1.0, 0.5),
                    width: 0.1,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let image = capture(&mut app, camera);
        assert!(
            image.get_pixel(130, 125).0[3].abs_diff(128) <= 3,
            "remote {distance} line must not erase fill: {:?}",
            image.get_pixel(130, 125).0
        );
        assert!(
            image.get_pixel(148, 148).0[3] > 100 && image.get_pixel(148, 148).0[3] <= 130,
            "remote line must preserve its local stroke"
        );
    }
    let document = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::splat(-1e8)),
                LineTo(Vec2::new(1e8, -1e8)),
                LineTo(Vec2::splat(1e8)),
                LineTo(Vec2::new(-1e8, 1e8)),
                Close,
                MoveTo(Vec2::splat(-0.5)),
                LineTo(Vec2::new(-0.5, 0.5)),
                LineTo(Vec2::splat(0.5)),
                LineTo(Vec2::new(0.5, -0.5)),
                Close,
            ],
        },
        Transform::from_scale(Vec3::new(10.0, 10.0, 1.0)),
        CanvasPaint {
            fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
            stroke: None,
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    assert_eq!(
        image.get_pixel(128, 128).0[3],
        0,
        "small reverse contour must preserve its hole inside a remote outline"
    );
    assert!(image.get_pixel(138, 128).0[3].abs_diff(128) <= 3);
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_remote_retraced_segment_preserves_local_fill() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    for distance in [1e8, 1e12] {
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    MoveTo(Vec2::splat(-0.5)),
                    LineTo(Vec2::new(0.5, -0.5)),
                    LineTo(Vec2::splat(0.5)),
                    LineTo(Vec2::new(-0.5, 0.5)),
                    LineTo(Vec2::splat(-0.5)),
                    LineTo(Vec2::splat(distance)),
                    LineTo(Vec2::splat(-0.5)),
                ],
            },
            Transform::from_scale(Vec3::new(10.0, 10.0, 1.0)),
            CanvasPaint {
                fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
                stroke: None,
                ..Default::default()
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let image = capture(&mut app, camera);
        assert!(
            image.get_pixel(130, 125).0[3].abs_diff(128) <= 3,
            "retraced remote {distance} segment must not erase the local fill: {:?}",
            image.get_pixel(130, 125).0
        );
        assert_eq!(image.get_pixel(138, 128).0[3], 0);
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_nearby_nonadjacent_vertices_preserve_self_intersection_fill() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    for distance in [1e8, 1e12] {
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    MoveTo(Vec2::ZERO),
                    LineTo(Vec2::splat(distance)),
                    LineTo(Vec2::Y),
                    LineTo(Vec2::new(distance, 0.0)),
                    Close,
                ],
            },
            Transform::from_scale(Vec3::new(10.0, 10.0, 1.0)),
            CanvasPaint {
                fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
                stroke: None,
                ..Default::default()
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let image = capture(&mut app, camera);
        assert!(
            image.get_pixel(138, 143).0[3].abs_diff(128) <= 3,
            "remote {distance} crossing must keep its local lobe: {:?}",
            image.get_pixel(138, 143).0
        );
        assert_eq!(image.get_pixel(118, 143).0[3], 0);
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_narrow_axis_aligned_notch_survives_remote_vertices() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    for distance in [1e8, 1e12] {
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    MoveTo(Vec2::new(0.0, -distance)),
                    LineTo(Vec2::new(1.0, -distance * 0.5)),
                    LineTo(Vec2::new(1.0, distance * 0.5)),
                    LineTo(Vec2::new(0.0, distance)),
                    LineTo(Vec2::new(distance, 0.0)),
                    Close,
                ],
            },
            Transform::from_scale(Vec3::new(10.0, 10.0, 1.0)),
            CanvasPaint {
                fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
                stroke: None,
                ..Default::default()
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let image = capture(&mut app, camera);
        assert_eq!(
            image.get_pixel(133, 128).0[3],
            0,
            "remote {distance} contour must preserve the narrow notch"
        );
        assert!(image.get_pixel(143, 128).0[3].abs_diff(128) <= 3);
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_overlapping_slender_contours_keep_bounded_geometry() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(0.0, -1e12)),
                LineTo(Vec2::new(1.0, 0.0)),
                LineTo(Vec2::new(0.0, 1e12)),
                Close,
                MoveTo(Vec2::new(0.0, -5e11)),
                LineTo(Vec2::new(1.0, 5e11)),
                LineTo(Vec2::new(0.0, 1.5e12)),
                Close,
            ],
        },
        Transform::from_scale(Vec3::new(100.0, 1e-10, 1.0)),
        CanvasPaint {
            fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
            stroke: None,
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    assert!(image.get_pixel(153, 153).0[3].abs_diff(128) <= 3);
    assert_eq!(image.get_pixel(118, 128).0[3], 0);
    let meshes = app.world().resource::<Assets<Mesh>>();
    assert!(meshes.iter().all(|(_, mesh)| mesh.count_vertices() < 1000));
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_wide_strokes_keep_a_local_gap_beside_remote_geometry() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(-50_000_004.0, -1e9)),
                LineTo(Vec2::new(-50_000_004.0, 1e9)),
                MoveTo(Vec2::new(50_000_004.0, -1e9)),
                LineTo(Vec2::new(50_000_004.0, 1e9)),
                MoveTo(Vec2::new(-50_000_004.0, 1e9)),
                LineTo(Vec2::new(1e12, 1e9)),
            ],
        },
        Transform::IDENTITY,
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                width: 1e8,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    assert_eq!(image.get_pixel(128, 128).0[3], 0);
    assert!(image.get_pixel(138, 128).0[3].abs_diff(128) <= 3);
    assert!(image.get_pixel(118, 128).0[3].abs_diff(128) <= 3);
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_staggered_diagonal_strokes_preserve_the_gap_after_one_axis_rebases() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let base = 2.0_f32.powi(40);
    let width = 2.0_f32.powi(27);
    let step = 2.0_f32.powi(20);
    let height = 2.0_f32.powi(30);
    let offset = (width + step) * 0.5;
    let x_scale = 10.0 / step;
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(base - offset - step, -height)),
                LineTo(Vec2::new(base - offset + step * 2.0, height * 2.0)),
                MoveTo(Vec2::new(base + offset, 0.0)),
                LineTo(Vec2::new(base + offset + step, height)),
                MoveTo(Vec2::new(base - offset, 0.0)),
                LineTo(Vec2::new(base - offset, 1.0)),
            ],
        },
        Transform::from_xyz(-base * x_scale, 0.0, 0.0).with_scale(Vec3::new(
            x_scale,
            10.0 / height,
            1.0,
        )),
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                width,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    for i in 1..9 {
        assert_eq!(
            image.get_pixel(128 + i, 128 + i).0[3],
            0,
            "the diagonal gap must stay transparent at {i}"
        );
    }
    assert!(image.get_pixel(118, 130).0[3].abs_diff(128) <= 3);
    assert!(image.get_pixel(148, 130).0[3].abs_diff(128) <= 3);
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_diagonal_gap_survives_without_coordinate_collisions() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let base = 2.0_f32.powi(28);
    let width = 2.0_f32.powi(27);
    let step = 2.0_f32.powi(20);
    let height = 2.0_f32.powi(30);
    let offset = (width + 256.0) * 0.5;
    for duplicate in [false, true] {
        let document = app.world_mut().spawn(CanvasDocument).id();
        let mut path = CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(base - offset - step, -height)),
                LineTo(Vec2::new(base - offset + step * 2.0, height * 2.0)),
                MoveTo(Vec2::new(base + offset, 0.0)),
                LineTo(Vec2::new(base + offset + step, height)),
            ],
        };
        if duplicate {
            path.commands.extend_from_within(..);
        }
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            path,
            Transform::from_xyz(-base / 16.0, 0.0, 0.0).with_scale(Vec3::new(
                1.0 / 16.0,
                1.0 / 16384.0,
                1.0,
            )),
            CanvasPaint {
                fill: None,
                stroke: Some(CanvasStroke {
                    color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                    width,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let image = capture(&mut app, camera);
        for i in 1..9 {
            assert_eq!(image.get_pixel(128 + i, 128 + i).0[3], 0);
        }
        assert!(image.get_pixel(110, 130).0[3].abs_diff(128) <= 3);
        assert!(image.get_pixel(148, 136).0[3].abs_diff(128) <= 3);
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_shared_stroke_corner_keeps_the_other_endpoint_gap() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let length = 2.0_f32.powi(-20);
    for (dx, dy, radius) in [(3.0, 4.0, 5.0), (5.0, 12.0, 13.0)] {
        let document = app.world_mut().spawn(CanvasDocument).id();
        CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![
                    MoveTo(Vec2::new(-radius, -2.0 * radius)),
                    LineTo(Vec2::new(-radius, 0.0)),
                    MoveTo(Vec2::new(dy + dx * length, dx - dy * length)),
                    LineTo(Vec2::new(dy, dx)),
                ],
            },
            Transform::from_scale(Vec3::new(40.0 / (dy * length), 40.0 / (dy * length), 1.0)),
            CanvasPaint {
                fill: None,
                stroke: Some(CanvasStroke {
                    color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                    width: 2.0 * radius,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
        let camera = view_camera(&mut app, document);
        let image = capture(&mut app, camera);
        assert_eq!(image.get_pixel(138, 98).0[3], 0);
        assert_eq!(image.get_pixel(132, 108).0[3], 0);
        assert!(image.get_pixel(118, 108).0[3].abs_diff(128) <= 3);
        assert!(image.get_pixel(158, 108).0[3].abs_diff(128) <= 3);
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_shared_corner_between_slanted_strokes_preserves_gap() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let length = 2.0_f32.powi(-20);
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(12.0 - 5.0 * length, -5.0 - 12.0 * length)),
                LineTo(Vec2::new(12.0, -5.0)),
                MoveTo(Vec2::new(-29.0, 2.0)),
                LineTo(Vec2::new(-5.0, 12.0)),
            ],
        },
        Transform::from_scale(Vec3::new(2.0 / length, 2.0 / length, 1.0)),
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                width: 26.0,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    assert_eq!(image.get_pixel(98, 108).0[3], 0);
    assert_eq!(image.get_pixel(113, 118).0[3], 0);
    assert!(
        image.get_pixel(128, 108).0[3].abs_diff(128) <= 3,
        "first side {:?}, further {:?}, inner {:?}",
        image.get_pixel(128, 108).0,
        image.get_pixel(138, 108).0,
        image.get_pixel(138, 118).0
    );
    assert!(image.get_pixel(68, 108).0[3].abs_diff(128) <= 3);
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_single_very_wide_diagonal_stroke_preserves_butt_faces() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    for width in [2.0_f32.powi(34), 2.0_f32.powi(80), 2.0_f32.powi(100)] {
        for crossing in [false, true] {
            let document = app.world_mut().spawn(CanvasDocument).id();
            let mut commands = vec![MoveTo(Vec2::splat(-50.0)), LineTo(Vec2::splat(50.0))];
            if crossing {
                commands.extend([
                    MoveTo(Vec2::new(-50.0, 50.0)),
                    LineTo(Vec2::new(50.0, -50.0)),
                ]);
            }
            CanvasDocument::create_path(
                app.world_mut(),
                document,
                CanvasPath { commands },
                Transform::IDENTITY,
                CanvasPaint {
                    fill: None,
                    stroke: Some(CanvasStroke {
                        color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                        width,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
            let camera = view_camera(&mut app, document);
            let image = capture(&mut app, camera);
            assert!(
                image.get_pixel(128, 128).0[3].abs_diff(128) <= 3,
                "width={width}, crossing={crossing}"
            );
            assert!(image.get_pixel(65, 190).0[3].abs_diff(128) <= 3);
            assert!(image.get_pixel(170, 170).0[3].abs_diff(128) <= 3);
            assert!(image.get_pixel(86, 86).0[3].abs_diff(128) <= 3);
            assert_eq!(image.get_pixel(250, 128).0[3], 0);
            assert_eq!(image.get_pixel(5, 128).0[3], 0);
            if crossing {
                assert!(image.get_pixel(190, 190).0[3].abs_diff(128) <= 3);
            } else {
                assert_eq!(image.get_pixel(190, 190).0[3], 0);
            }
        }
    }
}

#[test]
#[ignore = "requires an actual Windows DX12 adapter; run with --ignored"]
fn gpu_path_fill_curves_are_independent_of_a_huge_stroke_width() {
    use CanvasPathCommand::*;
    let mut app = gpu_app();
    let document = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_path(
        app.world_mut(),
        document,
        CanvasPath {
            commands: vec![
                MoveTo(Vec2::new(-50.0, -50.0)),
                QuadTo {
                    control: Vec2::new(50.0, -50.0),
                    to: Vec2::new(50.0, 50.0),
                },
                LineTo(Vec2::new(-50.0, 50.0)),
                Close,
            ],
        },
        Transform::default(),
        CanvasPaint {
            fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
            stroke: Some(CanvasStroke {
                color: Color::srgba(0.0, 0.0, 1.0, 0.5),
                width: 1e10,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let camera = view_camera(&mut app, document);
    let image = capture(&mut app, camera);
    assert!(
        image.get_pixel(128, 108).0[3].abs_diff(191) <= 3,
        "wide stroke must not flatten the fill curve"
    );
}
