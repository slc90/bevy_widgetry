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
    pointer: PointerId,
    at: Location,
    count: usize,
}

fn frame(app: &mut App) -> Result {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(error(format!("Pen benchmark frame errors: {errors:?}")))
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
    let pointer = pointer_ids()[1];
    app.world_mut().spawn(pointer);
    let view=app.world_mut().spawn_scene(bsn!{@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(VIEW_MARGIN),top:px(VIEW_MARGIN),width:px(VIEW_SIZE.x),height:px(VIEW_SIZE.y)}}).map_err(BevyError::error)?.id();
    app.world_mut().spawn_scene(bsn!{@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(SECONDARY_VIEW_LEFT),top:px(VIEW_MARGIN),width:px(VIEW_SIZE.x),height:px(VIEW_SIZE.y)}}).map_err(BevyError::error)?;
    frame(&mut app)?;
    frame(&mut app)?;
    CanvasView::set_mode_in_world(
        app.world_mut(),
        view,
        CanvasMode::Pen(CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                width: 2.0,
                ..default()
            }),
            ..default()
        }),
    )
    .map_err(|error| error.into_bevy_error())?;
    let center = app
        .world()
        .get::<UiGlobalTransform>(view)
        .ok_or_else(|| error("Pen benchmark layout missing"))?
        .translation;
    let target = bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
        .normalize(None)
        .ok_or_else(|| error("Pen benchmark target missing"))?;
    let at = Location {
        target,
        position: center,
    };
    for i in 0..count {
        let mut point = at.clone();
        point.position +=
            Vec2::from_angle(i as f32 / count as f32 * std::f32::consts::PI * PATH_SWEEP_PI_FACTOR)
                * PATH_RADIUS;
        for action in [
            PointerAction::Move { delta: Vec2::ONE },
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            queue_pointer(&mut app, pointer, point.clone(), action);
        }
        frame(&mut app)?;
    }
    let mut start = at.clone();
    start.position += Vec2::new(8.0, -42.0);
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
        return Err(error("Pen benchmark point must be captured"));
    }
    Ok(Fixture {
        app,
        view,
        document,
        pointer,
        at,
        count,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("canvas-pen-criterion")?;
    let entity_counts = [100, 1000];
    for count in entity_counts {
        run(
            &mut harness,
            &format!("canvas_pen/views2/commands{count}/held_endpoint_preview_and_frame"),
            false,
            || fixture(count),
            |f, i| {
                let point = if i % 2 == 0 {
                    Vec2::new(10.0, -45.0)
                } else {
                    Vec2::new(12.0, -40.0)
                };
                let mut at = f.at.clone();
                at.position += point;
                queue_pointer(
                    &mut f.app,
                    f.pointer,
                    at,
                    PointerAction::Move {
                        delta: Vec2::new(2.0, 5.0),
                    },
                );
                frame(&mut f.app)?;
                let draft = CanvasView::pen_draft(f.app.world(), f.view)
                    .map_err(|error| error.into_bevy_error())?
                    .ok_or_else(|| error("Pen benchmark draft missing"))?;
                if draft.path.commands.len() != f.count
                    || !CanvasDocument::shape_order(f.app.world_mut(), f.document)
                        .map_err(|error| error.into_bevy_error())?
                        .is_empty()
                    || f.app.world().resource::<Assets<Mesh>>().len() != 3
                {
                    return Err(error("Pen samples must keep a private bounded draft"));
                }
                let preview = f
                    .app
                    .world_mut()
                    .query::<(Entity, &Name)>()
                    .iter(f.app.world())
                    .find(|(_, n)| n.as_str() == "CanvasPenPreview")
                    .map(|(e, _)| e)
                    .ok_or_else(|| error("Pen benchmark preview missing"))?;
                let node = f
                    .app
                    .world()
                    .get::<Children>(preview)
                    .and_then(|c| c.last())
                    .copied()
                    .ok_or_else(|| error("Pen benchmark markers missing"))?;
                let handle = &f
                    .app
                    .world()
                    .get::<Mesh2d>(node)
                    .ok_or_else(|| error("Pen benchmark marker mesh missing"))?
                    .0;
                let positions = f
                    .app
                    .world()
                    .resource::<Assets<Mesh>>()
                    .get(handle)
                    .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
                    .and_then(|a| a.as_float3())
                    .ok_or_else(|| error("Pen benchmark marker positions missing"))?;
                let last = positions
                    .iter()
                    .rev()
                    .take(4)
                    .map(|p| Vec2::new(p[0], p[1]))
                    .sum::<Vec2>()
                    * 0.25;
                if last.distance(point) > 0.001 {
                    return Err(error("Pen sample did not update the held endpoint preview"));
                }
                Ok(())
            },
            |f| Ok(f.app.world().entities().count_spawned()),
        )?;
    }
    harness.finish()
}
