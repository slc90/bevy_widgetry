#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::camera::{RenderTarget, ScalingMode};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_ui_plugins, scene_app, spawn_ui_camera,
};

fn fixture(scale: f32) -> (App, Entity, Entity, Entity) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((TransformPlugin, CanvasPlugin));
    let ui_camera = spawn_ui_camera(&mut app, UVec2::new(1000, 800), scale);
    let world = app.world_mut();
    let document = world.spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_rectangle(
        world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::new(120.0, 80.0),
        },
        Transform::from_xyz(-20.0, 30.0, 0.0).with_rotation(Quat::from_rotation_z(0.3)),
        CanvasPaint {
            fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
            stroke: Some(CanvasStroke {
                color: Color::BLACK,
                width: 10.0,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let view = world.spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } template(move |_| Ok(UiTargetCamera(ui_camera))) Node { width: px(300), height: px(200) } }).unwrap().id();
    app.update();
    app.update();
    (app, document, shape, view)
}

fn named(world: &mut World, name: &str) -> Entity {
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, value)| value.as_str() == name)
        .unwrap()
        .0
}

fn positions(world: &World, entity: Entity) -> Vec<[f32; 3]> {
    let handle = &world.get::<Mesh2d>(entity).unwrap().0;
    let mesh = world.resource::<Assets<Mesh>>().get(handle).unwrap();
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions missing");
    };
    positions.clone()
}

#[test]
fn empty_path_paints_stay_on_cpu_and_hide_without_replacing_shared_handles() {
    use CanvasPathCommand::*;
    use bevy::asset::RenderAssetUsages;
    let (mut app, document, rectangle, a) = fixture(1.0);
    CanvasDocument::remove_shape(app.world_mut(), document, rectangle).unwrap();
    let square = CanvasPath {
        commands: vec![
            MoveTo(Vec2::ZERO),
            LineTo(Vec2::new(30.0, 0.0)),
            LineTo(Vec2::splat(30.0)),
            Close,
        ],
    };
    let empty = CanvasPath {
        commands: vec![MoveTo(Vec2::ZERO)],
    };
    let paint = CanvasPaint {
        stroke: Some(CanvasStroke::default()),
        ..default()
    };
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        square.clone(),
        Transform::default(),
        paint,
    )
    .unwrap();
    app.world_mut().spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } Node {width:px(200),height:px(200)} }).unwrap();
    let mut original = None;
    for path in [square.clone(), empty.clone(), square, empty] {
        CanvasDocument::update_path(
            app.world_mut(),
            document,
            shape,
            path,
            Transform::default(),
            paint,
        )
        .unwrap();
        app.update();
        let mut handles = Vec::new();
        let mut query = app.world_mut().query::<(&Name, &Mesh2d, &Visibility)>();
        let meshes = app.world().resource::<Assets<Mesh>>();
        for (name, mesh, visibility) in query.iter(app.world()) {
            if !matches!(name.as_str(), "CanvasFill" | "CanvasStroke") {
                continue;
            }
            handles.push(mesh.0.id());
            let mesh = meshes.get(&mesh.0).unwrap();
            assert!(mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD));
            if mesh.count_vertices() == 0 {
                assert!(
                    !mesh.asset_usage.contains(RenderAssetUsages::RENDER_WORLD),
                    "empty {name} was submitted to GPU"
                );
                assert_eq!(*visibility, Visibility::Hidden);
            } else {
                assert!(mesh.asset_usage.contains(RenderAssetUsages::RENDER_WORLD));
                assert_eq!(*visibility, Visibility::Inherited);
            }
        }
        assert_eq!(handles.len(), 4);
        handles.sort();
        if let Some(ref original) = original {
            assert_eq!(&handles, original);
        } else {
            original = Some(handles);
        }
        assert_eq!(
            CanvasView::document(app.world(), a).unwrap(),
            Some(document)
        );
    }
}

