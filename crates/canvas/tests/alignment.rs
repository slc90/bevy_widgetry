#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

#[allow(dead_code)]
mod common;

use bevy::{picking::pointer::PointerId, prelude::*};
use bevy_widgetry_canvas::*;
use bevy_widgetry_job::JobFailure;

const EPSILON: f32 = 0.001;

fn kind(error: JobFailure) -> CanvasErrorKind {
    error
        .diagnostics()
        .next()
        .unwrap()
        .source()
        .unwrap()
        .downcast_ref::<CanvasError>()
        .unwrap()
        .kind
}

fn rectangle(world: &mut World, document: Entity, size: Vec2, pose: Transform) -> Entity {
    CanvasDocument::create_rectangle(
        world,
        document,
        CanvasRectangle {
            size,
            corner_radius: 0.0,
        },
        pose,
        CanvasPaint::default(),
    )
    .unwrap()
}

fn position(world: &World, document: Entity, item: Entity) -> Vec2 {
    CanvasDocument::document_transform(world, document, item)
        .unwrap()
        .translation
        .truncate()
}

fn fixture() -> (World, Entity, [Entity; 3]) {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let a = rectangle(
        &mut world,
        document,
        Vec2::new(10.0, 14.0),
        Transform::from_xyz(-20.0, -30.0, 0.0),
    );
    let b = rectangle(
        &mut world,
        document,
        Vec2::new(12.0, 6.0),
        Transform::from_xyz(15.0, 14.0, 0.0)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
    );
    let c = rectangle(
        &mut world,
        document,
        Vec2::new(22.0, 10.0),
        Transform::from_xyz(80.0, 65.0, 0.0),
    );
    CanvasDocument::clear_history(&mut world, document).unwrap();
    (world, document, [a, b, c])
}

#[test]
fn aligns_each_edge_and_center_from_document_bounds_with_one_history_entry() {
    for (alignment, expected, vertical) in [
        (CanvasAlignment::Left, [-20.0, -22.0, -14.0], false),
        (CanvasAlignment::HorizontalCenter, [33.0; 3], false),
        (CanvasAlignment::Right, [86.0, 88.0, 80.0], false),
        (CanvasAlignment::Top, [-30.0, -31.0, -32.0], true),
        (CanvasAlignment::VerticalCenter, [16.5; 3], true),
        (CanvasAlignment::Bottom, [63.0, 64.0, 65.0], true),
    ] {
        let (mut world, document, items) = fixture();
        let original = items.map(|item| *world.get::<Transform>(item).unwrap());
        assert!(
            CanvasDocument::align_items(
                &mut world,
                document,
                &items,
                alignment,
                CanvasAlignTarget::Selection
            )
            .unwrap()
        );
        for ((item, expected), original) in items.into_iter().zip(expected).zip(original) {
            let axis = usize::from(vertical);
            let updated = world.get::<Transform>(item).unwrap();
            assert!((updated.translation[axis] - expected).abs() < EPSILON);
            assert_eq!(
                updated.translation[1 - axis],
                original.translation[1 - axis]
            );
            assert_eq!(updated.rotation, original.rotation);
            assert_eq!(updated.scale, original.scale);
        }
        assert_eq!(CanvasDocument::draw_order(&world, document).unwrap(), items);
        assert!(CanvasDocument::undo(&mut world, document).unwrap());
        assert!(!CanvasDocument::can_undo(&world, document).unwrap());
        assert_eq!(
            items.map(|item| *world.get::<Transform>(item).unwrap()),
            original
        );
        assert!(CanvasDocument::redo(&mut world, document).unwrap());
    }
}

#[test]
fn distributes_unequal_bounds_on_both_axes_and_keeps_the_outer_items_in_place() {
    for (distribution, axis, expected) in [
        (CanvasDistribution::Horizontal, 0, 27.0),
        (CanvasDistribution::Vertical, 1, 18.5),
    ] {
        let (mut world, document, [left, middle, right]) = fixture();
        let left_position = position(&world, document, left);
        let right_position = position(&world, document, right);
        assert!(
            !CanvasDocument::distribute_items(&mut world, document, &[left, right], distribution)
                .unwrap()
        );
        assert!(!CanvasDocument::can_undo(&world, document).unwrap());
        assert!(
            CanvasDocument::distribute_items(
                &mut world,
                document,
                &[right, left, middle],
                distribution
            )
            .unwrap()
        );
        assert_eq!(position(&world, document, left), left_position);
        assert_eq!(position(&world, document, right), right_position);
        assert!((position(&world, document, middle)[axis] - expected).abs() < EPSILON);
        assert!(CanvasDocument::undo(&mut world, document).unwrap());
        assert!(!CanvasDocument::can_undo(&world, document).unwrap());
    }
}

