const GRID_COLUMNS: usize = 20;
const GRID_SPACING: f32 = 30.0;
const BENCH_VIEWPORT: UVec2 = UVec2::new(800, 600);
const VIEW_MARGIN: f32 = 40.0;
const VIEW_SIZE: Vec2 = Vec2::new(200.0, 160.0);
const SECONDARY_VIEW_LEFT: f32 = 300.0;

use bevy::{picking::pointer::*, prelude::*, ui::UiGlobalTransform};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, run};
use bevy_widgetry_test_utils::{
    ErrorCapture, picking_app, pointer_ids, queue_pointer, spawn_picking_camera,
};

struct Fixture {
    app: App,
    pointer: PointerId,
    at: Location,
    count: usize,
    shape: Option<Entity>,
}

fn frame(app: &mut App) -> Result {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(error(format!("drawing frame errors: {errors:?}")))
    }
}

fn fixture(count: usize, mode: &str) -> Result<Fixture> {
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
    let pointer = pointer_ids()[1];
    app.world_mut().spawn(pointer);
    let document = app.world_mut().spawn(CanvasDocument).id();
    let mut selected = None;
    for i in 0..count {
        let editing = mode != "ellipse_preview" && i + 1 == count;
        let transform = if editing {
            Transform::default()
        } else {
            Transform::from_xyz(
                (i % GRID_COLUMNS) as f32 * GRID_SPACING,
                (i / GRID_COLUMNS) as f32 * GRID_SPACING,
                0.0,
            )
        };
        let shape = if editing && mode == "ellipse_parameters" {
            CanvasDocument::create_ellipse(
                app.world_mut(),
                document,
                CanvasEllipse {
                    radii: Vec2::new(40.0, 30.0),
                },
                transform,
                CanvasPaint::default(),
            )
            .map_err(|error| error.into_bevy_error())?
        } else {
            CanvasDocument::create_rectangle(
                app.world_mut(),
                document,
                CanvasRectangle {
                    corner_radius: if editing { 10.0 } else { 0.0 },
                    size: if editing {
                        Vec2::new(80.0, 60.0)
                    } else {
                        Vec2::splat(20.0)
                    },
                },
                transform,
                CanvasPaint::default(),
            )
            .map_err(|error| error.into_bevy_error())?
        };
        if editing {
            selected = Some(shape);
        }
    }
    let view=app.world_mut().spawn_scene(bsn! {@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(VIEW_MARGIN),top:px(VIEW_MARGIN),width:px(VIEW_SIZE.x),height:px(VIEW_SIZE.y)}}).map_err(BevyError::error)?.id();
    let other=app.world_mut().spawn_scene(bsn! {@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(SECONDARY_VIEW_LEFT),top:px(VIEW_MARGIN),width:px(VIEW_SIZE.x),height:px(VIEW_SIZE.y)}}).map_err(BevyError::error)?.id();
    frame(&mut app)?;
    frame(&mut app)?;
    if mode == "ellipse_preview" {
        CanvasView::set_mode_in_world(
            app.world_mut(),
            view,
            CanvasMode::Ellipse(CanvasPaint {
                stroke: Some(CanvasStroke {
                    width: 3.0,
                    ..default()
                }),
                ..default()
            }),
        )
        .map_err(|error| error.into_bevy_error())?;
    } else {
        CanvasView::set_selection_in_world(app.world_mut(), view, selected)
            .map_err(|error| error.into_bevy_error())?;
        CanvasView::set_selection_in_world(app.world_mut(), other, selected)
            .map_err(|error| error.into_bevy_error())?;
        frame(&mut app)?;
    }
    let position = app
        .world()
        .get::<UiGlobalTransform>(view)
        .ok_or_else(|| error("Canvas drawing view layout missing"))?
        .translation;
    let target = bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
        .normalize(None)
        .ok_or_else(|| error("Canvas drawing target missing"))?;
    let at = Location {
        target,
        position: position
            + if mode == "ellipse_preview" {
                Vec2::ZERO
            } else {
                Vec2::new(28.8, 21.6)
            },
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
    Ok(Fixture {
        app,
        pointer,
        at,
        count,
        shape: selected,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("canvas-drawing-criterion")?;
    let entity_counts = [100, 1000];
    for count in entity_counts {
        for mode in [
            "ellipse_preview",
            "rectangle_parameters",
            "ellipse_parameters",
        ] {
            run(
                &mut harness,
                &format!("canvas_drawing/views2/shapes{count}/{mode}_pointer_move_and_frame"),
                false,
                || fixture(count, mode),
                |f, i| {
                    let mut at = f.at.clone();
                    at.position += if i % 2 == 0 {
                        Vec2::new(60.0, 40.0)
                    } else {
                        Vec2::new(70.0, 30.0)
                    };
                    queue_pointer(
                        &mut f.app,
                        f.pointer,
                        at,
                        PointerAction::Move {
                            delta: Vec2::new(10.0, -10.0),
                        },
                    );
                    frame(&mut f.app)?;
                    if let Some(shape) = f.shape {
                        let size = match f.app.world().get::<CanvasGeometry>(shape) {
                            Some(CanvasGeometry::Rectangle(r)) => r.size,
                            Some(CanvasGeometry::Ellipse(e)) => e.radii * 2.0,
                            _ => return Err(error("Canvas parameter benchmark target missing")),
                        };
                        let expected = Vec2::new(80.0, 60.0)
                            + if i % 2 == 0 {
                                Vec2::new(60.0, 40.0)
                            } else {
                                Vec2::new(70.0, 30.0)
                            };
                        if size.distance(expected) > 0.001 {
                            return Err(error(
                                "Canvas parameter sample did not resize the typed geometry",
                            ));
                        }
                    }
                    if f.app.world().resource::<Assets<Mesh>>().len() != f.count * 2 + 2 {
                        return Err(error(
                            "Canvas draft meshes must remain view-local and bounded",
                        ));
                    }
                    Ok(())
                },
                |f| Ok(f.app.world().entities().count_spawned()),
            )?;
        }
    }
    harness.finish()
}
