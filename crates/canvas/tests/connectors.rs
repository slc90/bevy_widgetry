#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{picking::pointer::*, prelude::*};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key};

#[allow(dead_code)]
mod common;
#[path = "common/coordinates.rs"]
mod coordinates;

fn label() -> CanvasConnectorLabel {
    CanvasConnectorLabel {
        text: "A<&\nNext".to_owned(),
        font_size: 14.0,
        color: Color::srgb(0.5, 0.25, 0.0),
        position: 0.5,
    }
}

fn arrow_style(width: f32) -> CanvasConnectorStyle {
    CanvasConnectorStyle {
        stroke: CanvasStroke { width, ..default() },
        start_arrow: CanvasArrow::Triangle,
        end_arrow: CanvasArrow::Triangle,
    }
}

fn svg(world: &mut World, document: Entity) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connector.svg");
    CanvasDocument::export_svg(world, document, &path, None).unwrap();
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn database_cap_keeps_cardinal_anchors_on_the_closed_body() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let Some(CanvasGeometry::Path(path)) =
        CanvasFlowchartPreset::Database.geometry(Vec2::new(140.0, 70.0))
    else {
        panic!("database preset must produce a path");
    };
    let database = CanvasDocument::create_path(
        &mut world,
        document,
        path,
        Transform::from_xyz(90.0, 40.0, 0.0),
        CanvasPaint::default(),
    )
    .unwrap();
    for (anchor, expected) in [
        (CanvasAnchor::North, Vec2::new(90.0, 5.0)),
        (CanvasAnchor::East, Vec2::new(160.0, 40.0)),
        (CanvasAnchor::South, Vec2::new(90.0, 75.0)),
        (CanvasAnchor::West, Vec2::new(20.0, 40.0)),
    ] {
        let actual = CanvasDocument::shape_anchor(&world, document, database, anchor)
            .unwrap()
            .unwrap();
        assert!(actual.distance(expected) <= 0.01, "{anchor:?}: {actual:?}");
    }
}

#[test]
fn cardinal_anchors_project_parameter_geometry_and_closed_path_contours() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let ellipse = CanvasDocument::create_ellipse(
        &mut world,
        document,
        CanvasEllipse {
            radii: Vec2::new(20.0, 8.0),
        },
        Transform::from_xyz(100.0, -10.0, 0.0),
        CanvasPaint::default(),
    )
    .unwrap();
    let triangle = CanvasDocument::create_path(
        &mut world,
        document,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::new(-20.0, 10.0)),
                CanvasPathCommand::LineTo(Vec2::new(20.0, 10.0)),
                CanvasPathCommand::LineTo(Vec2::new(0.0, -10.0)),
                CanvasPathCommand::Close,
            ],
        },
        Transform::IDENTITY,
        CanvasPaint::default(),
    )
    .unwrap();
    for (anchor, ellipse_point, triangle_point) in [
        (
            CanvasAnchor::North,
            Vec2::new(100.0, -18.0),
            Vec2::new(0.0, -10.0),
        ),
        (
            CanvasAnchor::East,
            Vec2::new(120.0, -10.0),
            Vec2::new(10.0, 0.0),
        ),
        (
            CanvasAnchor::South,
            Vec2::new(100.0, -2.0),
            Vec2::new(0.0, 10.0),
        ),
        (
            CanvasAnchor::West,
            Vec2::new(80.0, -10.0),
            Vec2::new(-10.0, 0.0),
        ),
    ] {
        assert_eq!(
            CanvasDocument::shape_anchor(&world, document, ellipse, anchor).unwrap(),
            Some(ellipse_point)
        );
        assert_eq!(
            CanvasDocument::shape_anchor(&world, document, triangle, anchor).unwrap(),
            Some(triangle_point)
        );
    }
}

