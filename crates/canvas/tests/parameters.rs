#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{LogCapture, pointer_ids};
#[allow(dead_code)]
mod common;
use common::*;
#[path = "common/coordinates.rs"]
mod coordinates;
use coordinates::at_document;

#[test]
fn rounded_bounds_and_svg_preserve_the_rectangle_radius() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let rectangle = CanvasRectangle {
        size: Vec2::new(80.0, 60.0),
        corner_radius: 10.0,
    };
    let transform = Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4));
    let bounds = rectangle
        .bounds(&transform, &CanvasPaint::default())
        .unwrap();
    let extent = 50.0 / 2.0_f32.sqrt() + 10.0;
    assert!(bounds.max.distance(Vec2::splat(extent)) < 0.001);
    CanvasDocument::create_rectangle(
        &mut world,
        document,
        rectangle,
        transform,
        CanvasPaint::default(),
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rounded.svg");
    CanvasDocument::export_svg(&mut world, document, &path, None).unwrap();
    let svg = std::fs::read_to_string(path).unwrap();
    assert!(svg.contains("rx=\"10\" ry=\"10\""));
}

#[test]
fn parameter_sizes_clamp_before_crossing_the_fixed_corner_and_radius_fits_the_new_size() {
    let id = pointer_ids()[1];
    for ellipse in [false, true] {
        let (mut app, a, _, old, _, base) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        CanvasDocument::remove_shape(app.world_mut(), document, old).unwrap();
        let shape = if ellipse {
            CanvasDocument::create_ellipse(
                app.world_mut(),
                document,
                CanvasEllipse {
                    radii: Vec2::new(40.0, 30.0),
                },
                Transform::default(),
                CanvasPaint::default(),
            )
        } else {
            CanvasDocument::create_rectangle(
                app.world_mut(),
                document,
                CanvasRectangle {
                    size: Vec2::new(80.0, 60.0),
                    corner_radius: 20.0,
                },
                Transform::default(),
                CanvasPaint::default(),
            )
        }
        .unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
        update(&mut app);
        let mut at = at_document(&app, a, &base, Vec2::new(28.8, 21.6));
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        moved(&mut app, id, &mut at, Vec2::splat(-200.0));
        let expected = if ellipse {
            CanvasGeometry::Ellipse(CanvasEllipse {
                radii: Vec2::splat(0.5),
            })
        } else {
            CanvasGeometry::Rectangle(CanvasRectangle {
                size: Vec2::ONE,
                corner_radius: 0.5,
            })
        };
        assert_eq!(app.world().get::<CanvasGeometry>(shape), Some(&expected));
        let transform = *app.world().get::<Transform>(shape).unwrap();
        assert_eq!(transform.scale, Vec3::ONE);
        assert!(
            transform
                .transform_point(Vec3::splat(-0.5))
                .truncate()
                .distance(Vec2::new(-40.0, -30.0))
                < 0.001
        );
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        CanvasView::set_zoom_in_world(app.world_mut(), a, 10.0).unwrap();
        CanvasView::set_center_in_world(app.world_mut(), a, transform.translation.truncate())
            .unwrap();
        update(&mut app);
        let center = transform.translation.truncate();
        let start = center + Vec2::splat(0.325);
        at = at_document(&app, a, &base, start);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        let end = at_document(&app, a, &base, start + Vec2::splat(2.0));
        let delta = end.position - at.position;
        at = end;
        input(&mut app, id, &at, PointerAction::Move { delta });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        let size = match app.world().get::<CanvasGeometry>(shape).unwrap() {
            CanvasGeometry::Rectangle(r) => r.size,
            CanvasGeometry::Ellipse(e) => e.radii * 2.0,
            _ => panic!("must keep type"),
        };
        assert!(
            size.distance(Vec2::splat(3.0)) < 0.001,
            "regrab must resize local geometry: {size:?}"
        );
        assert_eq!(
            app.world().get::<Transform>(shape).unwrap().scale,
            Vec3::ONE
        );
        if !ellipse {
            let transform = *app.world().get::<Transform>(shape).unwrap();
            CanvasDocument::update_rectangle(
                app.world_mut(),
                document,
                shape,
                CanvasRectangle {
                    size: Vec2::ONE,
                    corner_radius: 0.0,
                },
                transform,
                CanvasPaint::default(),
            )
            .unwrap();
            CanvasView::set_center_in_world(app.world_mut(), a, transform.translation.truncate())
                .unwrap();
            update(&mut app);
            let start = transform.translation.truncate() - Vec2::splat(0.175);
            at = at_document(&app, a, &base, start);
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            let end = at_document(&app, a, &base, start + Vec2::splat(0.1));
            let delta = end.position - at.position;
            at = end;
            input(&mut app, id, &at, PointerAction::Move { delta });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            let rectangle = app
                .world()
                .get::<CanvasGeometry>(shape)
                .unwrap()
                .rectangle()
                .unwrap();
            assert!(rectangle.size.distance(Vec2::ONE) < 0.001);
            assert!((rectangle.corner_radius - 0.1).abs() < 0.001);
            assert_eq!(*app.world().get::<Transform>(shape).unwrap(), transform);
        }
    }
}

