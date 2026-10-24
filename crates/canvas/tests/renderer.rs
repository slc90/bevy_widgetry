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
            size: Vec2::new(120.0, 80.0),
        },
        Transform::from_xyz(-20.0, 30.0, 0.0).with_rotation(Quat::from_rotation_z(0.3)),
        CanvasPaint {
            fill: Some(Color::srgba(1.0, 0.0, 0.0, 0.5)),
            stroke: Some(CanvasStroke {
                color: Color::BLACK,
                width: 10.0,
            }),
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
        .get_mut::<CanvasRectangle>(shape)
        .unwrap()
        .size = Vec2::new(40.0, 20.0);
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
        .get_mut::<CanvasRectangle>(shape)
        .unwrap()
        .size = Vec2::splat(50.0);
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