#[test]
fn rejects_foreign_unavailable_or_connector_targets_without_moving_any_item() {
    let (mut world, document, [a, b, c]) = fixture();
    let foreign_document = world.spawn(CanvasDocument).id();
    let foreign = rectangle(&mut world, foreign_document, Vec2::ONE, Transform::IDENTITY);
    let connector = CanvasDocument::create_connector(
        &mut world,
        document,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            waypoints: vec![],
            end: Vec2::new(15.0, 20.0),
        },
        CanvasConnectorStyle::default(),
        None,
    )
    .unwrap();
    let locked = CanvasDocument::create_layer(&mut world, document, 1, "Locked").unwrap();
    let hidden = CanvasDocument::create_layer(&mut world, document, 2, "Hidden").unwrap();
    CanvasDocument::move_item_to_layer(&mut world, document, b, locked).unwrap();
    CanvasDocument::move_item_to_layer(&mut world, document, c, hidden).unwrap();
    CanvasDocument::set_layer_locked(&mut world, document, locked, true).unwrap();
    CanvasDocument::set_layer_visible(&mut world, document, hidden, false).unwrap();
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let original = position(&world, document, a);
    for invalid in [foreign, connector, b, c, Entity::PLACEHOLDER] {
        assert!(
            CanvasDocument::align_items(
                &mut world,
                document,
                &[a, invalid],
                CanvasAlignment::Right,
                CanvasAlignTarget::Selection
            )
            .is_err()
        );
        assert!(
            CanvasDocument::distribute_items(
                &mut world,
                document,
                &[a, invalid],
                CanvasDistribution::Horizontal
            )
            .is_err()
        );
        assert_eq!(position(&world, document, a), original);
        assert!(!CanvasDocument::can_undo(&world, document).unwrap());
    }
}

#[test]
fn aligns_nested_group_and_text_and_converts_child_motion_to_its_parent_space() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let [a, b, c] = [
        Vec2::new(0.0, -20.0),
        Vec2::new(20.0, 30.0),
        Vec2::new(-15.0, 10.0),
    ]
    .map(|at| {
        rectangle(
            &mut world,
            document,
            Vec2::splat(10.0),
            Transform::from_translation(at.extend(0.0)),
        )
    });
    let inner = CanvasDocument::group_items(&mut world, document, &[a, b], "Inner").unwrap();
    let outer = CanvasDocument::group_items(&mut world, document, &[inner, c], "Outer").unwrap();
    let mut group_transform = *world.get::<Transform>(outer).unwrap();
    group_transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    group_transform.scale = Vec3::new(2.0, 2.0, 1.0);
    CanvasDocument::update_group_transform(&mut world, document, outer, group_transform).unwrap();
    let text = CanvasDocument::create_text(
        &mut world,
        document,
        CanvasTextStyle {
            content: "Label".into(),
            ..default()
        },
        Transform::from_xyz(100.0, 6.0, 0.0),
    )
    .unwrap();
    let far = rectangle(
        &mut world,
        document,
        Vec2::splat(10.0),
        Transform::from_xyz(220.0, 60.0, 0.0),
    );
    assert!(
        CanvasDocument::align_items(
            &mut world,
            document,
            &[outer, text, far],
            CanvasAlignment::Right,
            CanvasAlignTarget::Selection,
        )
        .unwrap()
    );
    let shape_right = [a, b, c].map(|item| position(&world, document, item).x + 10.0);
    let label = world.get::<CanvasText>(text).unwrap().local_bounds();
    let label_position = position(&world, document, text);
    assert!((shape_right.into_iter().fold(f32::NEG_INFINITY, f32::max) - 225.0).abs() < EPSILON);
    assert!((label_position.x + label.max.x - 225.0).abs() < EPSILON);

    let outer_pose = *world.get::<Transform>(outer).unwrap();
    let child_before = position(&world, document, b);
    assert!(
        CanvasDocument::align_items(
            &mut world,
            document,
            &[b, far],
            CanvasAlignment::Right,
            CanvasAlignTarget::Selection,
        )
        .unwrap()
    );
    let child_after = position(&world, document, b);
    assert!((child_after.x - 215.0).abs() < EPSILON);
    assert!((child_after.y - child_before.y).abs() < EPSILON);
    assert_eq!(world.get::<Transform>(outer), Some(&outer_pose));
    assert!(
        CanvasDocument::align_items(
            &mut world,
            document,
            &[outer, b],
            CanvasAlignment::Left,
            CanvasAlignTarget::Selection,
        )
        .is_err()
    );
}