#[test]
fn attached_endpoints_move_with_the_shape_and_detach_in_the_deletion_entry() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let layer = CanvasDocument::create_layer(&mut world, document, 1, "Nodes").unwrap();
    let decoration = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(12.0),
            corner_radius: 0.0,
        },
        Transform::from_xyz(-50.0, 20.0, 0.0),
        CanvasPaint::default(),
    )
    .unwrap();
    let shape = CanvasDocument::create_rectangle_in_layer(
        &mut world,
        document,
        layer,
        CanvasRectangle {
            size: Vec2::new(20.0, 40.0),
            corner_radius: 3.0,
        },
        Transform::IDENTITY,
        CanvasPaint::default(),
    )
    .unwrap();
    let waypoint = Vec2::new(40.0, 22.0);
    let route = CanvasConnectorRoute {
        start: Vec2::X,
        end: Vec2::X * 90.0,
        waypoints: vec![waypoint],
    };
    let connector =
        CanvasDocument::create_connector(&mut world, document, route, default(), None).unwrap();
    let attached = CanvasConnectorEndpoint::Attached {
        target: shape,
        anchor: CanvasAnchor::East,
    };
    assert!(
        CanvasDocument::set_connector_endpoint(
            &mut world,
            document,
            connector,
            CanvasConnectorEnd::Start,
            attached
        )
        .unwrap()
    );
    let start = |world: &World| {
        world
            .get::<CanvasConnector>(connector)
            .unwrap()
            .route()
            .start
    };
    let endpoint = |world: &World| {
        CanvasDocument::connector_endpoint(world, document, connector, CanvasConnectorEnd::Start)
            .unwrap()
    };
    assert_eq!(start(&world), Vec2::X * 10.0);
    let moved = Transform::from_xyz(25.0, -10.0, 0.0)
        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    CanvasDocument::update_transform(&mut world, document, shape, moved).unwrap();
    assert!(start(&world).abs_diff_eq(Vec2::new(25.0, 0.0), 0.001));
    let wider = CanvasRectangle {
        size: Vec2::new(40.0, 40.0),
        corner_radius: 3.0,
    };
    CanvasDocument::update_rectangle(&mut world, document, shape, wider, moved, default()).unwrap();
    assert!(start(&world).abs_diff_eq(Vec2::new(25.0, 10.0), 0.001));
    assert_eq!(
        world
            .get::<CanvasConnector>(connector)
            .unwrap()
            .route()
            .waypoints,
        [waypoint]
    );
    let sibling = CanvasDocument::create_ellipse_in_layer(
        &mut world,
        document,
        layer,
        CanvasEllipse {
            radii: Vec2::splat(6.0),
        },
        Transform::from_xyz(50.0, 35.0, 0.0),
        default(),
    )
    .unwrap();
    let nodes =
        CanvasDocument::group_items(&mut world, document, &[shape, sibling], "Nodes").unwrap();
    let edges =
        CanvasDocument::group_items(&mut world, document, &[decoration, connector], "Edges")
            .unwrap();
    let pivot = Transform::from_xyz(12.0, 24.0, 0.0)
        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4))
        .with_scale(Vec3::new(1.5, 1.5, 1.0));
    CanvasDocument::update_group_transform(&mut world, document, nodes, pivot).unwrap();
    CanvasDocument::update_group_transform(
        &mut world,
        document,
        edges,
        Transform::from_xyz(5.0, 8.0, 0.0),
    )
    .unwrap();
    let local_anchor = Vec3::X * (wider.size.x * 0.5);
    let shape_pose = world.get::<Transform>(shape).unwrap();
    let expected = pivot
        .transform_point(shape_pose.transform_point(local_anchor))
        .truncate();
    let connection_pose = CanvasDocument::document_transform(&world, document, connector).unwrap();
    assert!(
        connection_pose
            .transform_point3(start(&world).extend(0.0))
            .truncate()
            .abs_diff_eq(expected, 0.001)
    );
    CanvasDocument::set_layer_visible(&mut world, document, layer, false).unwrap();
    assert!(!svg(&mut world, document).contains("<path "));
    CanvasDocument::set_layer_visible(&mut world, document, layer, true).unwrap();
    assert!(svg(&mut world, document).contains("<path "));
    let last = start(&world);
    let last_in_parent = world
        .get::<Transform>(connector)
        .unwrap()
        .transform_point(last.extend(0.0))
        .truncate();
    CanvasDocument::remove_shape(&mut world, document, shape).unwrap();
    assert_eq!(
        endpoint(&world),
        CanvasConnectorEndpoint::Free(last_in_parent)
    );
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(endpoint(&world), attached);
    assert_eq!(start(&world), last);
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        endpoint(&world),
        CanvasConnectorEndpoint::Free(last_in_parent)
    );
}

#[test]
fn unrepresentable_following_route_rejects_the_entire_transform_batch() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::splat(20.0),
            corner_radius: 0.0,
        },
        Transform::IDENTITY,
        default(),
    )
    .unwrap();
    let distance = 2e38;
    let connector = CanvasDocument::create_connector(
        &mut world,
        document,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            end: Vec2::new(-distance, 0.0),
            waypoints: Vec::new(),
        },
        default(),
        None,
    )
    .unwrap();
    CanvasDocument::set_connector_endpoint(
        &mut world,
        document,
        connector,
        CanvasConnectorEnd::Start,
        CanvasConnectorEndpoint::Attached {
            target: shape,
            anchor: CanvasAnchor::East,
        },
    )
    .unwrap();
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let route = world
        .get::<CanvasConnector>(connector)
        .unwrap()
        .route()
        .clone();
    assert!(
        CanvasDocument::update_transforms(
            &mut world,
            document,
            &[(shape, Transform::from_xyz(distance, 0.0, 0.0))]
        )
        .is_err()
    );
    assert_eq!(world.get::<Transform>(shape), Some(&Transform::IDENTITY));
    assert_eq!(
        world.get::<CanvasConnector>(connector).unwrap().route(),
        &route
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
}

