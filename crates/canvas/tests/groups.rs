#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    picking::{
        PickingSettings,
        events::PointerPress,
        pointer::{Location, PointerAction, PointerButton, PointerId},
    },
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, pointer_ids, queue_key, queue_pointer};

mod common;
use common::{input, moved, scroll, update};
#[path = "common/coordinates.rs"]
mod coordinates;
use coordinates::at_document;

const ITEM_SIZE: f32 = 12.0;
const TOLERANCE: f32 = 0.001;

fn fixture(
    pointer: PointerId,
    scale: f32,
    ui_scale: f32,
) -> (App, Entity, Entity, Entity, Entity, Location) {
    let (mut app, first, second, shape, window, location) =
        common::fixture(pointer, scale, ui_scale);
    // Picking uses wall time, so thread scheduling cannot determine the click count in these tests.
    app.world_mut()
        .resource_mut::<PickingSettings>()
        .multi_click_interval = std::time::Duration::MAX;
    (app, first, second, shape, window, location)
}

fn rectangle(world: &mut World, document: Entity, at: Vec2) -> Entity {
    CanvasDocument::create_rectangle(
        world,
        document,
        CanvasRectangle {
            size: Vec2::splat(ITEM_SIZE),
            corner_radius: 0.0,
        },
        Transform::from_translation(at.extend(0.0)),
        CanvasPaint::default(),
    )
    .unwrap()
}

fn near(actual: Vec2, expected: Vec2) {
    assert!(
        actual.abs_diff_eq(expected, TOLERANCE),
        "{actual} != {expected}"
    );
}

#[test]
fn grouping_and_ungrouping_keep_identity_order_and_projected_geometry() {
    let mut world = World::new();
    let doc = world
        .spawn((CanvasDocument, Transform::from_xyz(100.0, 30.0, 0.0)))
        .id();
    let layer = CanvasDocument::default_layer(&world, doc).unwrap();
    let a = rectangle(&mut world, doc, Vec2::new(-20.0, 0.0));
    let b = rectangle(&mut world, doc, Vec2::new(40.0, 20.0));
    let c = rectangle(&mut world, doc, Vec2::new(80.0, 0.0));
    let inner = CanvasDocument::group_items(&mut world, doc, &[b, a], "Inner").unwrap();
    assert_eq!(world.get::<CanvasGroup>(inner).unwrap().name(), "Inner");
    assert_eq!(
        CanvasDocument::item_order(&world, doc, CanvasContainer::Group(inner)).unwrap(),
        [a, b]
    );
    assert_eq!(
        CanvasDocument::shape_order(&mut world, doc).unwrap(),
        [a, b, c]
    );
    near(
        world
            .get::<Transform>(inner)
            .unwrap()
            .translation
            .truncate(),
        Vec2::new(10.0, 10.0),
    );
    near(
        CanvasDocument::document_transform(&world, doc, a)
            .unwrap()
            .translation
            .truncate(),
        Vec2::new(-20.0, 0.0),
    );

    let outer = CanvasDocument::group_items(&mut world, doc, &[inner, c], "Outer").unwrap();
    assert_eq!(
        CanvasDocument::item_order(&world, doc, CanvasContainer::Layer(layer)).unwrap(),
        [outer]
    );
    let transform = Transform::from_xyz(80.0, 50.0, 0.0)
        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::new(2.0, 2.0, 1.0));
    CanvasDocument::update_group_transform(&mut world, doc, outer, transform).unwrap();
    let a_local = world
        .get::<Transform>(inner)
        .unwrap()
        .transform_point(world.get::<Transform>(a).unwrap().translation);
    let expected = transform.transform_point(a_local).truncate();
    near(
        CanvasDocument::document_transform(&world, doc, a)
            .unwrap()
            .translation
            .truncate(),
        expected,
    );

    let logs = LogCapture::default();
    let invalid = transform.with_scale(Vec3::new(2.0, 1.0, 1.0));
    assert!(
        logs.run(|| CanvasDocument::update_group_transform(&mut world, doc, outer, invalid))
            .is_err()
    );
    assert_eq!(*world.get::<Transform>(outer).unwrap(), transform);
    assert_eq!(
        CanvasDocument::ungroup(&mut world, doc, outer).unwrap(),
        [inner, c]
    );
    assert_eq!(
        CanvasDocument::ungroup(&mut world, doc, inner).unwrap(),
        [a, b]
    );
    assert_eq!(
        CanvasDocument::item_order(&world, doc, CanvasContainer::Layer(layer)).unwrap(),
        [a, b, c]
    );
    near(
        world.get::<Transform>(a).unwrap().translation.truncate(),
        expected,
    );
}

#[test]
fn groups_cannot_mix_items_from_different_containers() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let a = rectangle(&mut world, doc, Vec2::ZERO);
    let b = rectangle(&mut world, doc, Vec2::splat(30.0));
    let other = CanvasDocument::create_layer(&mut world, doc, 1, "Other").unwrap();
    CanvasDocument::move_item_to_layer(&mut world, doc, b, other).unwrap();
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::group_items(&mut world, doc, &[a, b], "Mixed"))
            .is_err()
    );
    assert_eq!(CanvasDocument::draw_order(&world, doc).unwrap(), [a, b]);
}

#[test]
fn grouping_requires_a_contiguous_range_to_preserve_the_global_order() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let [a, b, c] = [-4.0, 0.0, 4.0].map(|x| rectangle(&mut world, document, Vec2::new(x, 0.0)));
    let logs = LogCapture::default();

    assert!(
        logs.run(|| CanvasDocument::group_items(&mut world, document, &[a, c], "Separated"))
            .is_err()
    );
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [a, b, c]
    );

    let group = CanvasDocument::group_items(&mut world, document, &[c, b], "Contiguous").unwrap();
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [a, b, c]
    );
    assert_eq!(
        CanvasDocument::ungroup(&mut world, document, group).unwrap(),
        [b, c]
    );
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [a, b, c]
    );
}

#[test]
fn group_origin_excludes_unpainted_shapes() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let unpainted = CanvasPaint {
        fill: None,
        ..default()
    };
    let visible = rectangle(&mut world, document, Vec2::ZERO);
    let rectangle = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(ITEM_SIZE),
            corner_radius: 0.0,
        },
        Transform::from_xyz(1000.0, 0.0, 0.0),
        unpainted,
    )
    .unwrap();
    let ellipse = CanvasDocument::create_ellipse(
        &mut world,
        document,
        CanvasEllipse {
            radii: Vec2::splat(ITEM_SIZE / 2.0),
        },
        Transform::from_xyz(0.0, 1000.0, 0.0),
        unpainted,
    )
    .unwrap();
    let group = CanvasDocument::group_items(
        &mut world,
        document,
        &[visible, rectangle, ellipse],
        "Group",
    )
    .unwrap();

    near(
        world
            .get::<Transform>(group)
            .unwrap()
            .translation
            .truncate(),
        Vec2::ZERO,
    );
}