#[test]
fn radius_and_size_drags_keep_submitted_geometry_when_their_view_or_pointer_is_canceled() {
    let id = pointer_ids()[1];
    for kind in 0..3 {
        for cause in 0..9 {
            let (mut app, a, b, old, window, base) = fixture(id, 1.0, 1.0);
            let document = CanvasView::document(app.world(), a).unwrap().unwrap();
            CanvasDocument::remove_shape(app.world_mut(), document, old).unwrap();
            let shape = if kind == 1 {
                CanvasDocument::create_ellipse(
                    app.world_mut(),
                    document,
                    CanvasEllipse {
                        radii: Vec2::new(40.0, 30.0),
                    },
                    Transform::default(),
                    CanvasPaint::default(),
                )
            } else {
                CanvasDocument::create_rectangle(
                    app.world_mut(),
                    document,
                    CanvasRectangle {
                        size: Vec2::new(80.0, 60.0),
                        corner_radius: 10.0,
                    },
                    Transform::default(),
                    CanvasPaint::default(),
                )
            }
            .unwrap();
            CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
            update(&mut app);
            let start = if kind == 2 {
                let p = Vec2::new(-30.0, -20.0);
                p - p.normalize() * (p.length() * 0.65).min(26.0)
            } else {
                let p = Vec2::new(40.0, 30.0);
                p - p.normalize() * 14.0
            };
            let mut at = at_document(&app, a, &base, start);
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            moved(&mut app, id, &mut at, Vec2::new(10.0, 10.0));
            let submitted = app.world().get::<CanvasGeometry>(shape).unwrap().clone();
            if kind == 2 {
                assert_eq!(
                    submitted,
                    CanvasGeometry::Rectangle(CanvasRectangle {
                        size: Vec2::new(80.0, 60.0),
                        corner_radius: 20.0
                    })
                );
            }
            let position = *app.world().get::<Transform>(shape).unwrap();
            match cause {
                0 => input(&mut app, id, &at, PointerAction::Cancel),
                1 => app.world_mut().get_mut::<Window>(window).unwrap().focused = false,
                2 => {
                    app.world_mut().entity_mut(a).insert(Visibility::Hidden);
                }
                3 => {
                    app.world_mut()
                        .entity_mut(a)
                        .insert(bevy::ui::InteractionDisabled);
                }
                4 => {
                    CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
                }
                5 => {
                    CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
                }
                6 => {
                    app.world_mut()
                        .resource_mut::<bevy::input_focus::InputFocus>()
                        .set(b, bevy::input_focus::FocusCause::Navigated);
                }
                7 => {
                    app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None;
                }
                _ => {
                    app.world_mut().despawn(a);
                }
            }
            update(&mut app);
            assert!(
                !app.world().resource::<PointerCaptureMap>().is_captured(&id),
                "kind={kind},cause={cause}"
            );
            moved(&mut app, id, &mut at, Vec2::new(5.0, 5.0));
            assert_eq!(app.world().get::<CanvasGeometry>(shape), Some(&submitted));
            assert_eq!(*app.world().get::<Transform>(shape).unwrap(), position);
        }
    }
}

