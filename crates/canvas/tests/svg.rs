#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use resvg::usvg::{self, roxmltree};
mod common;

fn values(text: &str) -> Vec<f32> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap())
        .collect()
}

#[test]
fn svg_large_arc_rotation_keeps_the_same_axis_as_document_geometry() {
    use CanvasPathCommand::*;
    for rotation in [1e20_f32, -1e30, f32::MAX] {
        let mut world = World::new();
        let document = world.spawn(CanvasDocument).id();
        CanvasDocument::create_path(
            &mut world,
            document,
            CanvasPath {
                commands: vec![
                    MoveTo(Vec2::ZERO),
                    ArcTo {
                        radii: Vec2::new(40.0, 15.0),
                        rotation,
                        large_arc: true,
                        sweep: false,
                        to: Vec2::new(30.0, 10.0),
                    },
                ],
            },
            Transform::default(),
            CanvasPaint::default(),
        )
        .unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        CanvasDocument::export_svg(&mut world, document, file.path(), None).unwrap();
        let xml = std::fs::read_to_string(file.path()).unwrap();
        let dom = roxmltree::Document::parse(&xml).unwrap();
        let path = dom.descendants().find(|n| n.has_tag_name("path")).unwrap();
        let arc = path.attribute("d").unwrap().split("A ").nth(1).unwrap();
        let degrees: f64 = arc.split_whitespace().nth(2).unwrap().parse().unwrap();
        let (sin, cos) = f64::from(rotation).sin_cos();
        let (exported_sin, exported_cos) = degrees.to_radians().sin_cos();
        assert!(
            (exported_sin - sin).abs() < 1e-12 && (exported_cos - cos).abs() < 1e-12,
            "{rotation}: {degrees}"
        );
        usvg::Tree::from_str(&xml, &usvg::Options::default()).unwrap();
    }
}

#[test]
fn svg_preserves_original_commands_arc_flags_and_all_paint_styles() {
    use CanvasPathCommand::*;
    for (cap, cap_name) in [
        (CanvasLineCap::Butt, "butt"),
        (CanvasLineCap::Round, "round"),
        (CanvasLineCap::Square, "square"),
    ] {
        for (join, join_name) in [
            (CanvasLineJoin::Miter, "miter"),
            (CanvasLineJoin::Round, "round"),
            (CanvasLineJoin::Bevel, "bevel"),
        ] {
            let mut world = World::new();
            let document = world.spawn(CanvasDocument).id();
            let mut commands = vec![
                MoveTo(Vec2::ZERO),
                LineTo(Vec2::new(10.0, 0.0)),
                QuadTo {
                    control: Vec2::new(15.0, -8.0),
                    to: Vec2::new(20.0, 0.0),
                },
                CubicTo {
                    control1: Vec2::new(25.0, 8.0),
                    control2: Vec2::new(30.0, -8.0),
                    to: Vec2::new(35.0, 0.0),
                },
                Close,
                MoveTo(Vec2::new(50.0, 0.0)),
            ];
            for (index, (large_arc, sweep)) in
                [(false, false), (false, true), (true, false), (true, true)]
                    .into_iter()
                    .enumerate()
            {
                commands.push(ArcTo {
                    radii: Vec2::new(20.0, 10.0),
                    rotation: std::f32::consts::FRAC_PI_2,
                    large_arc,
                    sweep,
                    to: Vec2::new(60.0 + index as f32 * 10.0, 5.0),
                });
            }
            let path = CanvasPath { commands };
            let shape = CanvasDocument::create_path(
                &mut world,
                document,
                path.clone(),
                Transform::from_xyz(5.0, -3.0, 9000.0)
                    .with_rotation(Quat::from_rotation_z(0.3))
                    .with_scale(Vec3::new(-1.5, 0.75, 0.0)),
                CanvasPaint {
                    fill: Some(Color::srgba(0.2, 0.4, 0.6, 0.25)),
                    fill_rule: CanvasFillRule::EvenOdd,
                    stroke: Some(CanvasStroke {
                        color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                        width: 3.0,
                        cap,
                        join,
                    }),
                },
            )
            .unwrap();
            let file = tempfile::NamedTempFile::new().unwrap();
            CanvasDocument::export_svg(&mut world, document, file.path(), None).unwrap();
            let xml = std::fs::read_to_string(file.path()).unwrap();
            let dom = roxmltree::Document::parse(&xml).unwrap();
            let paths = dom
                .descendants()
                .filter(|n| n.has_tag_name("path"))
                .collect::<Vec<_>>();
            assert_eq!(paths.len(), 2);
            assert_eq!(paths[0].attribute("fill-rule"), Some("evenodd"));
            assert_eq!(paths[0].attribute("fill-opacity"), Some("0.25"));
            assert_eq!(paths[1].attribute("stroke-opacity"), Some("0.5"));
            assert_eq!(paths[1].attribute("stroke-linecap"), Some(cap_name));
            assert_eq!(paths[1].attribute("stroke-linejoin"), Some(join_name));
            assert_eq!(paths[1].attribute("stroke-miterlimit"), Some("4"));
            let d = paths[0].attribute("d").unwrap();
            assert_eq!(paths[1].attribute("d"), Some(d));
            assert!(
                d.starts_with("M 0 0 L 10 0 Q 15 -8 20 0 C 25 8 30 -8 35 0 Z M 50 0 "),
                "{d}"
            );
            for (arc, flags) in
                d.split("A ")
                    .skip(1)
                    .zip([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]])
            {
                let args = values(arc);
                assert_eq!(&args[..2], &[20.0, 10.0]);
                assert!((args[2] - 90.0).abs() < 0.00001);
                assert_eq!(&args[3..5], &flags);
            }
            usvg::Tree::from_str(&xml, &usvg::Options::default()).unwrap();
            assert_eq!(
                world.get::<CanvasGeometry>(shape),
                Some(&CanvasGeometry::Path(path))
            );
        }
    }
}

