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
        .query::<(&Text2d, &Transform)>()
        .iter(app.world())
        .filter(|(text, _)| text.0 == "STEP")
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
