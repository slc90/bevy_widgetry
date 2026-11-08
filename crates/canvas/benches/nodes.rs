const PATH_SWEEP_PI_FACTOR: f32 = 1.5;
const PATH_RADIUS: f32 = 40.0;
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
    document: Entity,
    shape: Entity,
    pointer: PointerId,
    at: Location,
}
fn frame(app: &mut App) -> Result {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(error(format!("Canvas node benchmark errors: {errors:?}")))
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
    let path = CanvasPath {
        commands: (0..count)
            .map(|i| {
                let point = Vec2::from_angle(
                    i as f32 / count as f32 * std::f32::consts::PI * PATH_SWEEP_PI_FACTOR,
                ) * PATH_RADIUS;
                if i == 0 {
                    CanvasPathCommand::MoveTo(point)
                } else {
                    CanvasPathCommand::LineTo(point)
                }
            })
            .collect(),
    };
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        path,
        Transform::default(),
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                width: 2.0,
                ..default()
            }),
            ..default()
        },
    )
    .map_err(|error| error.into_bevy_error())?;
    let view=app.world_mut().spawn_scene(bsn!{@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(VIEW_MARGIN),top:px(VIEW_MARGIN),width:px(VIEW_SIZE.x),height:px(VIEW_SIZE.y)}}).map_err(BevyError::error)?.id();
    app.world_mut().spawn_scene(bsn!{@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(SECONDARY_VIEW_LEFT),top:px(VIEW_MARGIN),width:px(VIEW_SIZE.x),height:px(VIEW_SIZE.y)}}).map_err(BevyError::error)?;
    CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape))
        .map_err(|error| error.into_bevy_error())?;
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::PathNodeEdit)
        .map_err(|error| error.into_bevy_error())?;
    let pointer = pointer_ids()[1];
    app.world_mut().spawn(pointer);
    frame(&mut app)?;
    frame(&mut app)?;
    let center = app
        .world()
        .get::<UiGlobalTransform>(view)
        .ok_or_else(|| error("Canvas node layout missing"))?
        .translation;
    let target = bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
        .normalize(None)
        .ok_or_else(|| error("Canvas node target missing"))?;
    let at = Location {
        target,
        position: center,
    };
    let mut start = at.clone();
    start.position += Vec2::new(40.0, 0.0);
    queue_pointer(
        &mut app,
        pointer,
        start.clone(),
        PointerAction::Move { delta: Vec2::ONE },
    );
    frame(&mut app)?;
    queue_pointer(
        &mut app,
        pointer,
        start,
        PointerAction::Press(PointerButton::Primary),
    );
    frame(&mut app)?;
    if !app
        .world()
        .resource::<PointerCaptureMap>()
        .is_captured(&pointer)
    {
        return Err(error("Canvas node must capture the real pointer"));
    }
    Ok(Fixture {
        app,
        view,
        document,
        shape,
        pointer,
        at,
    })
}
fn main() -> Result {
    let mut harness = Harness::new("canvas-nodes-criterion")?;
    let entity_counts = [100, 1000];
    for count in entity_counts {
        run(
            &mut harness,
            &format!("canvas_nodes/views2/commands{count}/drag_anchor_mesh_and_frame"),
            false,
            || fixture(count),
            |f, i| {
                let point = Vec2::new(40.0, if i % 2 == 0 { 2.0 } else { 4.0 });
                let mut at = f.at.clone();
                at.position += point;
                queue_pointer(
                    &mut f.app,
                    f.pointer,
                    at,
                    PointerAction::Move {
                        delta: Vec2::new(0.0, 2.0),
                    },
                );
                frame(&mut f.app)?;
                let Some(CanvasGeometry::Path(path)) = f.app.world().get::<CanvasGeometry>(f.shape)
                else {
                    return Err(error("Node benchmark Path missing"));
                };
                if path.commands[0] != CanvasPathCommand::MoveTo(point)
                    || path.commands.len() != count
                    || f.app.world().resource::<Assets<Mesh>>().len() != 3
                    || CanvasView::single_selection(f.app.world(), f.view)
                        .map_err(|error| error.into_bevy_error())?
                        != Some(f.shape)
                    || CanvasDocument::shape_order(f.app.world_mut(), f.document)
                        .map_err(|error| error.into_bevy_error())?
                        != [f.shape]
                {
                    return Err(error(
                        "Node samples must update the anchor and keep bounded shared assets",
                    ));
                }
                Ok(())
            },
            |f| Ok(f.app.world().entities().count_spawned()),
        )?;
        run(
            &mut harness,
            &format!("canvas_nodes/views2/commands{count}/convert_segment_mesh_and_frame"),
            false,
            || fixture(count),
            |f, i| {
                let index = count / 2;
                let Some(CanvasGeometry::Path(path)) = f.app.world().get::<CanvasGeometry>(f.shape)
                else {
                    return Err(error("Topology benchmark Path missing"));
                };
                let to = match path.commands[index] {
                    CanvasPathCommand::LineTo(p) | CanvasPathCommand::QuadTo { to: p, .. } => p,
                    _ => return Err(error("Topology sample segment missing")),
                };
                let CanvasPathCommand::LineTo(from) = path.commands[index - 1] else {
                    return Err(error("Topology sample previous point missing"));
                };
                let segment = if i % 2 == 0 {
                    CanvasPathSegment::Quadratic {
                        control: (from + to) * 0.5,
                    }
                } else {
                    CanvasPathSegment::Line
                };
                if !CanvasDocument::set_path_segment(
                    f.app.world_mut(),
                    f.document,
                    f.shape,
                    index,
                    segment,
                )
                .map_err(|error| error.into_bevy_error())?
                {
                    return Err(error("Topology benchmark must perform a real conversion"));
                }
                frame(&mut f.app)?;
                let Some(CanvasGeometry::Path(path)) = f.app.world().get::<CanvasGeometry>(f.shape)
                else {
                    return Err(error("Topology benchmark Path disappeared"));
                };
                if path.commands.len() != count
                    || !matches!(
                        (path.commands[index], segment),
                        (
                            CanvasPathCommand::QuadTo { .. },
                            CanvasPathSegment::Quadratic { .. }
                        ) | (CanvasPathCommand::LineTo(_), CanvasPathSegment::Line)
                    )
                    || f.app
                        .world()
                        .resource::<PointerCaptureMap>()
                        .is_captured(&f.pointer)
                    || f.app.world().resource::<Assets<Mesh>>().len() != 3
                {
                    return Err(error(
                        "Topology conversion must refresh bounded node geometry and release stale capture",
                    ));
                }
                Ok(())
            },
            |f| Ok(f.app.world().entities().count_spawned()),
        )?;
    }
    harness.finish()
}
