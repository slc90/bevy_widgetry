#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    camera::visibility::RenderLayers, picking::pointer::*, prelude::*, ui::widget::ViewportNode,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key};

#[allow(dead_code)]
mod common;
#[path = "common/coordinates.rs"]
mod coordinates;

const TOLERANCE: f32 = 0.001;
const GRID: CanvasGrid = CanvasGrid {
    visible: true,
    snap: CanvasGridSnap::ALL,
    spacing: 20.0,
    major_every: Some(4),
};

#[derive(Resource, Default)]
struct GridEvents(Vec<(Entity, CanvasGrid)>);

fn positions(world: &World, handle: &Handle<Mesh>) -> Vec<[f32; 3]> {
    world
        .resource::<Assets<Mesh>>()
        .get(handle)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap()
        .to_vec()
}

#[test]
fn grid_is_local_to_the_view_updates_in_place_and_has_no_document_history() {
    let (mut app, a, b, shape, _, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), a).unwrap().unwrap();
    app.init_resource::<GridEvents>();
    app.add_observer(
        |event: On<CanvasGridChanged>, mut events: ResMut<GridEvents>| {
            events.0.push((event.entity, event.grid));
        },
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("grid.svg");
    let before = {
        CanvasDocument::export_svg(app.world_mut(), doc, &path, None).unwrap();
        std::fs::read(&path).unwrap()
    };
    assert!(CanvasView::set_grid_in_world(app.world_mut(), a, GRID).unwrap());
    assert!(!CanvasView::set_grid_in_world(app.world_mut(), a, GRID).unwrap());
    assert!(
        CanvasView::set_grid_in_world(
            app.world_mut(),
            a,
            CanvasGrid {
                spacing: 0.0,
                ..GRID
            },
        )
        .is_err()
    );
    assert!(
        CanvasView::set_grid_in_world(
            app.world_mut(),
            a,
            CanvasGrid {
                major_every: Some(0),
                ..GRID
            },
        )
        .is_err()
    );
    assert_eq!(CanvasView::grid(app.world(), a).unwrap(), GRID);
    assert_eq!(
        CanvasView::grid(app.world(), b).unwrap(),
        CanvasGrid::default()
    );
    assert_eq!(app.world().resource::<GridEvents>().0, [(a, GRID)]);
    common::update(&mut app);
    let (overlay, handle, material, layers) = app
        .world_mut()
        .query::<(
            Entity,
            &Name,
            &Mesh2d,
            &MeshMaterial2d<ColorMaterial>,
            &RenderLayers,
        )>()
        .iter(app.world())
        .find(|(_, name, ..)| name.as_str() == "CanvasGrid")
        .map(|(entity, _, mesh, material, layers)| {
            (entity, mesh.0.clone(), material.0.clone(), layers.clone())
        })
        .unwrap();
    assert!(matches!(
        app.world()
            .resource::<Assets<ColorMaterial>>()
            .get(&material)
            .unwrap()
            .alpha_mode,
        bevy::sprite_render::AlphaMode2d::Blend
    ));
    for (view, shown) in [(a, true), (b, false)] {
        let camera = app
            .world()
            .get::<ViewportNode>(view)
            .unwrap()
            .camera
            .unwrap();
        assert_eq!(
            layers.intersects(app.world().get::<RenderLayers>(camera).unwrap()),
            shown
        );
    }
    let initial = positions(app.world(), &handle);
    let center = Vec2::new(-14.0, 13.0);
    CanvasView::set_center_in_world(app.world_mut(), a, center).unwrap();
    CanvasView::set_zoom_in_world(app.world_mut(), a, 1.5).unwrap();
    common::update(&mut app);
    let panned = positions(app.world(), &handle);
    assert_ne!(initial, panned);
    assert_eq!(app.world().get::<Mesh2d>(overlay).unwrap().0, handle);
    for quad in panned.as_chunks::<4>().0 {
        let axis = usize::from((quad[2][1] - quad[0][1]) < (quad[2][0] - quad[0][0]));
        let line_center = (quad[2][axis] + quad[0][axis]) * 0.5 + center[axis];
        assert!(
            (line_center / GRID.spacing - (line_center / GRID.spacing).round()).abs() < TOLERANCE
        );
    }
    click(&mut app, a, &base, Vec2::new(-40.0, 20.0));
    assert_eq!(CanvasView::single_selection(app.world(), a).unwrap(), None);
    assert_eq!(
        CanvasDocument::draw_order(app.world(), doc).unwrap(),
        [shape]
    );
    CanvasDocument::export_svg(app.world_mut(), doc, &path, None).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(CanvasDocument::undo(app.world_mut(), doc).unwrap());
    assert!(!CanvasDocument::can_undo(app.world(), doc).unwrap());
    assert_eq!(CanvasView::grid(app.world(), a).unwrap(), GRID);
    assert!(app.world_mut().despawn(a));
    common::update(&mut app);
    assert!(app.world().get_entity(overlay).is_err());
    assert!(!app.world().resource::<Assets<Mesh>>().contains(&handle));
    assert!(
        !app.world()
            .resource::<Assets<ColorMaterial>>()
            .contains(&material)
    );
}