#[test]
fn rectangle_corner_radius_is_validated_transactionally_and_changes_actual_picking() {
    let id = pointer_ids()[1];
    let (mut app, a, _, shape, _, base) = fixture(id, 1.0, 1.0);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let rectangle = CanvasRectangle {
        size: Vec2::splat(80.0),
        corner_radius: 30.0,
    };
    CanvasDocument::update_rectangle(
        app.world_mut(),
        document,
        shape,
        rectangle,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    for radius in [-1.0, 41.0, f32::NAN, f32::INFINITY] {
        let logs = LogCapture::default();
        let e = logs
            .run(|| {
                CanvasDocument::update_rectangle(
                    app.world_mut(),
                    document,
                    shape,
                    CanvasRectangle {
                        corner_radius: radius,
                        ..rectangle
                    },
                    Transform::default(),
                    CanvasPaint::default(),
                )
            })
            .unwrap_err();
        assert_eq!(e.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(
            app.world().get::<CanvasGeometry>(shape),
            Some(&CanvasGeometry::Rectangle(rectangle))
        );
        assert!(!logs.records().is_empty());
    }
    update(&mut app);
    for (point, selected) in [
        (Vec2::new(38.0, -38.0), None),
        (Vec2::new(0.0, -38.0), Some(shape)),
    ] {
        let at = at_document(&app, a, &base, point);
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(
            CanvasView::single_selection(app.world(), a).unwrap(),
            selected
        );
    }
}

#[test]
fn typed_size_handles_keep_the_opposite_geometry_corner_and_original_transform_scale() {
    for axes in [
        Vec2::new(-1.0, -1.0),
        Vec2::NEG_Y,
        Vec2::new(1.0, -1.0),
        Vec2::X,
        Vec2::ONE,
        Vec2::Y,
        Vec2::new(-1.0, 1.0),
        Vec2::NEG_X,
    ] {
        for id in pointer_ids() {
            for ellipse in [false, true] {
                let (mut app, a, b, old, _, base) = fixture(id, 1.0, 1.0);
                let document = CanvasView::document(app.world(), a).unwrap().unwrap();
                CanvasDocument::remove_shape(app.world_mut(), document, old).unwrap();
                let original = Transform::from_xyz(0.0, 0.0, 5.0)
                    .with_rotation(Quat::from_rotation_z(0.3))
                    .with_scale(Vec3::new(-1.2, 0.8, 0.0));
                let shape = if ellipse {
                    CanvasDocument::create_ellipse(
                        app.world_mut(),
                        document,
                        CanvasEllipse {
                            radii: Vec2::new(40.0, 30.0),
                        },
                        original,
                        CanvasPaint::default(),
                    )
                } else {
                    CanvasDocument::create_rectangle(
                        app.world_mut(),
                        document,
                        CanvasRectangle {
                            size: Vec2::new(80.0, 60.0),
                            corner_radius: 10.0,
                        },
                        original,
                        CanvasPaint::default(),
                    )
                }
                .unwrap();
                CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
                CanvasView::set_selection_in_world(app.world_mut(), b, Some(shape)).unwrap();
                update(&mut app);
                let corner = original
                    .transform_point((Vec2::new(40.0, 30.0) * axes).extend(0.0))
                    .truncate();
                let center = original.translation.truncate();
                let start = corner
                    + (center - corner).normalize()
                        * 14.0_f32.min((center - corner).length() * 0.35);
                let mut at = at_document(&app, a, &base, start);
                input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
                input(
                    &mut app,
                    id,
                    &at,
                    PointerAction::Press(PointerButton::Primary),
                );
                assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
                let delta = (original.rotation
                    * ((Vec2::new(20.0, 10.0) * axes).extend(0.0) * original.scale))
                    .truncate();
                let next = at_document(&app, a, &base, start + delta);
                let d = next.position - at.position;
                at = next;
                input(&mut app, id, &at, PointerAction::Move { delta: d });
                let changed = *app.world().get::<Transform>(shape).unwrap();
                assert_eq!(changed.rotation, original.rotation);
                assert_eq!(changed.scale, original.scale);
                let size = match app.world().get::<CanvasGeometry>(shape).unwrap() {
                    CanvasGeometry::Rectangle(r) => r.size,
                    CanvasGeometry::Ellipse(e) => e.radii * 2.0,
                    _ => panic!("must retain parametric type"),
                };
                assert!(
                    size.distance(Vec2::new(80.0, 60.0) + Vec2::new(20.0, 10.0) * axes.abs())
                        < 0.002,
                    "size={size:?}, axes={axes:?}, ellipse={ellipse}"
                );
                assert!(
                    changed
                        .transform_point((-size * axes * 0.5).extend(0.0))
                        .truncate()
                        .distance(
                            original
                                .transform_point((-Vec2::new(40.0, 30.0) * axes).extend(0.0))
                                .truncate()
                        )
                        < 0.002
                );
                input(&mut app, id, &at, PointerAction::Cancel);
                assert_eq!(*app.world().get::<Transform>(shape).unwrap(), changed);
                assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            }
        }
    }
}