#[test]
fn svg_mixed_shapes_preserve_holes_and_reordering_overrides_z() {
    use CanvasPathCommand::*;
    for rule in [CanvasFillRule::NonZero, CanvasFillRule::EvenOdd] {
        for reverse in [false, true] {
            let mut world = World::new();
            let document = world.spawn(CanvasDocument).id();
            CanvasDocument::create_rectangle(
                &mut world,
                document,
                CanvasRectangle {
                    size: Vec2::splat(100.0),
                    corner_radius: 6.0,
                },
                Transform::from_xyz(0.0, 0.0, 500.0),
                CanvasPaint {
                    fill: Some(Color::srgb(1.0, 0.0, 0.0)),
                    ..default()
                },
            )
            .unwrap();
            let ellipse = CanvasDocument::create_ellipse(
                &mut world,
                document,
                CanvasEllipse {
                    radii: Vec2::splat(35.0),
                },
                Transform::from_xyz(0.0, 0.0, -500.0),
                CanvasPaint {
                    fill: Some(Color::srgb(1.0, 0.5, 0.0)),
                    ..default()
                },
            )
            .unwrap();
            let mut commands = vec![
                MoveTo(Vec2::splat(-25.0)),
                LineTo(Vec2::new(25.0, -25.0)),
                LineTo(Vec2::splat(25.0)),
                LineTo(Vec2::new(-25.0, 25.0)),
                Close,
            ];
            let mut inner = [
                Vec2::splat(-10.0),
                Vec2::new(10.0, -10.0),
                Vec2::splat(10.0),
                Vec2::new(-10.0, 10.0),
            ];
            if reverse {
                inner.reverse();
            }
            commands.push(MoveTo(inner[0]));
            commands.extend(inner[1..].iter().copied().map(LineTo));
            commands.push(Close);
            CanvasDocument::create_path(
                &mut world,
                document,
                CanvasPath { commands },
                Transform::default(),
                CanvasPaint {
                    fill: Some(Color::srgb(0.0, 0.0, 1.0)),
                    fill_rule: rule,
                    stroke: Some(CanvasStroke {
                        color: Color::BLACK,
                        width: 2.0,
                        ..default()
                    }),
                },
            )
            .unwrap();
            let directory = tempfile::tempdir().unwrap();
            let file = directory.path().join("矢量-孔洞-😀.svg");
            CanvasDocument::export_svg(&mut world, document, &file, None).unwrap();
            let original = std::fs::read(&file).unwrap();
            let render = |xml: &[u8]| {
                let tree = usvg::Tree::from_data(xml, &usvg::Options::default()).unwrap();
                let mut image = resvg::tiny_skia::Pixmap::new(100, 100).unwrap();
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::identity(),
                    &mut image.as_mut(),
                );
                image
            };
            let image = render(&original);
            let center = image.pixel(50, 50).unwrap();
            let hole = reverse || rule == CanvasFillRule::EvenOdd;
            assert_eq!(center.red(), if hole { 255 } else { 0 });
            assert_eq!(center.blue(), if hole { 0 } else { 255 });
            assert_eq!(image.pixel(70, 50).unwrap().blue(), 255);
            assert_eq!(image.pixel(82, 50).unwrap().red(), 255);
            CanvasDocument::reorder_shape(&mut world, document, ellipse, CanvasOrder::Front)
                .unwrap();
            assert_eq!(std::fs::read(&file).unwrap(), original);
            CanvasDocument::export_svg(&mut world, document, &file, None).unwrap();
            let reordered = render(&std::fs::read(&file).unwrap());
            assert_eq!(reordered.pixel(70, 50).unwrap().red(), 255);
        }
    }
}

