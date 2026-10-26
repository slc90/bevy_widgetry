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
fn svg_uses_document_bounds_centered_stroke_and_full_transform_without_a_view() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let size = Vec2::new(120.0, 80.0);
    let paint = CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            color: Color::srgba(0.1, 0.3, 0.8, 0.5),
            width: 10.0,
        }),
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
            CanvasRectangle { size },
            transform,
            paint,
        )
        .unwrap();
        CanvasDocument::export_svg(&mut world, document, file.path(), None).unwrap();
        let xml = std::fs::read_to_string(file.path()).unwrap();
        let dom = roxmltree::Document::parse(&xml).unwrap();
        let root = dom.root_element();
        let bounds = CanvasRectangle { size }.bounds(&transform, &paint).unwrap();
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
                size: Vec2::splat(20.0),
            },
            Transform::from_xyz(-20.0, -10.0, z),
            CanvasPaint {
                fill: Some(color),
                stroke: None,
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
            let CanvasGeometry::Rectangle(rectangle) = geometry.as_mut();
            rectangle.size.x = f32::NAN;
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
            let CanvasGeometry::Rectangle(rectangle) = geometry.as_mut();
            rectangle.size.x = 20.0;
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
                    }),
                },
            ),
            (
                30.0,
                upper_z,
                CanvasPaint {
                    fill: Some(Color::srgba(0.0, 0.0, 1.0, 0.5)),
                    stroke: None,
                },
            ),
        ] {
            CanvasDocument::create_rectangle(
                &mut world,
                document,
                CanvasRectangle {
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
                    stroke: Some(CanvasStroke { color, width: 2.0 }),
                }
            } else {
                CanvasPaint {
                    fill: Some(color),
                    stroke: None,
                }
            };
            CanvasDocument::update_rectangle(
                &mut world,
                document,
                shape,
                CanvasRectangle {
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
