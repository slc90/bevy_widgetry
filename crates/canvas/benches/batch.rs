const GRID_COLUMNS: usize = 50;
const GRID_SPACING: f32 = 14.0;
const BENCH_VIEWPORT: UVec2 = UVec2::new(800, 600);
const VIEW_MARGIN: f32 = 40.0;
const VIEW_SIZE: Vec2 = Vec2::new(200.0, 160.0);
const SECONDARY_VIEW_LEFT: f32 = 300.0;

use bevy::{
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
    ui::UiGlobalTransform,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, run};
use bevy_widgetry_test_utils::{
    ErrorCapture, picking_app, pointer_ids, queue_pointer, spawn_picking_camera,
};
struct Fixture {
    app: App,
    view: Entity,
    other: Entity,
    document: Entity,
    shapes: Vec<Entity>,
    pointer: PointerId,
    at: Location,
}
fn frame(app: &mut App) -> Result {
    let e = ErrorCapture::default();
    e.run(|| app.update());
    let errors = e.take();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(error(format!("batch frame errors: {errors:?}")))
    }
}
fn fixture(count: usize) -> Result<Fixture> {
    let mut app = picking_app();
    app.add_plugins((TransformPlugin, CanvasPlugin));
    app.set_error_handler(ErrorCapture::handler());
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(BENCH_VIEWPORT.x, BENCH_VIEWPORT.y),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, BENCH_VIEWPORT, 1.0);
    let document = app.world_mut().spawn(CanvasDocument).id();
    let mut shapes = Vec::new();
    for i in 0..count {
        let transform = Transform::from_xyz(
            (i % GRID_COLUMNS) as f32 * GRID_SPACING,
            (i / GRID_COLUMNS) as f32 * GRID_SPACING,
            0.0,
        );
        let shape = match i % 3 {
            0 => CanvasDocument::create_rectangle(
                app.world_mut(),
                document,
                CanvasRectangle {
                    size: Vec2::splat(10.0),
                    corner_radius: 0.0,
                },
                transform,
                CanvasPaint::default(),
            )
            .map_err(|error| error.into_bevy_error())?,
            1 => CanvasDocument::create_ellipse(
                app.world_mut(),
                document,
                CanvasEllipse {
                    radii: Vec2::new(5.0, 4.0),
                },
                transform,
                CanvasPaint::default(),
            )
            .map_err(|error| error.into_bevy_error())?,
            _ => CanvasDocument::create_path(
                app.world_mut(),
                document,
                CanvasPath {
                    commands: vec![
                        CanvasPathCommand::MoveTo(Vec2::splat(-5.0)),
                        CanvasPathCommand::LineTo(Vec2::new(5.0, -5.0)),
                        CanvasPathCommand::LineTo(Vec2::new(0.0, 5.0)),
                        CanvasPathCommand::Close,
                    ],
                },
                transform,
                CanvasPaint::default(),
            )
            .map_err(|error| error.into_bevy_error())?,
        };
        shapes.push(shape);
    }
    let view=app.world_mut().spawn_scene(bsn!{@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(VIEW_MARGIN),top:px(VIEW_MARGIN),width:px(VIEW_SIZE.x),height:px(VIEW_SIZE.y)}}).map_err(BevyError::error)?.id();
    let other=app.world_mut().spawn_scene(bsn!{@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(SECONDARY_VIEW_LEFT),top:px(VIEW_MARGIN),width:px(VIEW_SIZE.x),height:px(VIEW_SIZE.y)}}).map_err(BevyError::error)?.id();
    CanvasView::set_selection_in_world(app.world_mut(), view, shapes.clone())
        .map_err(|error| error.into_bevy_error())?;
    CanvasView::set_selection_in_world(app.world_mut(), other, [shapes[0]])
        .map_err(|error| error.into_bevy_error())?;
    let pointer = pointer_ids()[1];
    app.world_mut().spawn(pointer);
    frame(&mut app)?;
    frame(&mut app)?;
    let center = app
        .world()
        .get::<UiGlobalTransform>(view)
        .ok_or_else(|| error("batch layout missing"))?
        .translation;
    let target = bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
        .normalize(None)
        .ok_or_else(|| error("batch target missing"))?;
    let at = Location {
        target,
        position: center,
    };
    queue_pointer(
        &mut app,
        pointer,
        at.clone(),
        PointerAction::Move { delta: Vec2::ONE },
    );
    frame(&mut app)?;
    queue_pointer(
        &mut app,
        pointer,
        at.clone(),
        PointerAction::Press(PointerButton::Primary),
    );
    frame(&mut app)?;
    if !app
        .world()
        .resource::<PointerCaptureMap>()
        .is_captured(&pointer)
    {
        return Err(error("batch must capture the actual selected shape"));
    }
    Ok(Fixture {
        app,
        view,
        other,
        document,
        shapes,
        pointer,
        at,
    })
}
fn main() -> Result {
    let mut harness = Harness::new("canvas-batch-criterion")?;
    let entity_counts = [100, 1000];
    for count in entity_counts {
        run(
            &mut harness,
            &format!("canvas_batch/views2/shapes{count}/move_all_and_frame"),
            false,
            || fixture(count),
            |f, index| {
                let delta = if index % 2 == 0 {
                    Vec2::new(3.0, 2.0)
                } else {
                    Vec2::new(6.0, 4.0)
                };
                let at = Location {
                    position: f.at.position + delta,
                    ..f.at.clone()
                };
                queue_pointer(
                    &mut f.app,
                    f.pointer,
                    at,
                    PointerAction::Move { delta: Vec2::ONE },
                );
                frame(&mut f.app)?;
                for (i, shape) in f.shapes.iter().enumerate() {
                    let expected = Vec2::new(
                        (i % GRID_COLUMNS) as f32 * GRID_SPACING,
                        (i / GRID_COLUMNS) as f32 * GRID_SPACING,
                    ) + delta;
                    if f.app
                        .world()
                        .get::<Transform>(*shape)
                        .is_none_or(|t| t.translation.truncate() != expected)
                    {
                        return Err(error(
                            "each batch sample must move every shape by the same Document delta",
                        ));
                    }
                }
                if CanvasView::selection(f.app.world(), f.view)
                    .map_err(|error| error.into_bevy_error())?
                    .len()
                    != count
                    || CanvasView::selection(f.app.world(), f.other)
                        .map_err(|error| error.into_bevy_error())?
                        != [f.shapes[0]]
                    || CanvasDocument::shape_order(f.app.world_mut(), f.document)
                        .map_err(|error| error.into_bevy_error())?
                        != f.shapes
                    || f.app.world().resource::<Assets<Mesh>>().len() != 2 * count + 2
                {
                    return Err(error(
                        "batch samples must preserve selection, order and bounded shared geometry",
                    ));
                }
                Ok(())
            },
            |f| Ok(f.app.world().entities().count_spawned()),
        )?;
    }
    harness.finish()
}