#[test]
fn grouping_keeps_a_painted_origin_with_distant_unpainted_members() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let painted_origin = Vec2::new(8192.0, 0.0);
    let painted = rectangle(&mut world, document, painted_origin);
    let unpainted = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(ITEM_SIZE),
            corner_radius: 0.0,
        },
        Transform::from_xyz(100.2, 0.0, 0.0),
        CanvasPaint {
            fill: None,
            ..default()
        },
    )
    .unwrap();
    let before = CanvasDocument::document_transform(&world, document, unpainted).unwrap();

    let group =
        CanvasDocument::group_items(&mut world, document, &[painted, unpainted], "Group").unwrap();

    let after = CanvasDocument::document_transform(&world, document, unpainted).unwrap();
    assert_eq!(
        world
            .get::<Transform>(group)
            .unwrap()
            .translation
            .truncate(),
        painted_origin
    );
    near(after.translation.truncate(), before.translation.truncate());
}

#[test]
fn grouping_accepts_subpixel_rounding_across_a_4k_document() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let start = Vec2::new(-0.12, 0.0);
    let a = rectangle(&mut world, document, start);
    let b = rectangle(&mut world, document, Vec2::new(4096.0, 0.0));

    let group = CanvasDocument::group_items(&mut world, document, &[a, b], "Group").unwrap();

    let after = CanvasDocument::document_transform(&world, document, a).unwrap();
    near(after.translation.truncate(), start);
    assert_eq!(
        CanvasDocument::item_order(&world, document, CanvasContainer::Group(group)).unwrap(),
        [a, b]
    );
}

#[test]
fn group_origin_uses_the_drawn_path_instead_of_an_isolated_move() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let path = CanvasDocument::create_path(
        &mut world,
        document,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::ZERO),
                CanvasPathCommand::LineTo(10.0 * Vec2::X),
                CanvasPathCommand::MoveTo(Vec2::splat(10_000.0)),
            ],
        },
        Transform::IDENTITY,
        CanvasPaint {
            stroke: Some(CanvasStroke::default()),
            ..default()
        },
    )
    .unwrap();
    let sibling = rectangle(&mut world, document, Vec2::new(20.0, 0.0));
    let group =
        CanvasDocument::group_items(&mut world, document, &[path, sibling], "Group").unwrap();

    near(
        world
            .get::<Transform>(group)
            .unwrap()
            .translation
            .truncate(),
        Vec2::new(13.0, 0.0),
    );
}

#[test]
fn a_fill_only_group_path_ignores_contours_without_a_drawn_region() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let item = CanvasDocument::create_path(
        &mut world,
        document,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::ZERO),
                CanvasPathCommand::LineTo(10.0 * Vec2::X),
                CanvasPathCommand::LineTo(10.0 * Vec2::Y),
                CanvasPathCommand::Close,
            ],
        },
        Transform::IDENTITY,
        CanvasPaint {
            fill: Some(Color::WHITE),
            fill_rule: CanvasFillRule::EvenOdd,
            stroke: None,
        },
    )
    .unwrap();
    let far = Vec2::splat(10_000.0);
    CanvasDocument::append_line(&mut world, document, item, far, far + 100.0 * Vec2::X).unwrap();
    let canceled = [far, far + 100.0 * Vec2::X, far + 100.0 * Vec2::Y];
    for _ in 0..2 {
        CanvasDocument::append_polygon(&mut world, document, item, &canceled).unwrap();
    }
    let sibling = rectangle(&mut world, document, Vec2::new(20.0, 0.0));
    let group =
        CanvasDocument::group_items(&mut world, document, &[item, sibling], "Group").unwrap();

    near(
        world
            .get::<Transform>(group)
            .unwrap()
            .translation
            .truncate(),
        Vec2::new(13.0, 2.0),
    );
}

#[test]
fn group_bounds_keep_a_painted_triangle_with_one_very_short_edge() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let far = 2.0_f32.powi(55);
    let path = CanvasDocument::create_path(
        &mut world,
        document,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::splat(far)),
                CanvasPathCommand::LineTo(Vec2::Y),
                CanvasPathCommand::LineTo(Vec2::ZERO),
                CanvasPathCommand::Close,
            ],
        },
        Transform::IDENTITY,
        CanvasPaint {
            fill: Some(Color::WHITE),
            stroke: None,
            ..default()
        },
    )
    .unwrap();
    let sibling = rectangle(&mut world, document, Vec2::ZERO);
    let group =
        CanvasDocument::group_items(&mut world, document, &[path, sibling], "Group").unwrap();
    assert_eq!(
        world
            .get::<Transform>(group)
            .unwrap()
            .translation
            .truncate(),
        Vec2::splat(far * 0.5)
    );
}

#[test]
fn grouping_rejects_reparenting_that_loses_existing_positions() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let a = rectangle(&mut world, doc, Vec2::new(10.0, 0.0));
    let b = rectangle(&mut world, doc, Vec2::new(1.0e9, 0.0));
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::group_items(&mut world, doc, &[a, b], "Too far"))
            .is_err()
    );
    assert_eq!(CanvasDocument::draw_order(&world, doc).unwrap(), [a, b]);
    assert_eq!(world.get::<Transform>(a).unwrap().translation.x, 10.0);
}

#[test]
fn ungrouping_rejects_a_local_transform_that_would_displace_a_member() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let a = rectangle(&mut world, doc, Vec2::new(10.0, 0.0));
    let b = rectangle(&mut world, doc, Vec2::new(20.0, 0.0));
    let c = rectangle(&mut world, doc, Vec2::new(60.0, 0.0));
    let inner = CanvasDocument::group_items(&mut world, doc, &[a, b], "Inner").unwrap();
    let outer = CanvasDocument::group_items(&mut world, doc, &[inner, c], "Outer").unwrap();
    CanvasDocument::update_group_transform(
        &mut world,
        doc,
        outer,
        Transform::from_xyz(1.0e9, 0.0, 0.0),
    )
    .unwrap();
    CanvasDocument::update_group_transform(
        &mut world,
        doc,
        inner,
        Transform::from_xyz(-1.0e9, 0.0, 0.0),
    )
    .unwrap();
    let before = CanvasDocument::document_transform(&world, doc, a).unwrap();
    assert_eq!(before.translation.x, -5.0);
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::ungroup(&mut world, doc, inner))
            .is_err()
    );
    assert_eq!(
        CanvasDocument::document_transform(&world, doc, a).unwrap(),
        before
    );
    assert_eq!(
        CanvasDocument::item_order(&world, doc, CanvasContainer::Group(outer)).unwrap(),
        [inner, c]
    );
}