#[test]
fn path_views_share_meshes_refresh_commands_and_keep_transform_updates_geometry_free() {
    use CanvasPathCommand::*;
    let (mut app, document, _, a) = fixture(1.0);
    let p = CanvasPath {
        commands: vec![
            MoveTo(Vec2::new(-30.0, -20.0)),
            LineTo(Vec2::new(30.0, -20.0)),
            QuadTo {
                control: Vec2::new(50.0, 30.0),
                to: Vec2::new(-30.0, 20.0),
            },
            Close,
            MoveTo(Vec2::new(60.0, 0.0)),
            LineTo(Vec2::new(90.0, 10.0)),
        ],
    };
    let paint = CanvasPaint {
        fill: Some(Color::srgb(0.6, 0.2, 0.8)),
        stroke: Some(CanvasStroke {
            color: Color::WHITE,
            width: 4.0,
            ..Default::default()
        }),
        ..Default::default()
    };
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        p.clone(),
        Transform::default(),
        paint,
    )
    .unwrap();
    let b = app
        .world_mut()
        .spawn_scene(
            bsn! {@CanvasView {@document:{Some(document)}} Node {width:px(200),height:px(200)}},
        )
        .unwrap()
        .id();
    app.update();
    app.update();
    let fills = app
        .world_mut()
        .query::<(Entity, &Name, &MeshMaterial2d<ColorMaterial>)>()
        .iter(app.world())
        .filter(|(_, n, m)| {
            n.as_str() == "CanvasFill"
                && app
                    .world()
                    .resource::<Assets<ColorMaterial>>()
                    .get(&m.0)
                    .unwrap()
                    .color
                    == paint.fill.unwrap()
        })
        .map(|(e, _, _)| e)
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 2);
    let handle = app.world().get::<Mesh2d>(fills[0]).unwrap().0.clone();
    assert_eq!(app.world().get::<Mesh2d>(fills[1]).unwrap().0, handle);
    let mesh_count = app.world().resource::<Assets<Mesh>>().len();
    let mut styled = paint;
    for step in 0..4 {
        match step {
            0 => styled.fill_rule = CanvasFillRule::EvenOdd,
            1 => styled.stroke.as_mut().unwrap().cap = CanvasLineCap::Round,
            2 => styled.stroke.as_mut().unwrap().join = CanvasLineJoin::Bevel,
            _ => styled.stroke.as_mut().unwrap().width = 12.0,
        }
        let mut reader = app
            .world()
            .resource::<Messages<AssetEvent<Mesh>>>()
            .get_cursor_current();
        CanvasDocument::update_path(
            app.world_mut(),
            document,
            shape,
            p.clone(),
            Transform::default(),
            styled,
        )
        .unwrap();
        app.update();
        app.update();
        assert!(
            reader
                .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
                .any(|e| matches!(e,AssetEvent::Modified{id} if *id==handle.id())),
            "style step {step} did not refresh mesh"
        );
        assert_eq!(app.world().get::<Mesh2d>(fills[0]).unwrap().0, handle);
        assert_eq!(app.world().get::<Mesh2d>(fills[1]).unwrap().0, handle);
        assert_eq!(
            positions(app.world(), fills[0]),
            positions(app.world(), fills[1])
        );
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), mesh_count);
    }
    CanvasDocument::update_path(
        app.world_mut(),
        document,
        shape,
        p.clone(),
        Transform::default(),
        paint,
    )
    .unwrap();
    app.update();
    app.update();
    let before = positions(app.world(), fills[0]);
    let transform = Transform::from_xyz(20.0, 30.0, 0.0)
        .with_rotation(Quat::from_rotation_z(0.3))
        .with_scale(Vec3::new(-2.0, 0.5, 0.0));
    let mut reader = app
        .world()
        .resource::<Messages<AssetEvent<Mesh>>>()
        .get_cursor_current();
    CanvasDocument::update_path(
        app.world_mut(),
        document,
        shape,
        p.clone(),
        transform,
        CanvasPaint {
            fill: Some(Color::srgb(0.2, 0.8, 0.6)),
            stroke: paint.stroke.map(|s| CanvasStroke {
                color: Color::BLACK,
                ..s
            }),
            ..paint
        },
    )
    .unwrap();
    app.update();
    assert_eq!(positions(app.world(), fills[0]), before);
    assert!(
        !reader
            .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
            .any(|e| matches!(e,AssetEvent::Modified{id} if *id==handle.id()))
    );
    let mut changed = p;
    changed.commands[1] = LineTo(Vec2::new(45.0, -20.0));
    CanvasDocument::update_path(app.world_mut(), document, shape, changed, transform, paint)
        .unwrap();
    app.update();
    assert_ne!(positions(app.world(), fills[0]), before);
    assert_eq!(
        positions(app.world(), fills[0]),
        positions(app.world(), fills[1])
    );
    assert_eq!(CanvasView::single_selection(app.world(), a).unwrap(), None);
    assert_eq!(CanvasView::single_selection(app.world(), b).unwrap(), None);
    let before_append = positions(app.world(), fills[0]);
    CanvasDocument::append_rectangle(
        app.world_mut(),
        document,
        shape,
        Rect::new(-80.0, -50.0, -60.0, -30.0),
    )
    .unwrap();
    CanvasDocument::append_circle(
        app.world_mut(),
        document,
        shape,
        Vec2::new(0.0, -80.0),
        10.0,
    )
    .unwrap();
    CanvasDocument::append_ellipse(
        app.world_mut(),
        document,
        shape,
        Vec2::new(60.0, -60.0),
        Vec2::new(20.0, 10.0),
    )
    .unwrap();
    CanvasDocument::append_line(
        app.world_mut(),
        document,
        shape,
        Vec2::new(0.0, 80.0),
        Vec2::new(20.0, 80.0),
    )
    .unwrap();
    CanvasDocument::append_polyline(
        app.world_mut(),
        document,
        shape,
        &[
            Vec2::new(30.0, 70.0),
            Vec2::new(40.0, 80.0),
            Vec2::new(50.0, 70.0),
        ],
    )
    .unwrap();
    CanvasDocument::append_polygon(
        app.world_mut(),
        document,
        shape,
        &[
            Vec2::new(-40.0, 70.0),
            Vec2::new(-30.0, 80.0),
            Vec2::new(-20.0, 70.0),
        ],
    )
    .unwrap();
    app.update();
    assert_ne!(positions(app.world(), fills[0]), before_append);
    assert_eq!(
        positions(app.world(), fills[0]),
        positions(app.world(), fills[1])
    );
    assert_eq!(app.world().get::<Mesh2d>(fills[0]).unwrap().0, handle);
    assert_eq!(app.world().get::<Mesh2d>(fills[1]).unwrap().0, handle);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), mesh_count);
    assert_eq!(
        CanvasDocument::shape_order(app.world_mut(), document)
            .unwrap()
            .len(),
        2
    );
    CanvasDocument::remove_shape(app.world_mut(), document, shape).unwrap();
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&handle)
            .is_none()
    );
    assert!(!app.world().entities().contains(fills[0]));
    assert!(!app.world().entities().contains(fills[1]));
}

