#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{picking::pointer::*, prelude::*, ui::UiTransform};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::pointer_ids;
#[allow(dead_code)]
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;
use coordinates::at_document;

fn click(app: &mut App, id: PointerId, at: &Location) {
    input(app, id, at, PointerAction::Move { delta: Vec2::ONE });
    input(app, id, at, PointerAction::Press(PointerButton::Primary));
    input(app, id, at, PointerAction::Release(PointerButton::Primary));
}

#[test]
fn holes_and_subpath_gaps_pass_through_and_views_select_whole_shapes_independently() {
    for id in pointer_ids() {
        let (mut app, a, b, original, _, base) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        CanvasDocument::remove_shape(app.world_mut(), document, original).unwrap();
        let ellipse = CanvasDocument::create_ellipse(
            app.world_mut(),
            document,
            CanvasEllipse {
                radii: Vec2::new(15.0, 10.0),
            },
            Transform::default(),
            CanvasPaint::default(),
        )
        .unwrap();
        let path = CanvasDocument::create_path(
            app.world_mut(),
            document,
            CanvasPath {
                commands: vec![CanvasPathCommand::MoveTo(Vec2::ZERO)],
            },
            Transform::default(),
            CanvasPaint {
                fill_rule: CanvasFillRule::EvenOdd,
                ..default()
            },
        )
        .unwrap();
        CanvasDocument::append_rectangle(
            app.world_mut(),
            document,
            path,
            Rect::new(-40.0, -40.0, 40.0, 40.0),
        )
        .unwrap();
        CanvasDocument::append_rectangle(
            app.world_mut(),
            document,
            path,
            Rect::new(-20.0, -20.0, 20.0, 20.0),
        )
        .unwrap();
        CanvasDocument::append_rectangle(
            app.world_mut(),
            document,
            path,
            Rect::new(70.0, -10.0, 90.0, 10.0),
        )
        .unwrap();
        update(&mut app);
        for (point, expected) in [
            (Vec2::ZERO, Some(ellipse)),
            (Vec2::new(-30.0, 0.0), Some(path)),
            (Vec2::new(80.0, 0.0), Some(path)),
            (Vec2::new(55.0, 0.0), None),
            (Vec2::new(14.0, 9.0), None),
        ] {
            CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
            let at = at_document(&app, a, &base, point);
            click(&mut app, id, &at);
            assert_eq!(
                CanvasView::selection(app.world(), a).unwrap(),
                expected,
                "point={point}"
            );
        }
        let at = at_document(&app, b, &base, Vec2::ZERO);
        click(&mut app, id, &at);
        assert_eq!(
            CanvasView::selection(app.world(), b).unwrap(),
            Some(ellipse)
        );
        let at = at_document(&app, a, &base, Vec2::new(-30.0, 0.0));
        click(&mut app, id, &at);
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), Some(path));
        assert_eq!(
            CanvasView::selection(app.world(), b).unwrap(),
            Some(ellipse)
        );
        let before = *app.world().get::<Transform>(path).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&Name>()
                .iter(app.world())
                .filter(|name| name.as_str() == "CanvasSelectionOutline")
                .count(),
            2
        );
        let commands = match app.world().get::<CanvasGeometry>(path).unwrap() {
            CanvasGeometry::Path(path) => Some(path.clone()),
            _ => None,
        }
        .unwrap();
        CanvasDocument::update_path(
            app.world_mut(),
            document,
            path,
            commands,
            before,
            CanvasPaint::default(),
        )
        .unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
        let at = at_document(&app, a, &base, Vec2::ZERO);
        click(&mut app, id, &at);
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), Some(path));
        CanvasDocument::reorder_shape(app.world_mut(), document, path, CanvasOrder::Back).unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
        click(&mut app, id, &at);
        assert_eq!(
            CanvasView::selection(app.world(), a).unwrap(),
            Some(ellipse)
        );
        CanvasView::set_selection_in_world(app.world_mut(), a, Some(path)).unwrap();
        CanvasDocument::remove_shape(app.world_mut(), document, path).unwrap();
        update(&mut app);
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), None);
        assert_eq!(
            app.world_mut()
                .query::<&Name>()
                .iter(app.world())
                .filter(|name| name.as_str() == "CanvasSelectionOutline")
                .count(),
            1
        );
        assert_eq!(
            CanvasView::selection(app.world(), b).unwrap(),
            Some(ellipse)
        );
    }
}