#[test]
fn svg_rejects_missing_components_foreign_members_and_invalid_paths_without_repair() {
    use bevy_widgetry_test_utils::LogCapture;
    for invalid in 0..5 {
        let mut world = World::new();
        let document = world.spawn(CanvasDocument).id();
        let shape = CanvasDocument::create_path(
            &mut world,
            document,
            CanvasPath {
                commands: vec![
                    CanvasPathCommand::MoveTo(Vec2::ZERO),
                    CanvasPathCommand::LineTo(Vec2::ONE),
                ],
            },
            Transform::default(),
            CanvasPaint::default(),
        )
        .unwrap();
        match invalid {
            0 => {
                world.entity_mut(shape).remove::<CanvasPaint>();
            }
            1 => {
                world.entity_mut(shape).remove::<CanvasShape>();
            }
            2 => {
                let other = world.spawn(CanvasDocument).id();
                world.entity_mut(shape).insert(ChildOf(other));
            }
            3 => {
                world
                    .entity_mut(shape)
                    .insert(CanvasGeometry::Path(CanvasPath {
                        commands: vec![CanvasPathCommand::Close],
                    }));
            }
            _ => {
                world.entity_mut(shape).remove::<Transform>();
            }
        }
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"preserve").unwrap();
        let logs = LogCapture::default();
        assert!(matches!(
            logs.run(|| CanvasDocument::export_svg(&mut world, document, file.path(), None)),
            Err(CanvasExportError::InvalidGeometry(_))
        ));
        assert_eq!(std::fs::read(file.path()).unwrap(), b"preserve");
    }
}