#[test]
fn free_endpoint_setter_takes_the_parent_position_of_a_translated_connector() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let connector = CanvasDocument::create_connector(
        &mut world,
        document,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            end: Vec2::X * 60.0,
            waypoints: Vec::new(),
        },
        default(),
        None,
    )
    .unwrap();
    let pose = Transform::from_xyz(100.0, 0.0, 0.0);
    CanvasDocument::update_transforms(&mut world, document, &[(connector, pose)]).unwrap();
    let parent_point = Vec2::X * 20.0;
    CanvasDocument::set_connector_endpoint(
        &mut world,
        document,
        connector,
        CanvasConnectorEnd::Start,
        CanvasConnectorEndpoint::Free(parent_point),
    )
    .unwrap();
    let value = world.get::<CanvasConnector>(connector).unwrap();
    assert_eq!(
        pose.transform_point(value.route().start.extend(0.0))
            .truncate(),
        parent_point
    );
    assert_eq!(
        CanvasDocument::connector_endpoint(&world, document, connector, CanvasConnectorEnd::Start)
            .unwrap(),
        CanvasConnectorEndpoint::Free(parent_point)
    );
}

#[test]
fn straight_connector_has_its_own_geometry_and_one_creation_entry() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let route = CanvasConnectorRoute {
        start: Vec2::new(20.0, 30.0),
        waypoints: Vec::new(),
        end: Vec2::new(100.0, 30.0),
    };
    let item = CanvasDocument::create_connector(
        &mut world,
        document,
        route.clone(),
        CanvasConnectorStyle::default(),
        None,
    )
    .unwrap();
    assert_eq!(world.get::<CanvasConnector>(item).unwrap().route(), &route);
    assert!(world.get::<CanvasGeometry>(item).is_none());
    let markup = svg(&mut world, document);
    assert_eq!(markup.matches("<path ").count(), 1);
    assert!(markup.contains("M 20 30 L 100 30"));
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [item]
    );
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert!(
        CanvasDocument::draw_order(&world, document)
            .unwrap()
            .is_empty()
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [item]
    );
}

#[test]
fn svg_uses_endpoint_tangents_and_centers_the_label_at_the_route_distance() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let end = Vec2::new(100.0, 40.0);
    let route = CanvasConnectorRoute {
        start: Vec2::ZERO,
        waypoints: vec![Vec2::ZERO, Vec2::new(100.0, 0.0), end],
        end,
    };
    let style = arrow_style(5.0);
    let label = label();
    let item = CanvasDocument::create_connector(
        &mut world,
        document,
        route.clone(),
        style,
        Some(label.clone()),
    )
    .unwrap();
    let markup = svg(&mut world, document);
    assert!(markup.contains("M 0 0 L 0 0 L 100 0 L 100 40 L 100 40"));
    assert!(markup.contains("M 0 0 L 20 -8 L 20 8 Z"));
    assert!(markup.contains("M 100 40 L 92 20 L 108 20 Z"));
    assert!(markup.contains("<text transform=\"matrix(1 0 0 1 70 0)\""));
    assert!(markup.contains("@font-face") && markup.contains("A&lt;&amp;"));
    assert!(markup.contains("font-size=\"14\""));

    let end_only = CanvasConnectorStyle {
        start_arrow: CanvasArrow::None,
        ..style
    };
    CanvasDocument::update_connector(&mut world, document, item, route.clone(), end_only, None)
        .unwrap();
    let markup = svg(&mut world, document);
    assert!(!markup.contains("M 0 0 L 20 -8"));
    assert!(markup.contains("M 100 40 L 92 20 L 108 20 Z"));
    assert!(!markup.contains("<text "));
    let invalid = CanvasConnectorLabel {
        position: 1.5,
        ..label.clone()
    };
    assert!(
        CanvasDocument::update_connector(&mut world, document, item, route, style, Some(invalid),)
            .is_err()
    );
    assert_eq!(
        world.get::<CanvasConnector>(item).unwrap().style(),
        end_only
    );
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        world.get::<CanvasConnector>(item).unwrap().label(),
        Some(&label)
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert!(
        world
            .get::<CanvasConnector>(item)
            .unwrap()
            .label()
            .is_none()
    );
}