#[test]
fn nested_grouping_checks_reparenting_precision_in_document_space() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let tiny_scale = 2.0_f32.powi(-30);
    let near_origin = 2.0_f32.powi(-24);
    let [a, b, c] = [near_origin, 8.0, -8.0].map(|x| {
        CanvasDocument::create_rectangle(
            &mut world,
            doc,
            CanvasRectangle {
                size: Vec2::ONE,
                corner_radius: 0.0,
            },
            Transform::from_xyz(x, 0.0, 0.0).with_scale(Vec3::new(tiny_scale, tiny_scale, 1.0)),
            CanvasPaint::default(),
        )
        .unwrap()
    });
    let parent = CanvasDocument::group_items(&mut world, doc, &[a, b, c], "Parent").unwrap();
    let scale = 2.0_f32.powi(28);
    CanvasDocument::update_group_transform(
        &mut world,
        doc,
        parent,
        Transform::from_scale(Vec3::new(scale, scale, 1.0)),
    )
    .unwrap();
    let before = CanvasDocument::document_transform(&world, doc, a).unwrap();
    assert_eq!(before.translation.x, 16.0);
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::group_items(&mut world, doc, &[a, b], "Child"))
            .is_err()
    );
    assert_eq!(
        CanvasDocument::document_transform(&world, doc, a).unwrap(),
        before
    );
    assert_eq!(
        CanvasDocument::item_order(&world, doc, CanvasContainer::Group(parent)).unwrap(),
        [a, b, c]
    );
}

#[test]
fn grouping_accepts_local_rounding_when_document_positions_are_preserved() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let a = rectangle(&mut world, doc, Vec2::ZERO);
    let b = rectangle(&mut world, doc, Vec2::new(32.0, 0.0));
    let parent = CanvasDocument::group_items(&mut world, doc, &[a, b], "Parent").unwrap();
    let small_scale = 2.0_f32.powi(-10);
    CanvasDocument::update_group_transform(
        &mut world,
        doc,
        parent,
        Transform::from_xyz(1.0, 0.0, 0.0).with_scale(Vec3::new(small_scale, small_scale, 1.0)),
    )
    .unwrap();
    for (item, x) in [(a, 1024.0 + 2.0_f32.powi(-13)), (b, 8192.0)] {
        CanvasDocument::update_transform(&mut world, doc, item, Transform::from_xyz(x, 0.0, 0.0))
            .unwrap();
    }
    let old = [a, b].map(|item| CanvasDocument::document_transform(&world, doc, item).unwrap());
    assert_eq!(old[0].translation.x, 2.0);
    let nested = CanvasDocument::group_items(&mut world, doc, &[a, b], "Nested").unwrap();
    assert_eq!(
        CanvasDocument::item_order(&world, doc, CanvasContainer::Group(nested)).unwrap(),
        [a, b]
    );
    assert_eq!(
        [a, b].map(|item| CanvasDocument::document_transform(&world, doc, item).unwrap()),
        old,
    );
}

#[test]
fn regrouping_cannot_move_a_rotated_group_member_in_the_actual_view_projection() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let a = rectangle(&mut world, doc, Vec2::ZERO);
    let b = rectangle(&mut world, doc, Vec2::X * 32.0);
    let parent = CanvasDocument::group_items(&mut world, doc, &[a, b], "Parent").unwrap();
    let large = 2.0_f32.powi(48);
    let diag = std::f32::consts::FRAC_1_SQRT_2;
    CanvasDocument::update_group_transform(
        &mut world,
        doc,
        parent,
        Transform::from_xyz(0.0, -large, 0.0).with_rotation(Quat::from_xyzw(0.0, 0.0, diag, diag)),
    )
    .unwrap();
    for (item, x) in [(a, large + 2.0_f32.powi(25)), (b, 2.0_f32.powi(53) - large)] {
        CanvasDocument::update_transform(&mut world, doc, item, Transform::from_xyz(x, 0.0, 0.0))
            .unwrap();
    }
    let before = *world.get::<Transform>(a).unwrap();
    let visible =
        world.get::<Transform>(parent).unwrap().compute_affine() * before.compute_affine();
    let computed = CanvasDocument::document_transform(&world, doc, a).unwrap();
    near(
        computed.translation.truncate(),
        Vec2::new(visible.translation.x, visible.translation.y),
    );
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::group_items(&mut world, doc, &[a, b], "Nested"))
            .is_err()
    );
    assert_eq!(*world.get::<Transform>(a).unwrap(), before);
    assert_eq!(
        CanvasDocument::item_order(&world, doc, CanvasContainer::Group(parent)).unwrap(),
        [a, b]
    );
}

#[test]
fn regrouping_checks_descendants_even_if_the_direct_group_origin_rounds_within_tolerance() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let a = rectangle(&mut world, doc, Vec2::ZERO);
    let b = rectangle(&mut world, doc, Vec2::X * 32.0);
    let old_group = CanvasDocument::group_items(&mut world, doc, &[a, b], "Existing").unwrap();
    let c = rectangle(&mut world, doc, Vec2::X * 917_504.0);
    let origin = 9.0 * 2.0_f32.powi(-10);
    CanvasDocument::update_group_transform(
        &mut world,
        doc,
        old_group,
        Transform::from_xyz(origin, 0.0, 0.0),
    )
    .unwrap();
    for item in [a, b] {
        CanvasDocument::update_transform(
            &mut world,
            doc,
            item,
            Transform::from_xyz(131_072.0, 0.0, 0.0),
        )
        .unwrap();
    }
    let before = CanvasDocument::document_transform(&world, doc, a).unwrap();
    assert_eq!(before.translation.x, 131_072.0 + 2.0_f32.powi(-6));
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::group_items(&mut world, doc, &[old_group, c], "Would drift"))
            .is_err()
    );
    assert_eq!(
        CanvasDocument::draw_order(&world, doc).unwrap(),
        [old_group, a, b, c]
    );
    assert_eq!(
        CanvasDocument::document_transform(&world, doc, a).unwrap(),
        before
    );
}

#[test]
fn ungrouping_checks_far_path_geometry_when_axis_error_is_small() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let paint = CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke::default()),
        ..default()
    };
    let path = |start: Vec2| CanvasPath {
        commands: vec![
            CanvasPathCommand::MoveTo(start),
            CanvasPathCommand::LineTo(start + 16.0 * Vec2::X),
        ],
    };
    let item = CanvasDocument::create_path(
        &mut world,
        doc,
        path(Vec2::ZERO),
        Transform::IDENTITY,
        paint,
    )
    .unwrap();
    let other = rectangle(&mut world, doc, Vec2::ZERO);
    let group = CanvasDocument::group_items(&mut world, doc, &[item, other], "Group").unwrap();
    let component = std::f32::consts::FRAC_1_SQRT_2;
    let rotation = Quat::from_xyzw(0.0, 0.0, component, component);
    let distance = 2.0_f32.powi(24);
    let local = Transform::from_xyz(0.0, -distance, 0.0).with_rotation(rotation);
    let parent = Transform::from_rotation(rotation);
    let start = Vec2::new(distance, 0.0);
    CanvasDocument::update_path(&mut world, doc, item, path(start), local, paint).unwrap();
    CanvasDocument::update_group_transform(&mut world, doc, group, parent).unwrap();

    let before = CanvasDocument::document_transform(&world, doc, item).unwrap();
    let expanded = parent
        .mul_transform(local)
        .with_translation(parent.compute_affine().transform_point3(local.translation))
        .compute_affine();
    assert_eq!(before.translation, expanded.translation);
    assert!(
        before
            .transform_point3(start.extend(0.0))
            .distance(expanded.transform_point3(start.extend(0.0)))
            > 1.0
    );
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::ungroup(&mut world, doc, group))
            .is_err()
    );
    assert_eq!(
        CanvasDocument::document_transform(&world, doc, item).unwrap(),
        before
    );
    assert_eq!(
        CanvasDocument::container_of(&world, doc, item).unwrap(),
        CanvasContainer::Group(group)
    );
}