fn click(app: &mut App, view: Entity, base: &Location, point: Vec2) {
    let at = coordinates::at_document(app, view, base, point);
    for action in [
        PointerAction::Move { delta: Vec2::ONE },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        common::input(app, PointerId::Mouse, &at, action);
    }
}

fn drag(app: &mut App, view: Entity, base: &Location, start: Vec2, end: Vec2) {
    let from = coordinates::at_document(app, view, base, start);
    let to = coordinates::at_document(app, view, base, end);
    common::input(
        app,
        PointerId::Mouse,
        &from,
        PointerAction::Move { delta: Vec2::ONE },
    );
    common::input(
        app,
        PointerId::Mouse,
        &from,
        PointerAction::Press(PointerButton::Primary),
    );
    common::input(
        app,
        PointerId::Mouse,
        &to,
        PointerAction::Move {
            delta: to.position - from.position,
        },
    );
    common::input(
        app,
        PointerId::Mouse,
        &to,
        PointerAction::Release(PointerButton::Primary),
    );
}

#[test]
fn drawing_snaps_confirmed_points_and_preview_only_in_the_configured_view() {
    let (mut app, a, b, original, window, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    let doc = CanvasView::document(app.world(), a).unwrap().unwrap();
    let hidden_grid = CanvasGrid {
        visible: false,
        ..GRID
    };
    CanvasView::set_grid_in_world(app.world_mut(), a, hidden_grid).unwrap();
    CanvasView::set_mode_in_world(
        app.world_mut(),
        a,
        CanvasMode::Rectangle(CanvasPaint::default()),
    )
    .unwrap();
    drag(
        &mut app,
        a,
        &base,
        Vec2::new(-61.0, -39.0),
        Vec2::new(-22.0, -21.0),
    );
    let item = CanvasView::single_selection(app.world(), a)
        .unwrap()
        .unwrap();
    let rectangle = app
        .world()
        .get::<CanvasGeometry>(item)
        .unwrap()
        .rectangle()
        .unwrap();
    let pose = app.world().get::<Transform>(item).unwrap();
    assert_eq!(rectangle.size, Vec2::new(40.0, 20.0));
    assert_eq!(pose.translation.truncate(), Vec2::new(-40.0, -30.0));

    let mode = CanvasMode::Connector(CanvasConnectorStyle::default());
    CanvasView::set_grid_in_world(
        app.world_mut(),
        a,
        CanvasGrid {
            snap: CanvasGridSnap {
                connector_points: true,
                ..default()
            },
            ..hidden_grid
        },
    )
    .unwrap();
    CanvasView::set_grid_in_world(
        app.world_mut(),
        b,
        CanvasGrid {
            snap: CanvasGridSnap {
                drawing: true,
                ..default()
            },
            ..hidden_grid
        },
    )
    .unwrap();
    let start = Vec2::new(22.0, 21.0);
    let end = Vec2::new(42.0, 39.0);
    for (view, snap) in [(a, true), (b, false)] {
        let expected = if snap {
            (Vec2::splat(20.0), Vec2::splat(40.0))
        } else {
            (start, end)
        };
        CanvasView::set_mode_in_world(app.world_mut(), view, mode).unwrap();
        click(&mut app, view, &base, start);
        let hover = coordinates::at_document(&app, view, &base, end);
        common::input(
            &mut app,
            PointerId::Mouse,
            &hover,
            PointerAction::Move { delta: Vec2::ONE },
        );
        let preview = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "CanvasConnectorPreview")
            .unwrap()
            .0;
        let stroke = app.world().get::<Children>(preview).unwrap()[1];
        let handle = &app.world().get::<Mesh2d>(stroke).unwrap().0;
        // A butt cap ends at the target along the route tangent.
        let tangent = (expected.1 - expected.0).normalize();
        let preview_tip = positions(app.world(), handle)
            .into_iter()
            .map(|[x, y, _]| (Vec2::new(x, y) - expected.0).dot(tangent))
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((preview_tip - expected.0.distance(expected.1)).abs() < TOLERANCE);
        click(&mut app, view, &base, end);
        press_key(&mut app, window, KeyCode::Enter);
        let item = CanvasView::single_selection(app.world(), view)
            .unwrap()
            .unwrap();
        let route = app.world().get::<CanvasConnector>(item).unwrap().route();
        assert_eq!((route.start, route.end), expected);
    }
    let count = CanvasDocument::draw_order(app.world(), doc).unwrap().len();
    assert_eq!(count, 4);
    CanvasView::set_mode_in_world(app.world_mut(), a, mode).unwrap();
    click(&mut app, a, &base, start);
    press_key(&mut app, window, KeyCode::Escape);
    assert_eq!(
        CanvasDocument::draw_order(app.world(), doc).unwrap()[0],
        original
    );
    assert_eq!(
        CanvasDocument::draw_order(app.world(), doc).unwrap().len(),
        count
    );
    assert!(
        !app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .any(|name| matches!(name.as_str(), "CanvasGrid" | "CanvasConnectorPreview"))
    );
}