#[test]
fn connectors_keep_their_model_and_order_through_group_layer_and_history_operations() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let layer = CanvasDocument::create_layer(&mut world, doc, 1, "Flow").unwrap();
    let route = CanvasConnectorRoute {
        start: Vec2::ZERO,
        waypoints: vec![Vec2::X * 40.0],
        end: Vec2::new(40.0, 30.0),
    };
    let first = CanvasDocument::create_connector_in_layer(
        &mut world,
        doc,
        layer,
        route.clone(),
        arrow_style(3.0),
        Some(label()),
    )
    .unwrap();
    let second = CanvasDocument::duplicate_connector(&mut world, doc, first).unwrap();
    let group = CanvasDocument::group_items(&mut world, doc, &[first, second], "Route").unwrap();
    let copy = CanvasDocument::duplicate_connector(&mut world, doc, first).unwrap();
    assert_eq!(
        CanvasDocument::container_of(&world, doc, copy).unwrap(),
        CanvasContainer::Group(group)
    );
    assert_eq!(
        world.get::<CanvasConnector>(copy),
        world.get::<CanvasConnector>(first)
    );
    assert_eq!(world.get::<Transform>(copy), world.get::<Transform>(first));
    CanvasDocument::reorder_item(
        &mut world,
        doc,
        CanvasContainer::Group(group),
        copy,
        CanvasOrder::Back,
    )
    .unwrap();
    assert_eq!(
        CanvasDocument::draw_order(&world, doc).unwrap(),
        [group, copy, first, second]
    );
    let before = CanvasDocument::document_transform(&world, doc, copy).unwrap();
    let mut pose = *world.get::<Transform>(group).unwrap();
    let delta = Vec2::new(25.0, -20.0).extend(0.0);
    pose.translation += delta;
    CanvasDocument::update_group_transform(&mut world, doc, group, pose).unwrap();
    let after = CanvasDocument::document_transform(&world, doc, copy).unwrap();
    assert_eq!(
        after.translation.truncate(),
        before.translation.truncate() + delta.truncate()
    );
    assert_eq!(world.get::<CanvasConnector>(copy).unwrap().route(), &route);
    assert_eq!(svg(&mut world, doc).matches("<text ").count(), 3);
    CanvasDocument::remove_connector(&mut world, doc, copy).unwrap();
    assert_eq!(
        CanvasDocument::draw_order(&world, doc).unwrap(),
        [group, first, second]
    );
    assert!(CanvasDocument::undo(&mut world, doc).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, doc).unwrap(),
        [group, copy, first, second]
    );
    CanvasDocument::ungroup(&mut world, doc, group).unwrap();
    assert_eq!(
        CanvasDocument::document_transform(&world, doc, copy).unwrap(),
        after
    );
    let default_layer = CanvasDocument::default_layer(&world, doc).unwrap();
    CanvasDocument::move_item_to_layer(&mut world, doc, copy, default_layer).unwrap();
    assert_eq!(
        CanvasDocument::container_of(&world, doc, copy).unwrap(),
        CanvasContainer::Layer(default_layer)
    );
    assert!(
        CanvasDocument::shape_order(&mut world, doc)
            .unwrap()
            .is_empty()
    );
}

fn click_at(app: &mut App, view: Entity, base: &Location, point: Vec2) {
    let at = coordinates::at_document(app, view, base, point);
    for action in [
        PointerAction::Move { delta: Vec2::ONE },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        common::input(app, PointerId::Mouse, &at, action);
    }
}

#[test]
fn grouping_a_rotated_connector_uses_its_painted_projection() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let end = Vec2::splat(40.0);
    let stroke_width = 10.0;
    let connector = CanvasDocument::create_connector(
        &mut world,
        doc,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            waypoints: Vec::new(),
            end,
        },
        CanvasConnectorStyle {
            stroke: CanvasStroke {
                width: stroke_width,
                ..default()
            },
            ..default()
        },
        None,
    )
    .unwrap();
    CanvasDocument::update_transform(
        &mut world,
        doc,
        connector,
        Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
    )
    .unwrap();
    let rectangle_center = Vec2::splat(20.0);
    let rectangle = CanvasDocument::create_rectangle(
        &mut world,
        doc,
        CanvasRectangle {
            size: Vec2::splat(stroke_width),
            corner_radius: 0.0,
        },
        Transform::from_translation(rectangle_center.extend(0.0)),
        CanvasPaint::default(),
    )
    .unwrap();
    let group =
        CanvasDocument::group_items(&mut world, doc, &[connector, rectangle], "Flow").unwrap();
    let center = world
        .get::<Transform>(group)
        .unwrap()
        .translation
        .truncate();
    let expected = Vec2::new(rectangle_center.x * 0.5, end.length() * 0.5);
    assert!(
        center.abs_diff_eq(expected, 0.001),
        "{center:?} != {expected:?}"
    );
}