#[test]
fn ungrouping_accepts_a_rotated_circle_whose_geometry_stays_within_tolerance() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let component = std::f32::consts::FRAC_1_SQRT_2;
    let transform = Transform::from_rotation(Quat::from_xyzw(0.0, 0.0, component, component));
    let radius = 43_000.0;
    let circle = CanvasDocument::create_ellipse(
        &mut world,
        doc,
        CanvasEllipse {
            radii: Vec2::splat(radius),
        },
        transform,
        CanvasPaint::default(),
    )
    .unwrap();
    let other = rectangle(&mut world, doc, Vec2::ZERO);
    let group = CanvasDocument::group_items(&mut world, doc, &[circle, other], "Group").unwrap();
    CanvasDocument::update_group_transform(&mut world, doc, group, transform).unwrap();
    let before = CanvasDocument::document_transform(&world, doc, circle).unwrap();
    assert_eq!(before.translation, bevy::math::Vec3A::ZERO);

    assert_eq!(
        CanvasDocument::ungroup(&mut world, doc, group).unwrap(),
        [circle, other]
    );
    let after = CanvasDocument::document_transform(&world, doc, circle).unwrap();
    let x = (before.matrix3.x_axis - after.matrix3.x_axis).truncate();
    let y = (before.matrix3.y_axis - after.matrix3.y_axis).truncate();
    assert!(radius * (x + y).abs().max_element() > 0.01);
    let largest_error = radius * x.x.hypot(y.x).max(x.y.hypot(y.y));
    assert!(largest_error < 0.01);
}

#[test]
fn ungrouping_checks_quadratic_path_extrema_between_render_mesh_vertices() {
    let component = std::f32::consts::FRAC_1_SQRT_2;
    let transform = Transform::from_rotation(Quat::from_xyzw(0.0, 0.0, component, component));
    let path = |x| CanvasPath {
        commands: vec![
            CanvasPathCommand::MoveTo(Vec2::new(x, -1600.0)),
            CanvasPathCommand::QuadTo {
                control: Vec2::new(x + 102_400.0, 0.0),
                to: Vec2::new(x, 1600.0),
            },
            CanvasPathCommand::Close,
        ],
    };
    let paint = CanvasPaint {
        fill: Some(Color::WHITE),
        stroke: None,
        ..default()
    };
    let directory = tempfile::tempdir().unwrap();
    for (x, should_expand) in [(32_681.25, false), (32_650.0, true)] {
        let mut world = World::new();
        let doc = world.spawn(CanvasDocument).id();
        let shape =
            CanvasDocument::create_path(&mut world, doc, path(0.0), Transform::IDENTITY, paint)
                .unwrap();
        let other = rectangle(&mut world, doc, Vec2::ZERO);
        let group = CanvasDocument::group_items(&mut world, doc, &[shape, other], "Group").unwrap();
        CanvasDocument::update_path(&mut world, doc, shape, path(x), transform, paint).unwrap();
        CanvasDocument::update_group_transform(&mut world, doc, group, transform).unwrap();
        let before = CanvasDocument::document_transform(&world, doc, shape).unwrap();
        let exported_x = |world: &mut World, filename: &str| {
            let file = directory.path().join(filename);
            CanvasDocument::export_svg(
                world,
                doc,
                &file,
                Some(Rect::from_center_size(Vec2::ZERO, Vec2::splat(400_000.0))),
            )
            .unwrap();
            let svg = std::fs::read_to_string(file).unwrap();
            let matrix = svg
                .split_once("matrix(")
                .unwrap()
                .1
                .split_once(')')
                .unwrap()
                .0;
            let coefficients: Vec<f64> = matrix
                .split_whitespace()
                .map(|v| v.parse().unwrap())
                .collect();
            coefficients[0] * (f64::from(x) + 51_200.0) + coefficients[4]
        };
        let before_svg = should_expand.then(|| exported_x(&mut world, "before.svg"));
        let logs = LogCapture::default();
        let result = logs.run(|| CanvasDocument::ungroup(&mut world, doc, group));
        assert_eq!(result.is_ok(), should_expand, "path starting at x={x}");
        if should_expand {
            assert_eq!(result.unwrap(), [shape, other]);
            assert!((exported_x(&mut world, "after.svg") - before_svg.unwrap()).abs() <= 0.01);
        } else {
            assert_eq!(
                CanvasDocument::document_transform(&world, doc, shape).unwrap(),
                before
            );
            assert_eq!(
                CanvasDocument::container_of(&world, doc, shape).unwrap(),
                CanvasContainer::Group(group)
            );
        }
    }
}