#[test]
fn an_unrepresentable_child_update_leaves_other_item_transforms_untouched() {
    let (mut world, document, [a, b, c]) = fixture();
    let group = CanvasDocument::group_items(&mut world, document, &[b, c], "Small").unwrap();
    let mut group_transform = *world.get::<Transform>(group).unwrap();
    group_transform.scale = Vec3::new(1e-20, 1e-20, 1.0);
    CanvasDocument::update_group_transform(&mut world, document, group, group_transform).unwrap();
    let far = rectangle(
        &mut world,
        document,
        Vec2::splat(10.0),
        Transform::from_xyz(1e25, 0.0, 0.0),
    );
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let items = [a, b, far];
    let original = items.map(|item| *world.get::<Transform>(item).unwrap());
    assert!(
        CanvasDocument::align_items(
            &mut world,
            document,
            &items,
            CanvasAlignment::Right,
            CanvasAlignTarget::Selection,
        )
        .is_err()
    );
    assert_eq!(
        items.map(|item| *world.get::<Transform>(item).unwrap()),
        original
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
}

#[test]
fn view_alignment_reads_each_current_layout_center_and_zoom() {
    let (mut app, main, auxiliary, item, _, _) = common::fixture(PointerId::Mouse, 1.25, 1.5);
    let world = app.world_mut();
    let document = CanvasView::document(world, main).unwrap().unwrap();
    for (view, center, zoom) in [
        (main, Vec2::new(-65.0, 50.0), 2.0),
        (auxiliary, Vec2::new(70.0, -80.0), 0.5),
    ] {
        CanvasView::set_center_in_world(world, view, center).unwrap();
        CanvasView::set_zoom_in_world(world, view, zoom).unwrap();
        let node = world.get::<ComputedNode>(view).unwrap();
        let half_width = node.size().x * node.inverse_scale_factor() * 0.5 / zoom;
        assert!(
            CanvasDocument::align_items(
                world,
                document,
                &[item],
                CanvasAlignment::Left,
                CanvasAlignTarget::View(view)
            )
            .unwrap()
        );
        assert!(
            (position(world, document, item).x - (center.x - half_width + 15.0)).abs() < EPSILON
        );
    }
}

#[test]
fn distribution_keeps_the_original_order_when_overlapping_bounds_cannot_be_spaced() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let [wide, near, last] = [(50.0, 100.0), (15.0, 10.0), (35.0, 10.0)].map(|(x, width)| {
        rectangle(
            &mut world,
            document,
            Vec2::new(width, 10.0),
            Transform::from_xyz(x, 0.0, 0.0),
        )
    });
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let items = [wide, near, last];
    assert!(
        !CanvasDocument::distribute_items(
            &mut world,
            document,
            &items,
            CanvasDistribution::Horizontal,
        )
        .unwrap()
    );
    assert_eq!(
        items.map(|item| position(&world, document, item).x),
        [50.0, 15.0, 35.0]
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
}

#[test]
fn noop_layouts_still_validate_the_edit_target() {
    let (mut world, document, [item, other, _]) = fixture();
    let token = CanvasDocument::edit(&mut world, document, |world, token| {
        let busy = CanvasDocument::align_items(
            world,
            document,
            &[item],
            CanvasAlignment::Left,
            CanvasAlignTarget::Selection,
        )
        .unwrap_err();
        assert_eq!(kind(busy), CanvasErrorKind::Busy);
        assert!(
            !CanvasDocument::align_items(
                world,
                token,
                &[item],
                CanvasAlignment::Left,
                CanvasAlignTarget::Selection,
            )
            .unwrap()
        );
        Ok(token)
    })
    .unwrap();
    let expired = CanvasDocument::distribute_items(
        &mut world,
        token,
        &[item, other],
        CanvasDistribution::Horizontal,
    )
    .unwrap_err();
    assert_eq!(
        kind(expired),
        CanvasErrorKind::Invalid(CanvasIssue::EditToken)
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
}

#[test]
fn a_group_rejects_an_alignment_when_a_fixed_anchor_changes_the_planned_edge() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = rectangle(&mut world, document, Vec2::splat(20.0), Transform::IDENTITY);
    let connector = CanvasDocument::create_connector(
        &mut world,
        document,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            waypoints: vec![],
            end: Vec2::new(150.0, 0.0),
        },
        CanvasConnectorStyle::default(),
        None,
    )
    .unwrap();
    let external = rectangle(
        &mut world,
        document,
        Vec2::splat(20.0),
        Transform::from_xyz(150.0, 0.0, 0.0),
    );
    let far = rectangle(
        &mut world,
        document,
        Vec2::splat(10.0),
        Transform::from_xyz(400.0, 100.0, 0.0),
    );
    CanvasDocument::set_connector_endpoint(
        &mut world,
        document,
        connector,
        CanvasConnectorEnd::End,
        CanvasConnectorEndpoint::Attached {
            target: external,
            anchor: CanvasAnchor::West,
        },
    )
    .unwrap();
    let group =
        CanvasDocument::group_items(&mut world, document, &[shape, connector], "Group").unwrap();
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let original = *world.get::<Transform>(group).unwrap();
    let route = world
        .get::<CanvasConnector>(connector)
        .unwrap()
        .route()
        .clone();
    assert!(
        CanvasDocument::align_items(
            &mut world,
            document,
            &[group, far],
            CanvasAlignment::Right,
            CanvasAlignTarget::Selection,
        )
        .is_err()
    );
    assert_eq!(world.get::<Transform>(group), Some(&original));
    assert_eq!(
        world.get::<CanvasConnector>(connector).unwrap().route(),
        &route
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
    assert!(
        CanvasDocument::align_items(
            &mut world,
            document,
            &[group, far],
            CanvasAlignment::Left,
            CanvasAlignTarget::Selection,
        )
        .unwrap()
    );
    assert_eq!(world.get::<Transform>(group), Some(&original));
}

#[test]
fn distribution_rejects_a_group_whose_following_route_changes_its_cross_axis_size() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let panel = rectangle(
        &mut world,
        document,
        Vec2::new(400.0, 10.0),
        Transform::IDENTITY,
    );
    let route = CanvasDocument::create_connector(
        &mut world,
        document,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            waypoints: vec![],
            end: Vec2::new(0.0, 100.0),
        },
        CanvasConnectorStyle {
            stroke: CanvasStroke {
                width: 20.0,
                cap: CanvasLineCap::Butt,
                ..default()
            },
            ..default()
        },
        None,
    )
    .unwrap();
    let anchor = rectangle(
        &mut world,
        document,
        Vec2::splat(20.0),
        Transform::from_xyz(10.0, 100.0, 0.0),
    );
    CanvasDocument::set_connector_endpoint(
        &mut world,
        document,
        route,
        CanvasConnectorEnd::End,
        CanvasConnectorEndpoint::Attached {
            target: anchor,
            anchor: CanvasAnchor::West,
        },
    )
    .unwrap();
    let group =
        CanvasDocument::group_items(&mut world, document, &[panel, route], "Panel").unwrap();
    let first = rectangle(
        &mut world,
        document,
        Vec2::splat(10.0),
        Transform::from_xyz(-405.0, 0.0, 0.0),
    );
    let last = rectangle(
        &mut world,
        document,
        Vec2::splat(10.0),
        Transform::from_xyz(605.0, 0.0, 0.0),
    );
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let group_pose = *world.get::<Transform>(group).unwrap();
    let distorted_bounds = CanvasDocument::distribute_items(
        &mut world,
        document,
        &[first, group, last],
        CanvasDistribution::Horizontal,
    )
    .unwrap_err();
    assert_eq!(
        kind(distorted_bounds),
        CanvasErrorKind::Invalid(CanvasIssue::AlignmentBounds)
    );
    assert_eq!(world.get::<Transform>(group), Some(&group_pose));
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
}