#[test]
fn repeated_free_endpoint_at_roundoff_is_a_no_op() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let connector = CanvasDocument::create_connector(
        &mut world,
        document,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            end: Vec2::X * 40.0,
            waypoints: vec![],
        },
        default(),
        None,
    )
    .unwrap();
    CanvasDocument::update_transforms(
        &mut world,
        document,
        &[(connector, Transform::from_xyz(100.0, 0.0, 0.0))],
    )
    .unwrap();
    let endpoint = CanvasConnectorEndpoint::Free(Vec2::X * 0.1);
    assert!(
        CanvasDocument::set_connector_endpoint(
            &mut world,
            document,
            connector,
            CanvasConnectorEnd::Start,
            endpoint
        )
        .unwrap()
    );
    CanvasDocument::clear_history(&mut world, document).unwrap();
    assert!(
        !CanvasDocument::set_connector_endpoint(
            &mut world,
            document,
            connector,
            CanvasConnectorEnd::Start,
            endpoint
        )
        .unwrap()
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
    let target = CanvasDocument::create_rectangle(
        &mut world,
        document,
        CanvasRectangle {
            size: Vec2::new(2.0, 10.0),
            corner_radius: 0.0,
        },
        Transform::IDENTITY.with_scale(Vec3::new(0.1, 1.0, 1.0)),
        default(),
    )
    .unwrap();
    CanvasDocument::set_connector_endpoint(
        &mut world,
        document,
        connector,
        CanvasConnectorEnd::End,
        CanvasConnectorEndpoint::Attached {
            target,
            anchor: CanvasAnchor::East,
        },
    )
    .unwrap();
    let route = world.get::<CanvasConnector>(connector).unwrap().route();
    assert_eq!(route.start, route.end);
    CanvasDocument::clear_history(&mut world, document).unwrap();
    assert!(
        !CanvasDocument::set_connector_endpoint(
            &mut world,
            document,
            connector,
            CanvasConnectorEnd::Start,
            endpoint
        )
        .unwrap()
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
}

fn drag(app: &mut App, view: Entity, base: &Location, from: Vec2, delta: Vec2) {
    let mut at = coordinates::at_document(app, view, base, from);
    common::input(
        app,
        PointerId::Mouse,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    common::input(
        app,
        PointerId::Mouse,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    common::moved(app, PointerId::Mouse, &mut at, delta);
    common::input(
        app,
        PointerId::Mouse,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
}

#[test]
fn connector_drawing_and_endpoint_drags_use_shape_anchors_without_binding_waypoints() {
    let (mut app, view, _, shape, window, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasView::set_object_snap_in_world(
        app.world_mut(),
        view,
        CanvasObjectSnap {
            connector_anchors: true,
            ..default()
        },
    )
    .unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Connector(default())).unwrap();
    let west = Vec2::new(-18.0, 1.0);
    let waypoint = Vec2::new(-3.0, -17.0);
    let free = Vec2::new(50.0, -20.0);
    for point in [west, waypoint, free] {
        click_at(&mut app, view, &base, point);
    }
    press_key(&mut app, window, KeyCode::Enter);
    let item = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    let endpoint = |world: &World, end| {
        CanvasDocument::connector_endpoint(world, document, item, end).unwrap()
    };
    let attached = |anchor| CanvasConnectorEndpoint::Attached {
        target: shape,
        anchor,
    };
    assert_eq!(
        endpoint(app.world(), CanvasConnectorEnd::Start),
        attached(CanvasAnchor::West)
    );
    assert_eq!(
        app.world()
            .get::<CanvasConnector>(item)
            .unwrap()
            .route()
            .waypoints,
        [waypoint]
    );
    assert_eq!(
        endpoint(app.world(), CanvasConnectorEnd::End),
        CanvasConnectorEndpoint::Free(free)
    );
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::PathNodeEdit).unwrap();
    common::update(&mut app);
    drag(&mut app, view, &base, free, Vec2::new(17.0, -1.0) - free);
    assert_eq!(
        endpoint(app.world(), CanvasConnectorEnd::End),
        attached(CanvasAnchor::East)
    );
    let east = Vec2::X * 15.0;
    drag(&mut app, view, &base, east, free - east);
    assert_eq!(
        endpoint(app.world(), CanvasConnectorEnd::End),
        CanvasConnectorEndpoint::Free(free)
    );
    assert!(CanvasDocument::undo(app.world_mut(), document).unwrap());
    assert_eq!(
        endpoint(app.world(), CanvasConnectorEnd::End),
        attached(CanvasAnchor::East)
    );
}

#[test]
fn drawing_keeps_distinct_shape_anchors_with_the_same_grid_candidate() {
    let (mut app, view, _, _, window, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    let shapes = [-6.0, 6.0].map(|y| {
        CanvasDocument::create_ellipse(
            app.world_mut(),
            document,
            CanvasEllipse {
                radii: Vec2::splat(2.0),
            },
            Transform::from_xyz(0.0, y, 0.0),
            default(),
        )
        .unwrap()
    });
    CanvasView::set_object_snap_in_world(
        app.world_mut(),
        view,
        CanvasObjectSnap {
            connector_anchors: true,
            ..default()
        },
    )
    .unwrap();
    CanvasView::set_grid_in_world(
        app.world_mut(),
        view,
        CanvasGrid {
            snap: CanvasGridSnap::ALL,
            ..default()
        },
    )
    .unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Connector(default())).unwrap();
    for y in [-4.0, 4.0] {
        click_at(&mut app, view, &base, Vec2::new(0.0, y));
    }
    press_key(&mut app, window, KeyCode::Enter);
    let item = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    let connector = app.world().get::<CanvasConnector>(item).unwrap();
    assert_eq!(connector.route().start, Vec2::new(0.0, -4.0));
    assert_eq!(connector.route().end, Vec2::new(0.0, 4.0));
    assert_eq!(
        CanvasDocument::connector_endpoint(app.world(), document, item, CanvasConnectorEnd::Start)
            .unwrap(),
        CanvasConnectorEndpoint::Attached {
            target: shapes[0],
            anchor: CanvasAnchor::South
        }
    );
    assert_eq!(
        CanvasDocument::connector_endpoint(app.world(), document, item, CanvasConnectorEnd::End)
            .unwrap(),
        CanvasConnectorEndpoint::Attached {
            target: shapes[1],
            anchor: CanvasAnchor::North
        }
    );
}

#[test]
fn connector_deduplicates_repeated_ordinary_grid_candidates() {
    let (mut app, view, _, _, window, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    CanvasView::set_grid_in_world(
        app.world_mut(),
        view,
        CanvasGrid {
            snap: CanvasGridSnap {
                connector_points: true,
                ..default()
            },
            ..default()
        },
    )
    .unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Connector(default())).unwrap();
    for point in [
        Vec2::new(-25.0, 25.0),
        Vec2::new(46.0, 0.0),
        Vec2::new(54.0, 0.0),
        Vec2::new(75.0, 25.0),
    ] {
        click_at(&mut app, view, &base, point);
    }
    press_key(&mut app, window, KeyCode::Enter);
    let item = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    assert_eq!(
        app.world()
            .get::<CanvasConnector>(item)
            .unwrap()
            .route()
            .waypoints,
        [Vec2::X * 50.0]
    );
}

#[test]
fn a_click_near_the_start_anchor_can_become_a_free_waypoint() {
    let (mut app, view, _, _, window, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasDocument::create_ellipse(
        app.world_mut(),
        document,
        CanvasEllipse {
            radii: Vec2::splat(8.0),
        },
        Transform::from_xyz(0.0, 12.0, 0.0),
        default(),
    )
    .unwrap();
    CanvasView::set_object_snap_in_world(
        app.world_mut(),
        view,
        CanvasObjectSnap {
            connector_anchors: true,
            ..default()
        },
    )
    .unwrap();
    CanvasView::set_grid_in_world(
        app.world_mut(),
        view,
        CanvasGrid {
            snap: CanvasGridSnap {
                connector_points: true,
                ..default()
            },
            ..default()
        },
    )
    .unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Connector(default())).unwrap();
    for point in [
        Vec2::new(0.0, 4.0),
        Vec2::new(4.0, 1.0),
        Vec2::new(30.0, 20.0),
    ] {
        click_at(&mut app, view, &base, point);
    }
    press_key(&mut app, window, KeyCode::Enter);
    let item = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    assert_eq!(
        app.world()
            .get::<CanvasConnector>(item)
            .unwrap()
            .route()
            .waypoints,
        [Vec2::ZERO]
    );
}

#[test]
fn translating_an_attached_connector_uses_the_movable_end_for_grid_and_keeps_capture_at_zero() {
    for (grid, offsets) in [
        (true, [10.0, 11.0]),
        (false, [-40.0, -48.0]),
        (false, [-38.0, -40.0]),
    ] {
        let (mut app, view, _, shape, _, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
        let document = CanvasView::document(app.world(), view).unwrap().unwrap();
        CanvasDocument::update_transform(
            app.world_mut(),
            document,
            shape,
            Transform::from_xyz(-15.0, 0.0, 0.0),
        )
        .unwrap();
        let item = CanvasDocument::create_connector(
            app.world_mut(),
            document,
            CanvasConnectorRoute {
                start: Vec2::ZERO,
                end: Vec2::X * 40.0,
                waypoints: Vec::new(),
            },
            default(),
            (offsets[1] == -40.0).then(label),
        )
        .unwrap();
        CanvasDocument::set_connector_endpoint(
            app.world_mut(),
            document,
            item,
            CanvasConnectorEnd::Start,
            CanvasConnectorEndpoint::Attached {
                target: shape,
                anchor: CanvasAnchor::East,
            },
        )
        .unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), view, [item]).unwrap();
        CanvasView::set_grid_in_world(
            app.world_mut(),
            view,
            CanvasGrid {
                spacing: 40.0,
                snap: CanvasGridSnap {
                    movement: grid,
                    ..default()
                },
                ..default()
            },
        )
        .unwrap();
        common::update(&mut app);
        let grab = Vec2::X * 30.0;
        let mut last = coordinates::at_document(&app, view, &base, grab);
        common::input(
            &mut app,
            PointerId::Mouse,
            &last,
            PointerAction::Move { delta: Vec2::ONE },
        );
        common::input(
            &mut app,
            PointerId::Mouse,
            &last,
            PointerAction::Press(PointerButton::Primary),
        );
        for offset in offsets {
            let next = coordinates::at_document(&app, view, &base, grab + Vec2::X * offset);
            common::input(
                &mut app,
                PointerId::Mouse,
                &next,
                PointerAction::Move {
                    delta: next.position - last.position,
                },
            );
            assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [item]);
            let model = app.world().get::<CanvasConnector>(item).unwrap();
            let transform = app.world().get::<Transform>(item).unwrap();
            assert_eq!(
                transform.transform_point(model.route().start.extend(0.0)),
                Vec3::ZERO
            );
            assert_eq!(
                transform.transform_point(model.route().end.extend(0.0)),
                Vec3::X * (40.0 + offset)
            );
            last = next;
        }
        common::input(
            &mut app,
            PointerId::Mouse,
            &last,
            PointerAction::Release(PointerButton::Primary),
        );
        if offsets[1] == -40.0 {
            let label_count = app
                .world_mut()
                .query::<&Name>()
                .iter(app.world())
                .filter(|name| name.as_str() == "CanvasGlyph")
                .count();
            assert_eq!(label_count, 0);
            click_at(&mut app, view, &base, Vec2::new(65.0, 30.0));
            assert!(CanvasView::selection(app.world(), view).unwrap().is_empty());
            click_at(&mut app, view, &base, Vec2::ZERO);
            assert_eq!(CanvasView::selection(app.world(), view).unwrap(), [item]);
        }
    }
}

#[test]
fn pointer_drawing_cancels_cleanly_and_commits_editable_route_points() {
    let (mut app, view, auxiliary, original, window, base) =
        common::fixture(PointerId::Mouse, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasDocument::clear_history(app.world_mut(), doc).unwrap();
    let mode = CanvasMode::Connector(arrow_style(5.0));
    let start = Vec2::new(-65.0, -35.0);
    let middle = Vec2::new(40.0, -35.0);
    let end = Vec2::new(40.0, 28.0);
    CanvasView::set_mode_in_world(app.world_mut(), view, mode).unwrap();
    click_at(&mut app, view, &base, start);
    click_at(&mut app, view, &base, middle);
    let preview_count = |world: &mut World| {
        world
            .query::<&Name>()
            .iter(world)
            .filter(|name| name.as_str() == "CanvasConnectorPreview")
            .count()
    };
    assert_eq!(preview_count(app.world_mut()), 1);
    assert_eq!(
        CanvasDocument::draw_order(app.world(), doc).unwrap(),
        [original]
    );
    press_key(&mut app, window, KeyCode::Escape);
    assert_eq!(preview_count(app.world_mut()), 0);
    assert!(!CanvasDocument::can_undo(app.world(), doc).unwrap());
    CanvasView::set_mode_in_world(app.world_mut(), view, mode).unwrap();
    for point in [start, middle, end] {
        click_at(&mut app, view, &base, point);
    }
    press_key(&mut app, window, KeyCode::Enter);
    let item = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    assert_eq!(
        CanvasView::single_selection(app.world(), auxiliary).unwrap(),
        None
    );
    assert_eq!(preview_count(app.world_mut()), 0);
    let before = CanvasConnectorRoute {
        start,
        waypoints: vec![middle],
        end,
    };
    assert_eq!(
        app.world().get::<CanvasConnector>(item).unwrap().route(),
        &before
    );
    assert!(CanvasDocument::undo(app.world_mut(), doc).unwrap());
    assert!(!CanvasDocument::can_undo(app.world(), doc).unwrap());
    assert!(CanvasDocument::redo(app.world_mut(), doc).unwrap());

    CanvasView::set_selection_in_world(app.world_mut(), view, Some(item)).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::PathNodeEdit).unwrap();
    let delta = Vec2::new(-10.0, 10.0);
    drag(&mut app, view, &base, middle, delta);
    drag(&mut app, view, &base, end, delta);
    assert_eq!(
        app.world().get::<CanvasConnector>(item).unwrap().route(),
        &CanvasConnectorRoute {
            start,
            waypoints: vec![middle + delta],
            end: end + delta,
        }
    );
    assert!(CanvasDocument::undo(app.world_mut(), doc).unwrap());
    assert!(CanvasDocument::undo(app.world_mut(), doc).unwrap());
    assert_eq!(
        app.world().get::<CanvasConnector>(item).unwrap().route(),
        &before
    );
}

#[test]
fn connector_label_meshes_follow_a_moving_attachment_in_both_views() {
    let (mut app, view, _, shape, _, _) = common::fixture(PointerId::Mouse, 1., 1.);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    let connector = CanvasDocument::create_connector(
        app.world_mut(),
        document,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            end: Vec2::new(70., 10.),
            waypoints: Vec::new(),
        },
        arrow_style(3.),
        Some(CanvasConnectorLabel {
            text: "O".into(),
            position: 0.25,
            ..label()
        }),
    )
    .unwrap();
    CanvasDocument::set_connector_endpoint(
        app.world_mut(),
        document,
        connector,
        CanvasConnectorEnd::Start,
        CanvasConnectorEndpoint::Attached {
            target: shape,
            anchor: CanvasAnchor::East,
        },
    )
    .unwrap();
    let projected = |app: &mut App| {
        let route = app
            .world()
            .get::<CanvasConnector>(connector)
            .unwrap()
            .route();
        let expected = route.start.lerp(route.end, 0.25) * Vec2::new(1., -1.);
        let positions = app
            .world_mut()
            .query::<(&Name, &GlobalTransform)>()
            .iter(app.world())
            .filter(|(name, _)| name.as_str() == "CanvasTextProxy")
            .map(|(_, pose)| pose.translation().truncate())
            .collect::<Vec<_>>();
        assert_eq!(positions.len(), 2);
        assert!(positions.iter().all(|at| at.abs_diff_eq(expected, 0.001)));
        expected
    };
    common::update(&mut app);
    let before = projected(&mut app);
    CanvasDocument::update_transform(
        app.world_mut(),
        document,
        shape,
        Transform::from_xyz(8., 20., 0.),
    )
    .unwrap();
    common::update(&mut app);
    assert_ne!(projected(&mut app), before);
}