#[test]
fn ungrouping_preserves_valid_path_strokes_and_ignores_moves_without_segments() {
    use CanvasPathCommand::{ArcTo, Close, LineTo, MoveTo};
    let component = std::f32::consts::FRAC_1_SQRT_2;
    let transform = Transform::from_rotation(Quat::from_xyzw(0.0, 0.0, component, component));
    let stroke = CanvasStroke {
        width: 1000.0,
        ..default()
    };
    let line = |extent: f32| {
        vec![
            MoveTo(Vec2::new(-extent, 0.0)),
            LineTo(Vec2::new(extent, 0.0)),
        ]
    };
    let triangle = vec![
        MoveTo(Vec2::ZERO),
        LineTo(10.0 * Vec2::X),
        LineTo(10.0 * Vec2::Y),
        Close,
        MoveTo(Vec2::new(100_000.0, 0.0)),
    ];
    let stroke_only = |cap| CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke { cap, ..stroke }),
        ..default()
    };
    let far_move = MoveTo(Vec2::new(100_000.0, 0.0));
    let early_curve = [
        MoveTo(Vec2::ZERO),
        CanvasPathCommand::QuadTo {
            control: Vec2::new(5.0, 5.0),
            to: 10.0 * Vec2::X,
        },
    ];
    let collapsed_arc = [
        MoveTo(Vec2::new(-83_000.0, 0.0)),
        ArcTo {
            radii: Vec2::Y,
            rotation: 0.0,
            large_arc: false,
            sweep: true,
            to: Vec2::new(83_000.0, 0.0),
        },
    ];
    let mut wrong_results = Vec::new();
    for (name, commands, paint, expand) in [
        (
            "butt",
            line(83_000.0),
            stroke_only(CanvasLineCap::Butt),
            true,
        ),
        (
            "long butt",
            line(84_000.0),
            stroke_only(CanvasLineCap::Butt),
            false,
        ),
        (
            "round",
            line(83_000.0),
            stroke_only(CanvasLineCap::Round),
            true,
        ),
        (
            "square",
            line(83_000.0),
            stroke_only(CanvasLineCap::Square),
            false,
        ),
        (
            "unpainted move",
            triangle,
            CanvasPaint {
                fill: Some(Color::WHITE),
                stroke: None,
                ..default()
            },
            true,
        ),
        (
            "move with a round stroke",
            vec![far_move],
            stroke_only(CanvasLineCap::Round),
            true,
        ),
        (
            "closed point with a round stroke",
            vec![far_move, Close],
            stroke_only(CanvasLineCap::Round),
            false,
        ),
        (
            "closed point with a square stroke",
            vec![far_move, Close],
            stroke_only(CanvasLineCap::Square),
            false,
        ),
        (
            "closed point alongside a curve",
            [early_curve, [far_move, Close]].concat(),
            stroke_only(CanvasLineCap::Round),
            false,
        ),
        (
            "arc with zero radius",
            collapsed_arc.to_vec(),
            stroke_only(CanvasLineCap::Butt),
            true,
        ),
        (
            "arc with zero radius alongside a curve",
            [early_curve, collapsed_arc].concat(),
            stroke_only(CanvasLineCap::Butt),
            true,
        ),
    ] {
        let mut world = World::new();
        let doc = world.spawn(CanvasDocument).id();
        let shape = CanvasDocument::create_path(
            &mut world,
            doc,
            CanvasPath {
                commands: line(5.0),
            },
            Transform::IDENTITY,
            paint,
        )
        .unwrap();
        let other = rectangle(&mut world, doc, Vec2::ZERO);
        let group = CanvasDocument::group_items(&mut world, doc, &[shape, other], "Group").unwrap();
        CanvasDocument::update_path(
            &mut world,
            doc,
            shape,
            CanvasPath { commands },
            transform,
            paint,
        )
        .unwrap();
        CanvasDocument::update_group_transform(&mut world, doc, group, transform).unwrap();
        let logs = LogCapture::default();
        let result = logs.run(|| CanvasDocument::ungroup(&mut world, doc, group));
        if result.is_ok() != expand {
            wrong_results.push((name, expand, result.is_ok()));
        }
        if let Ok(members) = result {
            assert_eq!(members, [shape, other]);
        } else {
            assert_eq!(
                CanvasDocument::container_of(&world, doc, shape).unwrap(),
                CanvasContainer::Group(group)
            );
        }
    }
    assert!(wrong_results.is_empty(), "{wrong_results:?}");
}

#[test]
fn grouped_path_append_validates_the_document_projection_before_writing() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let path = CanvasPath {
        commands: vec![
            CanvasPathCommand::MoveTo(Vec2::ZERO),
            CanvasPathCommand::LineTo(Vec2::X),
        ],
    };
    let shape = CanvasDocument::create_path(
        &mut world,
        doc,
        path,
        Transform::IDENTITY,
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke::default()),
            ..default()
        },
    )
    .unwrap();
    let other = rectangle(&mut world, doc, Vec2::new(15.0, 0.0));
    let group = CanvasDocument::group_items(&mut world, doc, &[shape, other], "Group").unwrap();
    CanvasDocument::update_group_transform(
        &mut world,
        doc,
        group,
        Transform::from_scale(Vec3::new(2.0, 2.0, 1.0)),
    )
    .unwrap();
    let geometry = world.get::<CanvasGeometry>(shape).unwrap().clone();
    let logs = LogCapture::default();

    assert!(
        logs.run(|| CanvasDocument::append_line(
            &mut world,
            doc,
            shape,
            Vec2::ZERO,
            Vec2::new(2.0e38, 0.0),
        ))
        .is_err()
    );
    assert_eq!(world.get::<CanvasGeometry>(shape), Some(&geometry));
    CanvasDocument::append_line(&mut world, doc, shape, Vec2::ZERO, Vec2::splat(5.0)).unwrap();
    let directory = tempfile::tempdir().unwrap();
    CanvasDocument::export_svg(&mut world, doc, directory.path().join("append.svg"), None).unwrap();
}

#[test]
fn grouped_shape_updates_reject_locally_valid_transforms_that_overflow_in_the_document() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let a = CanvasDocument::create_rectangle(
        &mut world,
        doc,
        CanvasRectangle {
            size: Vec2::ONE,
            corner_radius: 0.0,
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let b = rectangle(&mut world, doc, Vec2::splat(10.0));
    let group = CanvasDocument::group_items(&mut world, doc, &[a, b], "Group").unwrap();
    let transform = world
        .get::<Transform>(group)
        .unwrap()
        .with_scale(Vec3::new(2.0, 2.0, 1.0));
    CanvasDocument::update_group_transform(&mut world, doc, group, transform).unwrap();
    let old = *world.get::<Transform>(a).unwrap();
    let oversized = old.with_scale(Vec3::new(2.0e38, 1.0, 1.0));
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::update_transform(&mut world, doc, a, oversized))
            .is_err()
    );
    assert!(
        logs.run(|| CanvasDocument::update_transforms(&mut world, doc, &[(a, oversized)]))
            .is_err()
    );
    assert!(
        logs.run(|| CanvasDocument::update_rectangle(
            &mut world,
            doc,
            a,
            CanvasRectangle {
                size: Vec2::ONE,
                corner_radius: 0.0
            },
            oversized,
            CanvasPaint::default(),
        ))
        .is_err()
    );
    assert_eq!(*world.get::<Transform>(a).unwrap(), old);
    let svg = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(&mut world, doc, svg.path(), None).unwrap();
}

#[test]
fn ungrouping_then_selecting_an_expanded_member_preserves_that_selection() {
    let (mut app, view, _, a, _, _) = fixture(pointer_ids()[0], 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    CanvasDocument::ungroup(app.world_mut(), doc, group).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, [a]).unwrap();
    update(&mut app);
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [a]);
}

#[test]
fn deleting_last_group_members_removes_the_empty_groups_and_frees_the_layer() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let source = CanvasDocument::default_layer(&world, doc).unwrap();
    let a = rectangle(&mut world, doc, Vec2::ZERO);
    let b = rectangle(&mut world, doc, Vec2::splat(30.0));
    let c = rectangle(&mut world, doc, Vec2::splat(65.0));
    let inner = CanvasDocument::group_items(&mut world, doc, &[a, b], "Inner").unwrap();
    let outer = CanvasDocument::group_items(&mut world, doc, &[inner, c], "Outer").unwrap();
    CanvasDocument::create_layer(&mut world, doc, 1, "Other").unwrap();
    CanvasDocument::remove_shapes(&mut world, doc, &[a, b]).unwrap();
    assert!(world.get_entity(inner).is_ok() && world.get::<ChildOf>(inner).is_none());
    assert_eq!(
        CanvasDocument::item_order(&world, doc, CanvasContainer::Group(outer)).unwrap(),
        [c]
    );
    CanvasDocument::remove_shape(&mut world, doc, c).unwrap();
    assert!(world.get_entity(outer).is_ok() && world.get::<ChildOf>(outer).is_none());
    CanvasDocument::remove_layer(&mut world, doc, source, CanvasLayerRemoval::OnlyIfEmpty).unwrap();
}