#[test]
fn ellipse_views_share_geometry_update_parameters_and_reclaim_only_their_own_assets() {
    use bevy::camera::primitives::Aabb;
    let (mut app, document, rectangle, a) = fixture(1.0);
    let ellipse = CanvasEllipse {
        radii: Vec2::new(30.0, 15.0),
    };
    let paint = CanvasPaint {
        fill: Some(Color::srgb(0.0, 1.0, 0.0)),
        stroke: Some(CanvasStroke {
            color: Color::WHITE,
            width: 6.0,
            ..Default::default()
        }),
        ..Default::default()
    };
    let shape = CanvasDocument::create_ellipse(
        app.world_mut(),
        document,
        ellipse,
        Transform::from_xyz(0.0, 0.0, -9000.0),
        paint,
    )
    .unwrap();
    let b = app.world_mut().spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } Node { width:px(200),height:px(200) } }).unwrap().id();
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(rectangle)).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    assert_eq!(
        CanvasView::single_selection(app.world(), a).unwrap(),
        Some(shape)
    );
    CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
    app.update();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 4);
    let fills = app
        .world_mut()
        .query::<(Entity, &Name, &Mesh2d, &MeshMaterial2d<ColorMaterial>)>()
        .iter(app.world())
        .filter(|(_, name, _, material)| {
            name.as_str() == "CanvasFill"
                && app
                    .world()
                    .resource::<Assets<ColorMaterial>>()
                    .get(&material.0)
                    .unwrap()
                    .color
                    == paint.fill.unwrap()
        })
        .map(|(entity, _, mesh, _)| (entity, mesh.0.clone()))
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0].1, fills[1].1);
    let before = positions(app.world(), fills[0].0);
    assert!(before.len() > 8);
    for point in &before {
        let normalized = Vec2::new(point[0], point[1]) / ellipse.radii;
        assert!(normalized.length_squared() <= 1.001);
    }
    let mut reader = app
        .world()
        .resource::<Messages<AssetEvent<Mesh>>>()
        .get_cursor_current();
    let transform = Transform::from_xyz(-10.0, 20.0, 9000.0)
        .with_rotation(Quat::from_rotation_z(0.5))
        .with_scale(Vec3::new(-2.0, 0.5, 0.0));
    CanvasDocument::update_ellipse(app.world_mut(), document, shape, ellipse, transform, paint)
        .unwrap();
    app.update();
    assert!(
        !reader
            .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
            .any(|event| matches!(event,AssetEvent::Modified { id } if *id==fills[0].1.id()))
    );
    assert_eq!(positions(app.world(), fills[0].0), before);
    CanvasDocument::update_ellipse(
        app.world_mut(),
        document,
        shape,
        CanvasEllipse {
            radii: Vec2::new(45.0, 10.0),
        },
        transform,
        paint,
    )
    .unwrap();
    app.update();
    assert_ne!(positions(app.world(), fills[0].0), before);
    assert_eq!(app.world().get::<Mesh2d>(fills[0].0).unwrap().0, fills[0].1);
    assert_eq!(app.world().get::<Mesh2d>(fills[1].0).unwrap().0, fills[0].1);
    for (entity, _) in &fills {
        let aabb = app.world().get::<Aabb>(*entity).unwrap();
        for point in positions(app.world(), *entity) {
            assert!(
                (Vec3::from_array(point) - Vec3::from(aabb.center))
                    .abs()
                    .cmple(Vec3::from(aabb.half_extents) + Vec3::splat(0.001))
                    .all()
            );
        }
    }
    assert!(
        CanvasDocument::reorder_shape(app.world_mut(), document, shape, CanvasOrder::Back).unwrap()
    );
    app.update();
    for (entity, _) in &fills {
        assert_eq!(
            app.world()
                .get::<GlobalTransform>(*entity)
                .unwrap()
                .translation()
                .z,
            0.0
        );
    }
    CanvasDocument::remove_shape(app.world_mut(), document, shape).unwrap();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    assert!(app.world().entities().contains(rectangle));
    assert_eq!(
        CanvasDocument::shape_order(app.world_mut(), document).unwrap(),
        [rectangle]
    );
    for view in [a, b] {
        assert_eq!(
            CanvasView::document(app.world(), view).unwrap(),
            Some(document)
        );
    }
}