#[test]
fn pointer_hits_arrow_wings_and_label_and_moves_the_connector_as_one_item() {
    let (mut app, view, auxiliary, _, window, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let route = CanvasConnectorRoute {
        start: Vec2::new(-72.0, -40.0),
        waypoints: vec![Vec2::new(-12.0, -40.0)],
        end: Vec2::new(-12.0, 34.0),
    };
    let item = CanvasDocument::create_connector(
        app.world_mut(),
        doc,
        route.clone(),
        arrow_style(8.0),
        None,
    )
    .unwrap();
    for (point, expected) in [
        (Vec2::new(-44.0, -50.0), Some(item)),
        (Vec2::new(-22.0, 4.0), Some(item)),
        (Vec2::new(-36.0, -40.0), Some(item)),
        (Vec2::new(-49.0, -12.0), None),
    ] {
        CanvasView::set_selection_in_world(app.world_mut(), view, None).unwrap();
        click_at(&mut app, view, &base, point);
        assert_eq!(
            CanvasView::single_selection(app.world(), view).unwrap(),
            expected
        );
    }
    let label = CanvasConnectorLabel {
        text: "STEP".to_owned(),
        ..label()
    };
    CanvasDocument::update_connector(
        app.world_mut(),
        doc,
        item,
        route.clone(),
        arrow_style(8.0),
        Some(label),
    )
    .unwrap();
    common::update(&mut app);
    let labels = app
        .world_mut()
        .query::<(&Name, &Transform)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "CanvasTextProxy")
        .map(|(_, pose)| pose.translation.truncate())
        .collect::<Vec<_>>();
    assert_eq!(labels, [Vec2::new(-12.0, -33.0); 2]);
    click_at(&mut app, auxiliary, &base, Vec2::new(-22.0, -30.0));
    assert_eq!(
        CanvasView::single_selection(app.world(), auxiliary).unwrap(),
        Some(item)
    );
    let delta = Vec2::new(8.0, 15.0);
    let body = Vec2::new(-55.0, -40.0);
    click_at(&mut app, view, &base, body);
    drag(&mut app, view, &base, body, delta);
    assert_eq!(
        app.world()
            .get::<Transform>(item)
            .unwrap()
            .translation
            .truncate(),
        delta
    );
    assert_eq!(
        app.world().get::<CanvasConnector>(item).unwrap().route(),
        &route
    );
    press_key(&mut app, window, KeyCode::Delete);
    assert!(
        !CanvasDocument::draw_order(app.world(), doc)
            .unwrap()
            .contains(&item)
    );
    assert_eq!(
        CanvasView::single_selection(app.world(), auxiliary).unwrap(),
        None
    );
    assert!(CanvasDocument::undo(app.world_mut(), doc).unwrap());
    assert!(
        CanvasDocument::draw_order(app.world(), doc)
            .unwrap()
            .contains(&item)
    );
}