#[test]
fn switching_the_active_layer_exits_another_layers_group_and_redirects_drawing() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, mut at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(35.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    let target = CanvasDocument::create_layer(app.world_mut(), doc, 1, "Target").unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    CanvasView::set_active_layer_in_world(app.world_mut(), view, target).unwrap();
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );
    CanvasView::set_mode_in_world(
        app.world_mut(),
        view,
        CanvasMode::Rectangle(CanvasPaint::default()),
    )
    .unwrap();
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, Vec2::splat(20.0));
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let drawn = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    assert_eq!(
        CanvasDocument::container_of(app.world(), doc, drawn).unwrap(),
        CanvasContainer::Layer(target)
    );
}

#[test]
fn resizing_a_child_stops_when_a_parent_transform_changes() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, base) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(40.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, [a]).unwrap();
    update(&mut app);
    let size = app
        .world()
        .get::<CanvasGeometry>(a)
        .unwrap()
        .rectangle()
        .unwrap()
        .size;
    let mut at = at_document(&app, view, &base, size * 0.5);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!(
        app.world()
            .resource::<bevy::picking::hover::PointerCaptureMap>()
            .is_captured(&pointer)
    );
    let old = *app.world().get::<Transform>(a).unwrap();
    let transform = app
        .world()
        .get::<Transform>(group)
        .unwrap()
        .with_scale(Vec3::new(2.0, 2.0, 1.0));
    app.world_mut().entity_mut(group).insert(transform);
    moved(&mut app, pointer, &mut at, Vec2::splat(15.0));
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(*app.world().get::<Transform>(a).unwrap(), old);
}

#[test]
fn a_group_drag_is_not_the_second_click_of_a_double_click() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, mut at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    update(&mut app);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let old = *app.world().get::<Transform>(group).unwrap();
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, Vec2::ONE);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_ne!(*app.world().get::<Transform>(group).unwrap(), old);
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn two_clicks_on_the_same_group_body_enter_that_group() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    update(&mut app);

    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    for _ in 0..2 {
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
    }
    assert_eq!(CanvasView::group_path(app.world(), view).unwrap(), [group]);
}

#[test]
fn small_group_can_enter_when_its_body_overlaps_scale_handle_hit_areas() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::ZERO);
    for item in [a, b] {
        CanvasDocument::update_rectangle(
            app.world_mut(),
            doc,
            item,
            CanvasRectangle {
                size: Vec2::splat(4.0),
                corner_radius: 0.0,
            },
            Transform::IDENTITY,
            CanvasPaint::default(),
        )
        .unwrap();
    }
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Small").unwrap();
    update(&mut app);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    for _ in 0..2 {
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
    }
    assert_eq!(CanvasView::group_path(app.world(), view).unwrap(), [group]);
}

#[test]
fn double_clicking_a_group_in_a_multiple_selection_enters_it() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let extra = rectangle(app.world_mut(), doc, Vec2::new(70.0, -40.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, [group, extra]).unwrap();
    update(&mut app);

    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [group]);
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );

    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(CanvasView::group_path(app.world(), view).unwrap(), [group]);
}

#[test]
fn clicking_another_view_breaks_the_group_double_click() {
    let pointer = pointer_ids()[0];
    let (mut app, view, second, a, _, at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    update(&mut app);
    let away = at_document(&app, second, &at, Vec2::ZERO);
    let mut previous = at.position - Vec2::ONE;
    for location in [&at, &away, &at] {
        input(
            &mut app,
            pointer,
            location,
            PointerAction::Move {
                delta: location.position - previous,
            },
        );
        input(
            &mut app,
            pointer,
            location,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            location,
            PointerAction::Release(PointerButton::Primary),
        );
        previous = location.position;
    }
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [group]);
    assert_eq!(CanvasView::selection(app.world(), second).unwrap(), [group]);
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );

    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(CanvasView::group_path(app.world(), view).unwrap(), [group]);
}

#[test]
fn group_double_click_can_pass_through_a_nonblocking_picked_child() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    let child = app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            ZIndex(1),
            Pickable {
                is_hoverable: true,
                should_block_lower: false,
            },
            ChildOf(view),
        ))
        .id();
    let child_presses = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = child_presses.clone();
    app.add_observer(move |press: On<PointerPress>| {
        if press.original_event_target() == child && press.entity == child {
            observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    });
    update(&mut app);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    for _ in 0..2 {
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
    }

    assert_eq!(child_presses.load(std::sync::atomic::Ordering::Relaxed), 2);
    assert_eq!(CanvasView::group_path(app.world(), view).unwrap(), [group]);
}

#[test]
fn clicking_unpicked_window_space_breaks_the_group_double_click() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, at) = fixture(pointer, 1.0, 1.0);
    app.world_mut()
        .resource_mut::<bevy::picking::PickingSettings>()
        .is_window_picking_enabled = false;
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    update(&mut app);
    let mut blank = at.clone();
    blank.position = Vec2::new(800.0, 550.0);
    let mut previous = at.position - Vec2::ONE;
    for location in [&at, &blank, &at] {
        input(
            &mut app,
            pointer,
            location,
            PointerAction::Move {
                delta: location.position - previous,
            },
        );
        if location == &blank {
            let hover = app.world().resource::<bevy::picking::hover::HoverMap>();
            assert!(
                hover.get(&pointer).is_none_or(|hits| hits.is_empty()),
                "{:?}",
                hover.get(&pointer)
            );
        }
        input(
            &mut app,
            pointer,
            location,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            location,
            PointerAction::Release(PointerButton::Primary),
        );
        previous = location.position;
    }
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [group]);
}