#[test]
fn layout_maps_document_units_to_logical_pixels_with_independent_target() {
    for scale in [1.0, 1.5, 2.0] {
        let (mut app, _, shape, view) = fixture(scale);
        let camera = app
            .world()
            .get::<ViewportNode>(view)
            .unwrap()
            .camera
            .unwrap();
        assert!(app.world().get::<Camera>(camera).unwrap().is_active);
        assert_eq!(*app.world().get::<Msaa>(camera).unwrap(), Msaa::Sample4);
        let image = app
            .world()
            .get::<RenderTarget>(camera)
            .unwrap()
            .as_image()
            .unwrap();
        assert_eq!(
            app.world()
                .resource::<Assets<Image>>()
                .get(image)
                .unwrap()
                .size(),
            Vec2::new(300.0, 200.0)
                .mul_add(Vec2::splat(scale), Vec2::ZERO)
                .as_uvec2()
        );
        let Projection::Orthographic(projection) = app.world().get::<Projection>(camera).unwrap()
        else {
            panic!("orthographic required");
        };
        assert!(
            matches!(projection.scaling_mode, ScalingMode::Fixed { width, height } if width == 300.0 && height == 200.0)
        );
        let clip = app
            .world()
            .get::<Camera>(camera)
            .unwrap()
            .computed
            .clip_from_view;
        assert!((clip.x_axis.x - 2.0 / 300.0).abs() < 1e-6);
        assert!((clip.y_axis.y - 2.0 / 200.0).abs() < 1e-6);
        let proxy = named(app.world_mut(), "CanvasShapeProxy");
        let original = *app.world().get::<Transform>(shape).unwrap();
        assert_eq!(*app.world().get::<Transform>(proxy).unwrap(), original);
        assert_eq!(
            app.world()
                .get::<GlobalTransform>(proxy)
                .unwrap()
                .translation()
                .truncate(),
            Vec2::new(-20.0, -30.0)
        );
        app.world_mut().get_mut::<Node>(view).unwrap().width = px(400);
        app.update();
        let clip = app
            .world()
            .get::<Camera>(camera)
            .unwrap()
            .computed
            .clip_from_view;
        assert!((clip.x_axis.x - 2.0 / 400.0).abs() < 1e-6);
        assert_eq!(*app.world().get::<Transform>(shape).unwrap(), original);
        app.world_mut()
            .entity_mut(view)
            .insert(bevy::ui::LayoutConfig {
                use_rounding: false,
            });
        app.world_mut().get_mut::<Node>(view).unwrap().width = px(300.25);
        app.update();
        let node = app.world().get::<ComputedNode>(view).unwrap();
        let logical_width = node.size().x * node.inverse_scale_factor();
        let clip = app
            .world()
            .get::<Camera>(camera)
            .unwrap()
            .computed
            .clip_from_view;
        assert!((clip.x_axis.x - 2.0 / logical_width).abs() < 1e-6);
    }
}

#[test]
fn zoom_changes_projection_without_rebuilding_geometry_across_layout_and_scale_changes() {
    let (mut app, document, shape, view) = fixture(1.0);
    let ui_camera = app
        .world()
        .get::<ComputedUiTargetCamera>(view)
        .unwrap()
        .get()
        .unwrap();
    let other = app
        .world_mut()
        .spawn_scene(bsn! {
            @CanvasView { @document: {Some(document)}, @zoom: 0.5 }
            template(move |_| Ok(UiTargetCamera(ui_camera))) Node { width:px(200),height:px(100) }
        })
        .unwrap()
        .id();
    let camera = app
        .world()
        .get::<ViewportNode>(view)
        .unwrap()
        .camera
        .unwrap();
    let fill = named(app.world_mut(), "CanvasFill");
    let handle = app.world().get::<Mesh2d>(fill).unwrap().0.clone();
    let mesh = positions(app.world(), fill);
    let original = *app.world().get::<Transform>(shape).unwrap();
    CanvasView::set_center_in_world(app.world_mut(), view, Vec2::new(-70.0, 30.0)).unwrap();
    for (scale, zoom, width) in [
        (1.0, 2.0, 300.0),
        (1.5, 0.1, 360.0),
        (2.0, 10.0, 400.0),
        (1.0, 1.0, 300.0),
    ] {
        app.world_mut()
            .get_mut::<Camera>(ui_camera)
            .unwrap()
            .computed
            .target_info
            .as_mut()
            .unwrap()
            .scale_factor = scale;
        app.world_mut().get_mut::<Node>(view).unwrap().width = px(width);
        CanvasView::set_zoom_in_world(app.world_mut(), view, zoom).unwrap();
        app.update();
        let clip = app
            .world()
            .get::<Camera>(camera)
            .unwrap()
            .computed
            .clip_from_view;
        assert!((clip.x_axis.x - 2.0 * zoom / width).abs() < 1e-6);
        assert!((clip.y_axis.y - 2.0 * zoom / 200.0).abs() < 1e-6);
        let image = app
            .world()
            .get::<RenderTarget>(camera)
            .unwrap()
            .as_image()
            .unwrap();
        assert_eq!(
            app.world()
                .resource::<Assets<Image>>()
                .get(image)
                .unwrap()
                .size(),
            Vec2::new(width, 200.0)
                .mul_add(Vec2::splat(scale), Vec2::ZERO)
                .as_uvec2()
        );
        assert_eq!(
            CanvasView::center(app.world(), view).unwrap(),
            Vec2::new(-70.0, 30.0)
        );
        assert_eq!(CanvasView::zoom(app.world(), other).unwrap(), 0.5);
        assert_eq!(app.world().get::<Mesh2d>(fill).unwrap().0, handle);
        assert_eq!(positions(app.world(), fill), mesh);
        assert_eq!(*app.world().get::<Transform>(shape).unwrap(), original);
    }
}

