const TEST_VIEWPORT: UVec2 = UVec2::new(1200, 800);
const VIEW_MARGIN: f32 = 40.0;
const PRIMARY_VIEW_SIZE: Vec2 = Vec2::new(200.0, 160.0);
const SECONDARY_VIEW_LEFT: f32 = 300.0;
const SECONDARY_VIEW_SIZE: Vec2 = Vec2::new(180.0, 150.0);

use bevy::{picking::pointer::*, prelude::*, ui::UiGlobalTransform};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{ErrorCapture, picking_app, queue_pointer, spawn_picking_camera};

pub fn update(app: &mut App) {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
}

pub fn fixture(
    id: PointerId,
    scale: f32,
    ui_scale: f32,
) -> (App, Entity, Entity, Entity, Entity, Location) {
    let mut app = picking_app();
    app.add_plugins((TransformPlugin, CanvasPlugin));
    app.set_error_handler(ErrorCapture::handler());
    app.insert_resource(UiScale(ui_scale));
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(TEST_VIEWPORT.x, TEST_VIEWPORT.y)
                    .with_scale_factor_override(scale),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, TEST_VIEWPORT, scale);
    app.world_mut()
        .get_mut::<Camera>(camera)
        .unwrap()
        .viewport
        .as_mut()
        .unwrap()
        .physical_position = UVec2::new(24, 32);
    let document = app.world_mut().spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_rectangle(
        app.world_mut(),
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(30.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let a = app.world_mut().spawn_scene(bsn! {
        @CanvasView { @document: {Some(document)} }
        Node { position_type: PositionType::Absolute, left: px(VIEW_MARGIN), top: px(VIEW_MARGIN), width: px(PRIMARY_VIEW_SIZE.x), height: px(PRIMARY_VIEW_SIZE.y) }
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    let b = app.world_mut().spawn_scene(bsn! {
        @CanvasView { @document: {Some(document)} }
        Node { position_type: PositionType::Absolute, left: px(SECONDARY_VIEW_LEFT), top: px(VIEW_MARGIN), width: px(SECONDARY_VIEW_SIZE.x), height: px(SECONDARY_VIEW_SIZE.y) }
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    update(&mut app);
    update(&mut app);
    let point = app.world().get::<UiGlobalTransform>(a).unwrap().translation;
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: (point + Vec2::new(24.0, 32.0)) / scale,
    };
    (app, a, b, shape, window, location)
}

pub fn input(app: &mut App, id: PointerId, at: &Location, action: PointerAction) {
    queue_pointer(app, id, at.clone(), action);
    update(app);
}

pub fn moved(app: &mut App, id: PointerId, at: &mut Location, delta: Vec2) {
    at.position += delta;
    input(app, id, at, PointerAction::Move { delta });
}

pub fn scroll(app: &mut App, id: PointerId, at: &Location, y: f32) {
    input(
        app,
        id,
        at,
        PointerAction::Scroll {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y,
            phase: bevy::input::touch::TouchPhase::Moved,
        },
    );
}