#[test]
fn svg_uses_document_bounds_centered_stroke_and_full_transform_without_a_view() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let size = Vec2::new(120.0, 80.0);
    let paint = CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            color: Color::srgba(0.1, 0.3, 0.8, 0.5),
            width: 10.0,
            ..Default::default()
        }),
        ..Default::default()
    };
    let file = tempfile::NamedTempFile::new().unwrap();
    for transform in [
        Transform::from_xyz(-100.0, -50.0, 0.0),
        Transform::from_xyz(-100.0, -50.0, 0.0)
            .with_rotation(Quat::from_rotation_z(0.7))
            .with_scale(Vec3::new(-1.4, 0.8, 0.0)),
    ] {
        let shape = CanvasDocument::create_rectangle(
            &mut world,
            document,
            CanvasRectangle {
                corner_radius: 0.0,
                size,
            },
            transform,
            paint,
        )
        .unwrap();
        CanvasDocument::export_svg(&mut world, document, file.path(), None).unwrap();
        let xml = std::fs::read_to_string(file.path()).unwrap();
        let dom = roxmltree::Document::parse(&xml).unwrap();
        let root = dom.root_element();
        let bounds = CanvasRectangle {
            corner_radius: 0.0,
            size,
        }
        .bounds(&transform, &paint)
        .unwrap();
        let view_box = values(root.attribute("viewBox").unwrap());
        for (a, b) in
            view_box
                .iter()
                .zip([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()])
        {
            assert!((*a - b).abs() < 0.0001);
        }
        let rect = dom
            .descendants()
            .find(|n| n.has_tag_name("rect") && n.attribute("transform").is_some())
            .unwrap();
        assert_eq!(rect.attribute("fill"), Some("none"));
        assert_eq!(rect.attribute("stroke-width"), Some("10"));
        assert!(
            (rect
                .attribute("stroke-opacity")
                .unwrap()
                .parse::<f32>()
                .unwrap()
                - 0.5)
                .abs()
                < 0.0001
        );
        let matrix = values(
            rect.attribute("transform")
                .unwrap()
                .trim_start_matches("matrix(")
                .trim_end_matches(')'),
        );
        for local in [Vec2::new(-60.0, -40.0), Vec2::new(60.0, 40.0)] {
            let actual = Vec2::new(
                matrix[0] * local.x + matrix[2] * local.y + matrix[4],
                matrix[1] * local.x + matrix[3] * local.y + matrix[5],
            );
            assert!(
                actual.distance(transform.transform_point(local.extend(0.0)).truncate()) < 0.0001
            );
        }
        let tree = usvg::Tree::from_str(&xml, &usvg::Options::default()).unwrap();
        assert!((tree.size().width() - bounds.width()).abs() < 0.001);
        CanvasDocument::remove_shape(&mut world, document, shape).unwrap();
    }
}

#[test]
fn svg_contains_all_shapes_in_depth_order_and_preserves_alpha_and_clipping() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    for (z, color) in [
        (2.0, Color::srgba(0.0, 1.0, 0.0, 0.5)),
        (0.0, Color::srgb(1.0, 0.0, 0.0)),
    ] {
        CanvasDocument::create_rectangle(
            &mut world,
            document,
            CanvasRectangle {
                corner_radius: 0.0,
                size: Vec2::splat(20.0),
            },
            Transform::from_xyz(-20.0, -10.0, z),
            CanvasPaint {
                fill: Some(color),
                stroke: None,
                ..Default::default()
            },
        )
        .unwrap();
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    let region = Rect::from_corners(Vec2::new(-50.0, -40.0), Vec2::new(10.0, 20.0));
    CanvasDocument::export_svg(&mut world, document, file.path(), Some(region)).unwrap();
    let xml = std::fs::read_to_string(file.path()).unwrap();
    let dom = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(
        dom.descendants()
            .filter(|n| n.has_tag_name("rect") && n.attribute("transform").is_some())
            .count(),
        2
    );
    let tree = usvg::Tree::from_str(&xml, &usvg::Options::default()).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(60, 60).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let center = pixmap.pixel(30, 30).unwrap();
    assert_eq!(center.alpha(), 255);
    assert_eq!(pixmap.pixel(0, 0).unwrap().alpha(), 0);
    CanvasDocument::export_svg(
        &mut world,
        document,
        file.path(),
        Some(Rect::from_corners(Vec2::splat(500.0), Vec2::splat(510.0))),
    )
    .unwrap();
    let tree = usvg::Tree::from_data(
        &std::fs::read(file.path()).unwrap(),
        &usvg::Options::default(),
    )
    .unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(10, 10).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    assert!(pixmap.data().iter().all(|v| *v == 0));
}