#[test]
fn connector_tool_projects_points_into_the_edited_group() {
    let (mut app, view, _, original, window, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let other = CanvasDocument::create_text(
        app.world_mut(),
        doc,
        CanvasTextStyle::default(),
        Transform::IDENTITY,
    )
    .unwrap();
    let group =
        CanvasDocument::group_items(app.world_mut(), doc, &[original, other], "Flow").unwrap();
    let scale = 2.0;
    CanvasDocument::update_group_transform(
        app.world_mut(),
        doc,
        group,
        Transform::from_xyz(5.0, -10.0, 0.0)
            .with_rotation(Quat::from_rotation_z(0.4))
            .with_scale(Vec3::new(scale, scale, 1.0)),
    )
    .unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    let style = CanvasConnectorStyle {
        stroke: CanvasStroke {
            width: 4.0,
            ..default()
        },
        ..default()
    };
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Connector(style)).unwrap();
    let start = Vec2::new(-60.0, -30.0);
    let end = Vec2::new(30.0, 40.0);
    click_at(&mut app, view, &base, start);
    click_at(&mut app, view, &base, end);
    press_key(&mut app, window, KeyCode::Enter);
    let item = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    assert_eq!(
        CanvasDocument::container_of(app.world(), doc, item).unwrap(),
        CanvasContainer::Group(group)
    );
    let connector = app.world().get::<CanvasConnector>(item).unwrap();
    assert!(connector.route().waypoints.is_empty());
    assert!((connector.style().stroke.width - style.stroke.width / scale).abs() < 0.001);
    assert_eq!(
        app.world().get::<Transform>(item),
        Some(&Transform::IDENTITY)
    );
    let project = CanvasDocument::document_transform(app.world(), doc, item).unwrap();
    for (local, expected) in [
        (connector.route().start, start),
        (connector.route().end, end),
    ] {
        assert!(
            project
                .transform_point3(local.extend(0.0))
                .truncate()
                .abs_diff_eq(expected, 0.001)
        );
    }
}