#[test]
fn escape_leaves_the_group_only_while_the_focused_view_accepts_input() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, window, _) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, [a]).unwrap();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(view, FocusCause::Navigated);
    app.world_mut()
        .entity_mut(view)
        .insert(bevy::ui::InteractionDisabled);
    let escape = |app: &mut App, window, state| {
        queue_key(
            app,
            KeyboardInput {
                key_code: KeyCode::Escape,
                logical_key: Key::Escape,
                state,
                text: None,
                repeat: false,
                window,
            },
        );
        update(app);
    };
    escape(&mut app, window, ButtonState::Pressed);
    assert_eq!(CanvasView::group_path(app.world(), view).unwrap(), [group]);
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [a]);

    escape(&mut app, window, ButtonState::Released);
    app.world_mut()
        .entity_mut(view)
        .remove::<bevy::ui::InteractionDisabled>();
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    let other_window = app
        .world_mut()
        .spawn(Window {
            focused: true,
            ..default()
        })
        .id();
    escape(&mut app, other_window, ButtonState::Pressed);
    assert_eq!(CanvasView::group_path(app.world(), view).unwrap(), [group]);
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [a]);

    escape(&mut app, other_window, ButtonState::Released);
    escape(&mut app, window, ButtonState::Pressed);
    assert_eq!(CanvasView::group_path(app.world(), view).unwrap(), [group]);
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [a]);

    escape(&mut app, window, ButtonState::Released);
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    app.world_mut()
        .get_mut::<Window>(other_window)
        .unwrap()
        .focused = false;
    escape(&mut app, window, ButtonState::Pressed);
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn group_edge_handle_scales_both_axes_and_anchors_the_opposite_edge() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::new(50.0, 0.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, [group]).unwrap();
    let old = *app.world().get::<Transform>(group).unwrap();
    let anchor = Vec2::new(-15.0, 0.0);
    let local_anchor = anchor - old.translation.truncate();
    update(&mut app);

    let mut handle = at_document(&app, view, &at, Vec2::new(56.0, 0.0));
    input(
        &mut app,
        pointer,
        &handle,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &handle,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut handle, Vec2::new(17.75, 0.0));
    input(
        &mut app,
        pointer,
        &handle,
        PointerAction::Release(PointerButton::Primary),
    );

    let edited = *app.world().get::<Transform>(group).unwrap();
    near(edited.scale.truncate(), Vec2::splat(1.25));
    near(
        edited.transform_point(local_anchor.extend(0.0)).truncate(),
        anchor,
    );
    assert_eq!(edited.scale.z, 1.0);
}

#[test]
fn an_uncaptured_first_press_movement_does_not_seed_a_group_double_click() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, mut at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    update(&mut app);

    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, Vec2::ONE);
    moved(&mut app, pointer, &mut at, -Vec2::ONE);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [group]);
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert!(
        CanvasView::group_path(app.world(), view)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn group_drawing_rejects_a_local_coordinate_that_displaces_the_preview() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, base) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    CanvasDocument::update_group_transform(
        app.world_mut(),
        doc,
        group,
        Transform::from_xyz(1.0e9, 0.0, 0.0),
    )
    .unwrap();
    CanvasView::set_mode_in_world(
        app.world_mut(),
        view,
        CanvasMode::Rectangle(CanvasPaint::default()),
    )
    .unwrap();
    let mut at = at_document(&app, view, &base, Vec2::new(3.0, -5.0));
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, Vec2::new(14.0, 10.0));
    let errors = ErrorCapture::default();
    queue_pointer(
        &mut app,
        pointer,
        at,
        PointerAction::Release(PointerButton::Primary),
    );
    errors.run(|| app.update());
    assert_eq!(
        CanvasDocument::item_order(app.world(), doc, CanvasContainer::Group(group)).unwrap(),
        [a, b]
    );
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let domain = records[0]
        .managed()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap()
        .source()
        .unwrap()
        .downcast_ref::<CanvasError>()
        .unwrap();
    assert_eq!(
        domain.kind,
        CanvasErrorKind::Invalid(CanvasIssue::GroupTransform)
    );
}

#[test]
fn group_pen_commit_rejects_a_displaced_preview_contour() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, base) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::splat(30.0));
    let group = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Group").unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    CanvasDocument::update_group_transform(
        app.world_mut(),
        doc,
        group,
        Transform::from_scale(Vec3::new(41.0, 41.0, 1.0)),
    )
    .unwrap();
    let center = 2.0_f32.powi(24) * Vec2::X;
    CanvasView::set_center_in_world(app.world_mut(), view, center).unwrap();
    CanvasView::set_mode_in_world(
        app.world_mut(),
        view,
        CanvasMode::Pen(CanvasPaint::default()),
    )
    .unwrap();
    for point in [center, center + 20.0 * Vec2::X, center + 20.0 * Vec2::Y] {
        let at = at_document(&app, view, &base, point);
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Move { delta: Vec2::ONE },
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
    }

    let logs = LogCapture::default();
    let result = logs.run(|| CanvasView::finish_pen_in_world(app.world_mut(), view));

    assert!(result.is_err(), "Pen committed {result:?}");
    assert_eq!(
        CanvasDocument::item_order(app.world(), doc, CanvasContainer::Group(group)).unwrap(),
        [a, b]
    );
}

#[test]
fn deep_parent_projection_matches_group_drawing_and_expansion() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, a, _, base) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let b = rectangle(app.world_mut(), doc, Vec2::new(32.0, 0.0));
    let c = rectangle(app.world_mut(), doc, Vec2::new(64.0, 0.0));
    let d = rectangle(app.world_mut(), doc, Vec2::new(96.0, 0.0));
    let inner = CanvasDocument::group_items(app.world_mut(), doc, &[a, b], "Inner").unwrap();
    let middle = CanvasDocument::group_items(app.world_mut(), doc, &[inner, c], "Middle").unwrap();
    let outer = CanvasDocument::group_items(app.world_mut(), doc, &[middle, d], "Outer").unwrap();
    for (group, x) in [(outer, 1.0e9), (middle, -1.0e9), (inner, 16.0)] {
        CanvasDocument::update_group_transform(
            app.world_mut(),
            doc,
            group,
            Transform::from_xyz(x, 0.0, 0.0),
        )
        .unwrap();
        CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    }
    let local = *app.world().get::<Transform>(a).unwrap();
    let parent = CanvasDocument::document_transform(app.world(), doc, inner).unwrap();
    assert_eq!(parent.translation.x, 16.0);
    assert_eq!(
        CanvasDocument::document_transform(app.world(), doc, a).unwrap(),
        parent * local.compute_affine(),
    );
    CanvasView::set_mode_in_world(
        app.world_mut(),
        view,
        CanvasMode::Rectangle(CanvasPaint::default()),
    )
    .unwrap();
    let seed = Vec2::new(8.0, 24.0);
    let delta = Vec2::splat(16.0);
    let mut at = at_document(&app, view, &base, seed);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, delta);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let drawn = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    let intended = seed + delta * 0.5;
    near(
        CanvasDocument::document_transform(app.world(), doc, drawn)
            .unwrap()
            .translation
            .truncate(),
        intended,
    );
    CanvasDocument::ungroup(app.world_mut(), doc, inner).unwrap();
    near(
        CanvasDocument::document_transform(app.world(), doc, drawn)
            .unwrap()
            .translation
            .truncate(),
        intended,
    );
}