#[test]
fn whole_item_motion_and_local_node_edits_snap_the_document_target_with_grab_offset() {
    let (mut app, view, _, shape, _, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    CanvasView::set_zoom_in_world(app.world_mut(), view, 2.0).unwrap();
    CanvasView::set_grid_in_world(app.world_mut(), view, GRID).unwrap();
    let start = Vec2::new(-18.0, -21.0);
    CanvasDocument::update_transform(
        app.world_mut(),
        doc,
        shape,
        Transform::from_translation(start.extend(0.0)),
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<Transform>(shape)
            .unwrap()
            .translation
            .truncate(),
        start
    );
    CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
    common::update(&mut app);
    let grab = Vec2::new(2.0, 1.0);
    drag(
        &mut app,
        view,
        &base,
        start + grab,
        Vec2::new(1.0, -21.0) + grab,
    );
    assert_eq!(
        app.world()
            .get::<Transform>(shape)
            .unwrap()
            .translation
            .truncate(),
        Vec2::new(0.0, -20.0)
    );

    let path = CanvasDocument::create_path(
        app.world_mut(),
        doc,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::ZERO),
                CanvasPathCommand::LineTo(Vec2::new(-6.0, 15.0)),
            ],
        },
        Transform::from_xyz(20.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::new(2.0, 2.0, 1.0)),
        CanvasPaint::default(),
    )
    .unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, Some(path)).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::PathNodeEdit).unwrap();
    common::update(&mut app);
    let node_grab = Vec2::new(1.0, 0.5);
    drag(
        &mut app,
        view,
        &base,
        Vec2::new(20.0, 0.0) + node_grab,
        Vec2::new(41.0, 22.0) + node_grab,
    );
    assert!(matches!(
        app.world().get::<CanvasGeometry>(path),
        Some(CanvasGeometry::Path(value))
            if matches!(value.commands.first(), Some(CanvasPathCommand::MoveTo(point))
                if point.abs_diff_eq(Vec2::new(10.0, -10.0), TOLERANCE))
    ));

    let connector = CanvasDocument::create_connector(
        app.world_mut(),
        doc,
        CanvasConnectorRoute {
            start: Vec2::new(-20.0, 0.0),
            waypoints: Vec::new(),
            end: Vec2::new(-8.0, 8.0),
        },
        CanvasConnectorStyle::default(),
        None,
    )
    .unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, Some(connector)).unwrap();
    common::update(&mut app);
    drag(
        &mut app,
        view,
        &base,
        Vec2::new(-20.0, 0.0),
        Vec2::new(-42.0, 19.0),
    );
    assert_eq!(
        app.world()
            .get::<CanvasConnector>(connector)
            .unwrap()
            .route()
            .start,
        Vec2::new(-40.0, 20.0)
    );
    CanvasView::set_grid_in_world(
        app.world_mut(),
        view,
        CanvasGrid {
            snap: CanvasGridSnap {
                connector_points: false,
                ..CanvasGridSnap::ALL
            },
            ..GRID
        },
    )
    .unwrap();
    drag(
        &mut app,
        view,
        &base,
        Vec2::new(-40.0, 20.0),
        Vec2::new(-42.0, 19.0),
    );
    assert_eq!(
        app.world()
            .get::<CanvasConnector>(connector)
            .unwrap()
            .route()
            .start,
        Vec2::new(-42.0, 19.0)
    );
}