#[test]
fn transparent_fill_has_geometry_but_none_and_ellipse_bounding_box_corners_do_not() {
    for id in pointer_ids() {
        let (mut app, view, _, shape, _, base) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), view).unwrap().unwrap();
        CanvasDocument::remove_shape(app.world_mut(), document, shape).unwrap();
        let ellipse = CanvasDocument::create_ellipse(
            app.world_mut(),
            document,
            CanvasEllipse {
                radii: Vec2::new(40.0, 20.0),
            },
            Transform::from_rotation(Quat::from_rotation_z(0.4))
                .with_scale(Vec3::new(-1.2, 0.8, 0.0)),
            CanvasPaint {
                fill: Some(Color::NONE),
                ..default()
            },
        )
        .unwrap();
        update(&mut app);
        let transform = *app.world().get::<Transform>(ellipse).unwrap();
        for (local, expected) in [(Vec2::ZERO, Some(ellipse)), (Vec2::new(35.0, 18.0), None)] {
            CanvasView::set_selection_in_world(app.world_mut(), view, None).unwrap();
            let at = at_document(
                &app,
                view,
                &base,
                transform.transform_point(local.extend(0.0)).truncate(),
            );
            click(&mut app, id, &at);
            assert_eq!(CanvasView::selection(app.world(), view).unwrap(), expected);
        }
        CanvasDocument::update_ellipse(
            app.world_mut(),
            document,
            ellipse,
            CanvasEllipse {
                radii: Vec2::new(40.0, 20.0),
            },
            transform,
            CanvasPaint {
                fill: None,
                stroke: Some(CanvasStroke {
                    width: 2.0,
                    ..default()
                }),
                ..default()
            },
        )
        .unwrap();
        let at = at_document(&app, view, &base, Vec2::ZERO);
        click(&mut app, id, &at);
        assert_eq!(CanvasView::selection(app.world(), view).unwrap(), None);
    }
}

#[test]
fn open_path_stroke_tolerance_stays_in_screen_pixels_across_zoom_and_ui_transforms() {
    for id in pointer_ids() {
        for zoom in [0.25, 1.0, 4.0] {
            let (mut app, view, _, shape, _, base) = fixture(id, 1.5, 1.5);
            let document = CanvasView::document(app.world(), view).unwrap().unwrap();
            CanvasDocument::remove_shape(app.world_mut(), document, shape).unwrap();
            let path = CanvasDocument::create_path(
                app.world_mut(),
                document,
                CanvasPath {
                    commands: vec![
                        CanvasPathCommand::MoveTo(Vec2::new(-40.0, 0.0)),
                        CanvasPathCommand::LineTo(Vec2::new(40.0, 0.0)),
                    ],
                },
                Transform::default(),
                CanvasPaint {
                    fill: None,
                    stroke: Some(CanvasStroke {
                        width: 2.0,
                        ..default()
                    }),
                    ..default()
                },
            )
            .unwrap();
            app.world_mut().entity_mut(view).insert(UiTransform {
                rotation: Rot2::radians(0.3),
                scale: Vec2::new(1.5, 0.6),
                ..default()
            });
            CanvasView::set_zoom_in_world(app.world_mut(), view, zoom).unwrap();
            update(&mut app);
            let edge = at_document(&app, view, &base, Vec2::new(0.0, 1.0));
            let middle = at_document(&app, view, &base, Vec2::ZERO);
            let normal = (edge.position - middle.position).normalize();
            for (distance, expected) in [(3.0, Some(path)), (6.0, None)] {
                CanvasView::set_selection_in_world(app.world_mut(), view, None).unwrap();
                let at = Location {
                    position: edge.position + normal * distance,
                    ..edge.clone()
                };
                click(&mut app, id, &at);
                assert_eq!(
                    CanvasView::selection(app.world(), view).unwrap(),
                    expected,
                    "zoom={zoom}, distance={distance}"
                );
            }
        }
    }
}

#[test]
fn rotated_large_ellipse_visible_mesh_edge_is_not_rejected_by_ideal_bounds() {
    let id = pointer_ids()[1];
    let (mut app, view, _, original, _, base) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasDocument::remove_shape(app.world_mut(), document, original).unwrap();
    let ellipse = CanvasEllipse {
        radii: Vec2::splat(1e8),
    };
    let transform = Transform::from_rotation(Quat::from_rotation_z(-0.1663));
    let shape = CanvasDocument::create_ellipse(
        app.world_mut(),
        document,
        ellipse,
        transform,
        CanvasPaint::default(),
    )
    .unwrap();
    update(&mut app);
    let handle = app
        .world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == "CanvasFill")
        .unwrap()
        .1
        .0
        .clone();
    let mesh = app.world().resource::<Assets<Mesh>>().get(&handle).unwrap();
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let x = (transform.rotation * Vec3::X).truncate().as_dvec2();
    let y = (transform.rotation * Vec3::Y).truncate().as_dvec2();
    let edge = positions
        .iter()
        .map(|p| x * f64::from(p[0]) + y * f64::from(p[1]))
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    let point = (edge * 0.999999).as_vec2();
    let ideal = ellipse.bounds(&transform, &CanvasPaint::default()).unwrap();
    assert!(
        point.x > ideal.max.x + 8.0,
        "fixture must lie outside the ideal broad phase: {point:?} vs {ideal:?}"
    );
    CanvasView::set_center_in_world(app.world_mut(), view, point).unwrap();
    update(&mut app);
    let at = at_document(&app, view, &base, point);
    click(&mut app, id, &at);
    assert_eq!(
        CanvasView::selection(app.world(), view).unwrap(),
        Some(shape)
    );
}