#[test]
fn rejected_exports_preserve_existing_files_and_valid_export_overwrites() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"keep me").unwrap();
    let missing = world.spawn_empty().id();
    assert!(matches!(
        CanvasDocument::export_svg(&mut world, missing, file.path(), None),
        Err(CanvasExportError::InvalidDocument)
    ));
    for region in [
        None,
        Some(Rect::from_corners(Vec2::ZERO, Vec2::splat(10.0))),
    ] {
        assert!(matches!(
            CanvasDocument::export_svg(&mut world, document, file.path(), region),
            Err(CanvasExportError::EmptyDocument)
        ));
        assert_eq!(std::fs::read(file.path()).unwrap(), b"keep me");
    }
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    for region in [
        Rect {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        },
        Rect {
            min: Vec2::ONE,
            max: Vec2::ZERO,
        },
        Rect {
            min: Vec2::ZERO,
            max: Vec2::splat(f32::INFINITY),
        },
    ] {
        assert!(matches!(
            CanvasDocument::export_svg(&mut world, document, file.path(), Some(region)),
            Err(CanvasExportError::InvalidRegion)
        ));
        assert_eq!(std::fs::read(file.path()).unwrap(), b"keep me");
    }
    world
        .get_mut::<CanvasGeometry>(shape)
        .map(|mut geometry| {
            let mut rectangle = *geometry.rectangle().unwrap();
            rectangle.size.x = f32::NAN;
            *geometry = CanvasGeometry::Rectangle(rectangle);
        })
        .unwrap();
    assert!(matches!(
        CanvasDocument::export_svg(&mut world, document, file.path(), None),
        Err(CanvasExportError::InvalidGeometry(_))
    ));
    assert_eq!(std::fs::read(file.path()).unwrap(), b"keep me");
    world
        .get_mut::<CanvasGeometry>(shape)
        .map(|mut geometry| {
            let mut rectangle = *geometry.rectangle().unwrap();
            rectangle.size.x = 20.0;
            *geometry = CanvasGeometry::Rectangle(rectangle);
        })
        .unwrap();
    CanvasDocument::export_svg(&mut world, document, file.path(), None).unwrap();
    assert!(
        std::fs::read_to_string(file.path())
            .unwrap()
            .contains("<svg")
    );
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        CanvasDocument::export_svg(&mut world, document, dir.path(), None),
        Err(CanvasExportError::Io(_))
    ));
}

#[test]
fn svg_is_unchanged_by_view_navigation_selection_and_uncommitted_drawing() {
    use bevy::picking::pointer::*;
    use common::*;
    let id = bevy_widgetry_test_utils::pointer_ids()[1];
    let (mut app, a, _, shape, _, mut at) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(app.world_mut(), document, file.path(), None).unwrap();
    let original = std::fs::read(file.path()).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    CanvasView::set_mode_in_world(
        app.world_mut(),
        a,
        CanvasMode::Rectangle(CanvasPaint::default()),
    )
    .unwrap();
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, id, &mut at, Vec2::splat(45.0));
    scroll(&mut app, id, &at, 2.0);
    CanvasView::set_center_in_world(app.world_mut(), a, Vec2::new(50.0, -80.0)).unwrap();
    update(&mut app);
    CanvasDocument::export_svg(app.world_mut(), document, file.path(), None).unwrap();
    assert_eq!(std::fs::read(file.path()).unwrap(), original);
    input(&mut app, id, &at, PointerAction::Cancel);
}