#[test]
fn selection_outline_tracks_rotated_geometry_in_each_view_and_owns_only_its_assets() {
    let (mut app, document, shape, view) = fixture(1.0);
    let fill = named(app.world_mut(), "CanvasFill");
    let fill_mesh = positions(app.world(), fill);
    let paint = *app.world().get::<CanvasPaint>(shape).unwrap();
    let transform = *app.world().get::<Transform>(shape).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
    app.update();
    let outline = named(app.world_mut(), "CanvasSelectionOutline");
    let outline_handle = app.world().get::<Mesh2d>(outline).unwrap().0.clone();
    let content = app.world().get::<ChildOf>(outline).unwrap().parent();
    assert_ne!(content, document);
    assert_eq!(
        app.world().get::<ChildOf>(shape).unwrap().parent(),
        document
    );
    for zoom in [1.0, 2.0, 0.5] {
        CanvasView::set_zoom_in_world(app.world_mut(), view, zoom).unwrap();
        app.update();
        let vertices = positions(app.world(), outline);
        assert!(vertices.len() >= 57);
        for i in 0..8 {
            let square = &vertices[8 + i * 4..12 + i * 4];
            assert!(((square[1][0] - square[0][0]) * zoom - 8.0).abs() < 0.001);
            assert!(((square[3][1] - square[0][1]) * zoom - 8.0).abs() < 0.001);
        }
        assert!(((vertices[45][0] - vertices[44][0]) * zoom - 4.0).abs() < 0.001);
        let edge = Vec2::from_array([
            vertices[1][0] - vertices[0][0],
            vertices[1][1] - vertices[0][1],
        ])
        .normalize();
        let rotated = (transform.rotation * Vec3::X).truncate();
        assert!(edge.dot(rotated).abs() > 0.999);
        let normal = Vec2::new(-edge.y, edge.x);
        let width = Vec2::new(
            vertices[0][0] - vertices[4][0],
            vertices[0][1] - vertices[4][1],
        )
        .dot(normal)
        .abs()
            * zoom;
        assert!((width - 2.0).abs() < 0.001);
        assert_eq!(
            app.world().get::<Mesh2d>(outline).unwrap().0,
            outline_handle
        );
        assert_eq!(positions(app.world(), fill), fill_mesh);
        assert_eq!(*app.world().get::<CanvasPaint>(shape).unwrap(), paint);
        assert_eq!(*app.world().get::<Transform>(shape).unwrap(), transform);
    }
    CanvasView::set_selection_in_world(app.world_mut(), view, None).unwrap();
    app.update();
    assert!(!app.world().entities().contains(outline));
    assert!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&outline_handle)
            .is_none()
    );
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
    app.update();
    app.world_mut().despawn(view);
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
}

#[test]
fn selected_views_own_separate_outlines_and_repair_callbacks_can_destroy_the_view() {
    let (mut app, document, shape, a) = fixture(1.0);
    let ui_camera = app
        .world()
        .get::<ComputedUiTargetCamera>(a)
        .unwrap()
        .get()
        .unwrap();
    let b = app
        .world_mut()
        .spawn_scene(bsn! {
            @CanvasView { @document:{Some(document)}, @zoom:0.5 }
            template(move |_|Ok(UiTargetCamera(ui_camera))) Node {width:px(200),height:px(100)}
        })
        .unwrap()
        .id();
    for view in [a, b] {
        CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
    }
    app.update();
    let outlines = app
        .world_mut()
        .query::<(Entity, &Name, &Mesh2d)>()
        .iter(app.world())
        .filter(|(_, name, _)| name.as_str() == "CanvasSelectionOutline")
        .map(|(entity, _, mesh)| (entity, mesh.0.clone()))
        .collect::<Vec<_>>();
    assert_eq!(outlines.len(), 2);
    assert_ne!(outlines[0].1, outlines[1].1);
    app.world_mut().entity_mut(a).observe(
        |e: On<CanvasSelectionChanged>, mut commands: Commands| {
            if e.selection.is_empty() {
                commands.entity(e.entity).despawn();
            }
        },
    );
    CanvasDocument::remove_shape(app.world_mut(), document, shape).unwrap();
    assert_eq!(CanvasView::single_selection(app.world(), a).unwrap(), None);
    app.update();
    assert!(!app.world().entities().contains(a));
    assert_eq!(CanvasView::single_selection(app.world(), b).unwrap(), None);
    assert!(app.world().entities().contains(document));
    for (entity, mesh) in outlines {
        assert!(!app.world().entities().contains(entity));
        assert!(app.world().resource::<Assets<Mesh>>().get(&mesh).is_none());
    }
}