#[test]
fn pointer_selects_a_group_and_programmatic_entry_exposes_children_in_one_view() {
    let pointer = pointer_ids()[0];
    let (mut app, primary, secondary, a, _, mut at) = fixture(pointer, 1.0, 1.0);
    let world = app.world_mut();
    let doc = CanvasView::document(world, primary).unwrap().unwrap();
    let b = rectangle(world, doc, Vec2::splat(35.0));
    let group = CanvasDocument::group_items(world, doc, &[a, b], "Root").unwrap();
    update(&mut app);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        CanvasView::single_selection(app.world(), primary).unwrap(),
        Some(group)
    );
    CanvasView::enter_group_in_world(app.world_mut(), primary, group).unwrap();
    assert_eq!(
        CanvasView::group_path(app.world(), primary).unwrap(),
        [group]
    );
    assert!(
        CanvasView::group_path(app.world(), secondary)
            .unwrap()
            .is_empty()
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert_eq!(
        CanvasView::single_selection(app.world(), primary).unwrap(),
        Some(a)
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let before = CanvasDocument::document_transform(app.world(), doc, a)
        .unwrap()
        .translation
        .truncate();
    let shift = Vec2::new(10.0, 4.0);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, shift);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    near(
        CanvasDocument::document_transform(app.world(), doc, a)
            .unwrap()
            .translation
            .truncate(),
        before + shift,
    );
    scroll(&mut app, pointer, &at, 1.0);
    assert!(
        CanvasView::zoom(app.world(), primary).unwrap()
            > CanvasView::zoom(app.world(), secondary).unwrap()
    );
}

#[test]
fn nested_view_proxies_and_svg_use_the_same_document_projection() {
    let (mut app, primary, _, a, _, _) = fixture(pointer_ids()[0], 1.0, 1.0);
    let world = app.world_mut();
    let document = CanvasView::document(world, primary).unwrap().unwrap();
    let b = rectangle(world, document, Vec2::new(35.0, 10.0));
    let c = rectangle(world, document, Vec2::new(20.0, -40.0));
    let inner = CanvasDocument::group_items(world, document, &[a, b], "Inner").unwrap();
    let outer = CanvasDocument::group_items(world, document, &[inner, c], "Outer").unwrap();
    let transform = Transform::from_xyz(10.0, 14.0, 5.0)
        .with_rotation(Quat::from_rotation_z(0.4))
        .with_scale(Vec3::new(1.5, 1.5, 1.0));
    CanvasDocument::update_group_transform(world, document, outer, transform).unwrap();
    let expected =
        [a, b, c].map(|shape| CanvasDocument::document_transform(world, document, shape).unwrap());
    update(&mut app);

    let world = app.world_mut();
    let mut groups = world.query::<(&Name, &ChildOf)>();
    let parents = groups
        .iter(world)
        .filter(|(name, _)| name.as_str() == "CanvasGroupProxy")
        .map(|(_, parent)| parent.parent())
        .collect::<Vec<_>>();
    assert_eq!(parents.len(), 4);
    assert_eq!(
        parents
            .iter()
            .filter(|&&parent| world.get::<Name>(parent).unwrap().as_str() == "CanvasContent")
            .count(),
        2
    );
    let mut shapes = world.query::<(&Name, &ChildOf, &GlobalTransform)>();
    let positions = shapes
        .iter(world)
        .filter(|(name, _, _)| name.as_str() == "CanvasShapeProxy")
        .map(|(_, parent, transform)| {
            assert_eq!(
                world.get::<Name>(parent.parent()).unwrap().as_str(),
                "CanvasGroupProxy"
            );
            let point = transform.translation().truncate();
            Vec2::new(point.x, -point.y)
        })
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 6);
    for projected in expected {
        assert_eq!(
            positions
                .iter()
                .filter(|point| point.abs_diff_eq(projected.translation.truncate(), TOLERANCE))
                .count(),
            2
        );
    }

    let file = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(world, document, file.path(), None).unwrap();
    let svg = std::fs::read_to_string(file.path()).unwrap();
    for (matrix, projected) in svg.split("matrix(").skip(1).zip(expected) {
        let values = matrix
            .split_once(')')
            .unwrap()
            .0
            .split_whitespace()
            .map(|value| value.parse::<f32>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 6);
        near(
            Vec2::new(values[4], values[5]),
            projected.translation.truncate(),
        );
        near(
            Vec2::new(values[0], values[1]),
            projected.matrix3.x_axis.truncate(),
        );
    }
    assert_eq!(svg.matches("matrix(").count(), expected.len());

    world.despawn(primary);
    update(&mut app);
    let world = app.world_mut();
    assert_eq!(
        groups
            .iter(world)
            .filter(|(name, _)| name.as_str() == "CanvasGroupProxy")
            .count(),
        2
    );
    assert_eq!(
        CanvasDocument::shape_order(world, document).unwrap(),
        [a, b, c]
    );
}

#[test]
#[allow(clippy::panic)]
fn node_drag_and_drawing_convert_between_group_and_document_coordinates() {
    let pointer = pointer_ids()[0];
    let (mut app, view, _, rectangle, _, base) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let local_endpoint = Vec2::new(30.0, 0.0);
    let path = CanvasDocument::create_path(
        app.world_mut(),
        doc,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::ZERO),
                CanvasPathCommand::LineTo(local_endpoint),
            ],
        },
        Transform::default(),
        CanvasPaint {
            stroke: Some(CanvasStroke::default()),
            ..default()
        },
    )
    .unwrap();
    let group =
        CanvasDocument::group_items(app.world_mut(), doc, &[rectangle, path], "Group").unwrap();
    let transform = Transform::from_xyz(10.0, 20.0, 0.0)
        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::new(1.5, 1.5, 1.0));
    CanvasDocument::update_group_transform(app.world_mut(), doc, group, transform).unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, [path]).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::PathNodeEdit).unwrap();
    update(&mut app);
    let projection = CanvasDocument::document_transform(app.world(), doc, path).unwrap();
    let local_transform = *app.world().get::<Transform>(path).unwrap();
    let endpoint = projection
        .transform_point3(local_endpoint.extend(0.0))
        .truncate();
    let local_shift = Vec2::new(8.0, -2.0);
    let world_shift = projection
        .transform_point3((local_endpoint + local_shift).extend(0.0))
        .truncate()
        - endpoint;
    let mut at = at_document(&app, view, &base, endpoint);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, world_shift * 0.5);
    moved(&mut app, pointer, &mut at, world_shift * 0.5);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let Some(CanvasGeometry::Path(path_data)) = app.world().get::<CanvasGeometry>(path) else {
        panic!("path entity does not have its original geometry kind");
    };
    let CanvasPathCommand::LineTo(actual) = path_data.commands[1] else {
        panic!("edited endpoint has the wrong command")
    };
    near(actual, local_endpoint + local_shift);
    assert_eq!(
        *app.world().get::<Transform>(path).unwrap(),
        local_transform
    );

    CanvasView::set_mode_in_world(
        app.world_mut(),
        view,
        CanvasMode::Rectangle(CanvasPaint::default()),
    )
    .unwrap();
    let start = Vec2::new(-30.0, 0.0);
    let size = Vec2::new(16.0, 10.0);
    let mut at = at_document(&app, view, &base, start);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, size);
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let drawn = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    assert_eq!(app.world().get::<ChildOf>(drawn).unwrap().parent(), group);
    let projected = CanvasDocument::document_transform(app.world(), doc, drawn).unwrap();
    near(projected.translation.truncate(), start + size * 0.5);
    near(projected.matrix3.x_axis.truncate(), Vec2::X);
    near(projected.matrix3.y_axis.truncate(), Vec2::Y);
}