#[test]
fn a_multiselection_drag_keeps_both_items_when_the_grid_absorbs_the_motion() {
    let (mut app, view, _, first, _, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let second = CanvasDocument::create_rectangle(
        app.world_mut(),
        doc,
        CanvasRectangle {
            size: Vec2::splat(15.0),
            corner_radius: 0.0,
        },
        Transform::from_xyz(40.0, 0.0, 0.0),
        CanvasPaint::default(),
    )
    .unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, [first, second]).unwrap();
    CanvasView::set_grid_in_world(app.world_mut(), view, GRID).unwrap();
    CanvasDocument::clear_history(app.world_mut(), doc).unwrap();
    common::update(&mut app);
    let selection = CanvasView::selection(app.world(), view).unwrap();
    drag(
        &mut app,
        view,
        &base,
        Vec2::new(2.0, 1.0),
        Vec2::new(5.0, 1.0),
    );
    assert_eq!(CanvasView::selection(app.world(), view).unwrap(), selection);
    assert_eq!(
        app.world().get::<Transform>(first),
        Some(&Transform::IDENTITY)
    );
    assert!(!CanvasDocument::can_undo(app.world(), doc).unwrap());
}

#[test]
fn radial_tools_keep_the_grid_vertex_when_shift_is_held() {
    let (mut app, view, _, _, _, base) = common::fixture(PointerId::Mouse, 1.0, 1.0);
    CanvasView::set_grid_in_world(
        app.world_mut(),
        view,
        CanvasGrid {
            spacing: 25.0,
            ..GRID
        },
    )
    .unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    let with_angle_snap = Vec2::from_angle(std::f32::consts::FRAC_PI_6) * 50.0;
    for tool in [
        CanvasPathTool::RegularPolygon { sides: 5 },
        CanvasPathTool::Star {
            points: 5,
            inner_ratio: 0.4,
        },
    ] {
        for (raw, expected) in [
            (Vec2::new(51.0, 26.0), Vec2::new(50.0, 25.0)),
            (Vec2::new(40.0, 30.0), with_angle_snap),
        ] {
            CanvasView::set_mode_in_world(
                app.world_mut(),
                view,
                CanvasMode::Parametric(tool, CanvasPaint::default()),
            )
            .unwrap();
            drag(&mut app, view, &base, Vec2::ZERO, raw);
            let item = CanvasView::single_selection(app.world(), view)
                .unwrap()
                .unwrap();
            assert!(
                matches!(app.world().get::<CanvasGeometry>(item), Some(CanvasGeometry::Path(path))
                if matches!(path.commands.first(), Some(CanvasPathCommand::MoveTo(point))
                    if point.abs_diff_eq(expected, TOLERANCE)))
            );
        }
    }
}