#[test]
fn geometry_reuses_assets_for_transform_and_color_but_updates_size_and_stroke() {
    let (mut app, _, shape, _) = fixture(1.0);
    let fill = named(app.world_mut(), "CanvasFill");
    let stroke = named(app.world_mut(), "CanvasStroke");
    let fill_handle = app.world().get::<Mesh2d>(fill).unwrap().0.clone();
    let original_fill = positions(app.world(), fill);
    assert_eq!(
        original_fill,
        vec![
            [-60.0, -40.0, 0.0],
            [60.0, -40.0, 0.0],
            [60.0, 40.0, 0.0],
            [-60.0, 40.0, 0.0]
        ]
    );
    let outline = positions(app.world(), stroke);
    assert_eq!(outline[0], [-65.0, -45.0, 0.0]);
    assert_eq!(outline[4], [-55.0, -35.0, 0.0]);
    assert_eq!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&fill_handle)
            .unwrap()
            .indices()
            .unwrap()
            .len(),
        6
    );
    app.world_mut()
        .get_mut::<Transform>(shape)
        .unwrap()
        .translation
        .x += 10.0;
    app.world_mut().get_mut::<CanvasPaint>(shape).unwrap().fill = None;
    app.update();
    assert_eq!(positions(app.world(), fill), original_fill);
    let proxy = named(app.world_mut(), "CanvasShapeProxy");
    assert_eq!(
        app.world()
            .get::<GlobalTransform>(proxy)
            .unwrap()
            .translation()
            .x,
        -10.0
    );
    assert_eq!(app.world().get::<Mesh2d>(fill).unwrap().0, fill_handle);
    assert_eq!(
        *app.world().get::<Visibility>(fill).unwrap(),
        Visibility::Hidden
    );
    app.world_mut()
        .get_mut::<CanvasGeometry>(shape)
        .map(|mut geometry| {
            let mut rectangle = *geometry.rectangle().unwrap();
            rectangle.size = Vec2::new(40.0, 20.0);
            *geometry = CanvasGeometry::Rectangle(rectangle);
        })
        .unwrap();
    app.world_mut()
        .get_mut::<CanvasPaint>(shape)
        .unwrap()
        .stroke
        .as_mut()
        .unwrap()
        .width = 6.0;
    app.update();
    assert_eq!(positions(app.world(), fill)[0], [-20.0, -10.0, 0.0]);
    assert_eq!(positions(app.world(), stroke)[0], [-23.0, -13.0, 0.0]);
    assert_eq!(positions(app.world(), stroke)[4], [-17.0, -7.0, 0.0]);
    assert_eq!(app.world().get::<Mesh2d>(fill).unwrap().0, fill_handle);
    app.world_mut()
        .get_mut::<CanvasPaint>(shape)
        .unwrap()
        .stroke
        .as_mut()
        .unwrap()
        .color = Color::srgba(0.0, 0.0, 1.0, 0.25);
    app.update();
    let material = &app
        .world()
        .get::<MeshMaterial2d<ColorMaterial>>(stroke)
        .unwrap()
        .0;
    assert_eq!(
        app.world()
            .resource::<Assets<ColorMaterial>>()
            .get(material)
            .unwrap()
            .alpha_mode,
        bevy::sprite_render::AlphaMode2d::Blend
    );
}

#[test]
fn hiding_and_despawning_view_releases_only_owned_render_resources() {
    let (mut app, document, shape, view) = fixture(1.0);
    let camera = app
        .world()
        .get::<ViewportNode>(view)
        .unwrap()
        .camera
        .unwrap();
    let image_count = app.world().resource::<Assets<Image>>().len();
    let entity_count = app.world().entities().count_spawned();
    app.world_mut().entity_mut(view).insert(Visibility::Hidden);
    app.update();
    assert!(!app.world().get::<Camera>(camera).unwrap().is_active);
    app.world_mut()
        .get_mut::<CanvasGeometry>(shape)
        .map(|mut geometry| {
            let mut rectangle = *geometry.rectangle().unwrap();
            rectangle.size = Vec2::splat(50.0);
            *geometry = CanvasGeometry::Rectangle(rectangle);
        })
        .unwrap();
    app.world_mut()
        .entity_mut(view)
        .insert(Visibility::Inherited);
    app.update();
    assert!(app.world().get::<Camera>(camera).unwrap().is_active);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(app.world().entities().count_spawned(), entity_count);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    app.world_mut().despawn(view);
    app.update();
    assert!(app.world().entities().contains(document));
    assert!(app.world().entities().contains(shape));
    assert!(app.world().get::<Camera>(camera).is_none());
    assert_eq!(
        app.world().resource::<Assets<Image>>().len(),
        image_count - 1
    );
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<ColorMaterial>>().len(), 0);
}