#[test]
fn svg_declares_linear_compositing_and_keeps_each_shapes_fill_and_stroke_together() {
    for upper_z in [0.0, 0.005, -9000.0, 9000.0] {
        let mut world = World::new();
        let document = world.spawn(CanvasDocument).id();
        for (size, z, paint) in [
            (
                20.0,
                0.0,
                CanvasPaint {
                    fill: Some(Color::srgb(1.0, 0.0, 0.0)),
                    stroke: Some(CanvasStroke {
                        color: Color::srgba(0.0, 1.0, 0.0, 0.5),
                        width: 4.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ),
            (
                30.0,
                upper_z,
                CanvasPaint {
                    fill: Some(Color::srgba(0.0, 0.0, 1.0, 0.5)),
                    stroke: None,
                    ..Default::default()
                },
            ),
        ] {
            CanvasDocument::create_rectangle(
                &mut world,
                document,
                CanvasRectangle {
                    corner_radius: 0.0,
                    size: Vec2::splat(size),
                },
                Transform::from_xyz(0.0, 0.0, z),
                paint,
            )
            .unwrap();
        }
        let file = tempfile::NamedTempFile::new().unwrap();
        CanvasDocument::export_svg(&mut world, document, file.path(), None).unwrap();
        let xml = std::fs::read_to_string(file.path()).unwrap();
        let dom = roxmltree::Document::parse(&xml).unwrap();
        assert_eq!(
            dom.root_element().attribute("color-interpolation"),
            Some("linearRGB")
        );
        let paints = dom
            .descendants()
            .filter(|node| node.has_tag_name("rect") && node.attribute("transform").is_some())
            .collect::<Vec<_>>();
        assert_eq!(paints.len(), 3);
        assert_eq!(paints[0].attribute("stroke"), Some("none"));
        assert_eq!(paints[1].attribute("fill"), Some("none"));
        assert_eq!(paints[2].attribute("stroke"), Some("none"));
        // At (9, 0), the front blue fill covers the back red fill and green stroke.
        // Interpret the exported paint colors and opacities in the declared SVG space.
        let mut composite = Vec3::ZERO;
        for node in paints {
            let kind = if node.attribute("fill") == Some("none") {
                "stroke"
            } else {
                "fill"
            };
            let rgb = values(
                node.attribute(kind)
                    .unwrap()
                    .trim_start_matches("rgb(")
                    .trim_end_matches(')')
                    .replace('%', "")
                    .as_str(),
            );
            let color = Srgba::rgb(rgb[0] / 100.0, rgb[1] / 100.0, rgb[2] / 100.0);
            let opacity = node
                .attribute(format!("{kind}-opacity").as_str())
                .unwrap()
                .parse::<f32>()
                .unwrap();
            let linear = LinearRgba::from(color);
            composite = Vec3::new(linear.red, linear.green, linear.blue) * opacity
                + composite * (1.0 - opacity);
        }
        let encoded = Srgba::from(LinearRgba::rgb(composite.x, composite.y, composite.z));
        assert!((encoded.red - 0.5370987).abs() < 0.0001);
        assert!((encoded.green - 0.5370987).abs() < 0.0001);
        assert!((encoded.blue - 0.735357).abs() < 0.0001);
    }
}

#[test]
fn svg_rejects_unrepresentable_hdr_and_out_of_range_alpha_before_touching_the_file() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"keep me").unwrap();
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    for color in [
        Color::linear_rgba(2.0, 0.0, 0.0, 0.5),
        Color::linear_rgba(-0.1, 0.0, 0.0, 1.0),
        Color::srgba(1.0, 0.0, 0.0, 1.5),
    ] {
        for stroke in [false, true] {
            let paint = if stroke {
                CanvasPaint {
                    fill: None,
                    stroke: Some(CanvasStroke {
                        color,
                        width: 2.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                }
            } else {
                CanvasPaint {
                    fill: Some(color),
                    stroke: None,
                    ..Default::default()
                }
            };
            CanvasDocument::update_rectangle(
                &mut world,
                document,
                shape,
                CanvasRectangle {
                    corner_radius: 0.0,
                    size: Vec2::splat(20.0),
                },
                Transform::default(),
                paint,
            )
            .unwrap();
            assert!(matches!(
                CanvasDocument::export_svg(&mut world, document, file.path(), None),
                Err(CanvasExportError::UnsupportedColor)
            ));
            assert_eq!(std::fs::read(file.path()).unwrap(), b"keep me");
        }
    }
}