#[test]
fn document_api_commits_before_notifying_and_validates_queued_requests() {
    #[derive(Resource, Default)]
    struct Seen(Vec<Option<Entity>>);
    let mut app = scene_app();
    app.init_resource::<Seen>();
    app.set_error_handler(ErrorCapture::handler());
    let a = app.world_mut().spawn(CanvasDocument).id();
    let b = app.world_mut().spawn(CanvasDocument).id();
    let view = app
        .world_mut()
        .spawn_scene(bsn! { @CanvasView { @document: {Some(a)} } })
        .unwrap()
        .id();
    app.world_mut().add_observer(
        |event: On<CanvasDocumentChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                CanvasView::document(&world, event.entity).unwrap(),
                event.document
            );
            world.resource_mut::<Seen>().0.push(event.document);
        },
    );
    assert!(!CanvasView::set_document_in_world(app.world_mut(), view, Some(a)).unwrap());
    assert!(CanvasView::set_document_in_world(app.world_mut(), view, Some(b)).unwrap());
    CanvasView::set_document(&mut app.world_mut().commands(), view, None);
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Seen>().0, vec![Some(b), None]);
    app.world_mut().commands().entity(a).despawn();
    CanvasView::set_document(&mut app.world_mut().commands(), view, Some(a));
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert_eq!(CanvasView::document(app.world(), view).unwrap(), None);
    assert_eq!(app.world().resource::<Seen>().0, vec![Some(b), None]);
}

#[test]
fn planar_stroke_order_is_independent_of_model_z_scale() {
    let (mut app, _, shape, _) = fixture(1.0);
    let fill = named(app.world_mut(), "CanvasFill");
    let stroke = named(app.world_mut(), "CanvasStroke");
    let fill_positions = positions(app.world(), fill);
    for scale_z in [-1.0, 0.0, 2.0] {
        app.world_mut().get_mut::<Transform>(shape).unwrap().scale = Vec3::new(-1.0, -1.0, scale_z);
        app.update();
        let fill_z = app
            .world()
            .get::<GlobalTransform>(fill)
            .unwrap()
            .translation()
            .z;
        let stroke_z = app
            .world()
            .get::<GlobalTransform>(stroke)
            .unwrap()
            .translation()
            .z;
        assert!(stroke_z > fill_z);
        assert_eq!(positions(app.world(), fill), fill_positions);
        assert_eq!(
            app.world().get::<Transform>(shape).unwrap().scale.z,
            scale_z
        );
    }
}

#[test]
fn shared_document_views_reuse_geometry_and_rebind_after_document_removal() {
    let (mut app, document, shape, a) = fixture(1.0);
    let b = app.world_mut().spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } Node { width: px(280), height: px(180) } }).unwrap().id();
    app.update();
    app.update();
    let camera_a = app.world().get::<ViewportNode>(a).unwrap().camera.unwrap();
    let camera_b = app.world().get::<ViewportNode>(b).unwrap().camera.unwrap();
    assert_ne!(camera_a, camera_b);
    let image_a = app
        .world()
        .get::<RenderTarget>(camera_a)
        .unwrap()
        .as_image()
        .unwrap()
        .clone();
    let image_b = app
        .world()
        .get::<RenderTarget>(camera_b)
        .unwrap()
        .as_image()
        .unwrap()
        .clone();
    assert_ne!(image_a, image_b);
    let layers_a = app
        .world()
        .get::<bevy::camera::visibility::RenderLayers>(camera_a)
        .unwrap();
    let layers_b = app
        .world()
        .get::<bevy::camera::visibility::RenderLayers>(camera_b)
        .unwrap();
    assert!(!layers_a.intersects(layers_b));
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    let fills = app
        .world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "CanvasFill")
        .map(|(_, mesh)| mesh.0.clone())
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0], fills[1]);
    app.world_mut()
        .get_mut::<CanvasGeometry>(shape)
        .map(|mut geometry| {
            let mut rectangle = *geometry.rectangle().unwrap();
            rectangle.size = Vec2::new(90.0, 60.0);
            *geometry = CanvasGeometry::Rectangle(rectangle);
        })
        .unwrap();
    app.world_mut()
        .get_mut::<Transform>(shape)
        .unwrap()
        .translation
        .x = 25.0;
    app.update();
    let proxies = app
        .world_mut()
        .query::<(&Name, &Transform)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "CanvasShapeProxy")
        .map(|(_, transform)| transform.translation.x)
        .collect::<Vec<_>>();
    assert_eq!(proxies, vec![25.0, 25.0]);
    let fill_entities = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .filter(|(_, name)| name.as_str() == "CanvasFill")
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    for entity in fill_entities {
        assert_eq!(positions(app.world(), entity)[0], [-45.0, -30.0, 0.0]);
        assert_eq!(app.world().get::<Mesh2d>(entity).unwrap().0, fills[0]);
    }
    app.world_mut().despawn(a);
    app.update();
    assert!(app.world().entities().contains(shape));
    assert!(app.world().get::<Camera>(camera_a).is_none());
    assert!(app.world().get::<Camera>(camera_b).unwrap().is_active);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    assert!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&fills[0])
            .is_some()
    );
    let layout = app.world().get::<ComputedNode>(b).unwrap().size();
    app.world_mut().despawn(document);
    app.update();
    assert!(app.world().entities().contains(b));
    assert_eq!(app.world().get::<ComputedNode>(b).unwrap().size(), layout);
    assert_eq!(CanvasView::document(app.world(), b).unwrap(), None);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<ColorMaterial>>().len(), 0);
    assert_eq!(
        app.world().get::<ViewportNode>(b).unwrap().camera,
        Some(camera_b)
    );
    assert!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&image_b)
            .is_some()
    );
    let next = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_rectangle(
        app.world_mut(),
        next,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(25.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    CanvasView::set_document_in_world(app.world_mut(), b, Some(next)).unwrap();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    app.world_mut().entity_mut(b).insert(Visibility::Hidden);
    app.world_mut().despawn(next);
    app.update();
    assert_eq!(CanvasView::document(app.world(), b).unwrap(), None);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    app.world_mut().despawn(b);
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&image_b)
            .is_none()
    );
}

#[test]
fn removing_a_shape_reclaims_only_its_shared_meshes() {
    let (mut app, document, first, a) = fixture(1.0);
    let second = CanvasDocument::create_rectangle(
        app.world_mut(),
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let b = app.world_mut().spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } Node { width: px(200), height: px(200) } }).unwrap().id();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 4);
    CanvasDocument::remove_shape(app.world_mut(), document, first).unwrap();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    assert_eq!(app.world().resource::<Assets<ColorMaterial>>().len(), 4);
    assert!(app.world().entities().contains(second));
    CanvasDocument::remove_shape(app.world_mut(), document, second).unwrap();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<ColorMaterial>>().len(), 0);
    for view in [a, b] {
        assert_eq!(
            CanvasView::document(app.world(), view).unwrap(),
            Some(document)
        );
        let camera = app
            .world()
            .get::<ViewportNode>(view)
            .unwrap()
            .camera
            .unwrap();
        assert!(app.world().get::<Camera>(camera).unwrap().is_active);
    }
}

#[test]
fn document_repair_observers_can_despawn_views_without_stale_renderer_access() {
    let (mut app, document, _, a) = fixture(1.0);
    app.set_error_handler(ErrorCapture::handler());
    let b = app.world_mut().spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } Node { width: px(200), height: px(200) } }).unwrap().id();
    app.update();
    app.world_mut()
        .add_observer(|event: On<CanvasDocumentChanged>, mut commands: Commands| {
            if event.document.is_none() {
                commands.entity(event.entity).despawn();
            }
        });
    app.world_mut().despawn(document);
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(!app.world().entities().contains(a));
    assert!(!app.world().entities().contains(b));
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<ColorMaterial>>().len(), 0);
}

#[test]
fn hiding_rebuilding_and_unbinding_views_preserves_shared_geometry_ownership() {
    let (mut app, document, shape, a) = fixture(1.0);
    let b = app.world_mut().spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } Node { width: px(200), height: px(200) } }).unwrap().id();
    app.update();
    app.world_mut().entity_mut(a).insert(Visibility::Hidden);
    app.world_mut()
        .get_mut::<CanvasGeometry>(shape)
        .map(|mut geometry| {
            let mut rectangle = *geometry.rectangle().unwrap();
            rectangle.size = Vec2::splat(50.0);
            *geometry = CanvasGeometry::Rectangle(rectangle);
        })
        .unwrap();
    app.update();
    let cameras = [a, b].map(|view| {
        app.world()
            .get::<ViewportNode>(view)
            .unwrap()
            .camera
            .unwrap()
    });
    assert!(!app.world().get::<Camera>(cameras[0]).unwrap().is_active);
    assert!(app.world().get::<Camera>(cameras[1]).unwrap().is_active);
    app.world_mut().entity_mut(a).insert(Visibility::Inherited);
    app.update();
    let fills = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .filter(|(_, name)| name.as_str() == "CanvasFill")
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 2);
    for fill in &fills {
        assert_eq!(positions(app.world(), *fill)[0], [-25.0, -25.0, 0.0]);
    }
    let handle = app.world().get::<Mesh2d>(fills[0]).unwrap().0.clone();
    app.world_mut().despawn(a);
    app.update();
    let c = app.world_mut().spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } Node { width: px(180), height: px(160) } }).unwrap().id();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    for (_, mesh) in app
        .world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "CanvasFill")
    {
        assert_eq!(mesh.0, handle);
    }
    CanvasView::set_document_in_world(app.world_mut(), b, None).unwrap();
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&handle)
            .is_some()
    );
    CanvasView::set_document_in_world(app.world_mut(), c, None).unwrap();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<ColorMaterial>>().len(), 0);
    assert!(app.world().entities().contains(shape));
    CanvasView::set_document_in_world(app.world_mut(), c, Some(document)).unwrap();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
}
