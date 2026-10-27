use bevy::{picking::pointer::*, prelude::*, ui::UiGlobalTransform};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, run};
use bevy_widgetry_test_utils::{
    ErrorCapture, picking_app, pointer_ids, queue_pointer, spawn_picking_camera,
};

struct Fixture {
    app: App,
    view: Entity,
    other_view: Entity,
    bottom: Entity,
    top: Entity,
    location: Location,
    pointer: PointerId,
}

fn frame(app: &mut App) -> Result {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(error(format!("selection frame errors: {errors:?}")))
    }
}

fn fixture(count: usize) -> Result<Fixture> {
    use CanvasPathCommand::*;
    let mut app = picking_app();
    app.add_plugins((TransformPlugin, CanvasPlugin));
    app.set_error_handler(ErrorCapture::handler());
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(800, 600),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::new(800, 600), 1.0);
    let pointer = pointer_ids()[1];
    app.world_mut().spawn(pointer);
    let document = app.world_mut().spawn(CanvasDocument).id();
    let bottom = CanvasDocument::create_ellipse(
        app.world_mut(),
        document,
        CanvasEllipse {
            radii: Vec2::splat(10.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )?;
    let path = CanvasPath {
        commands: vec![
            MoveTo(Vec2::splat(-40.0)),
            LineTo(Vec2::new(40.0, -40.0)),
            LineTo(Vec2::splat(40.0)),
            LineTo(Vec2::new(-40.0, 40.0)),
            Close,
            MoveTo(Vec2::splat(-20.0)),
            LineTo(Vec2::new(20.0, -20.0)),
            LineTo(Vec2::splat(20.0)),
            LineTo(Vec2::new(-20.0, 20.0)),
            Close,
        ],
    };
    let mut top = bottom;
    for _ in 0..count {
        top = CanvasDocument::create_path(
            app.world_mut(),
            document,
            path.clone(),
            Transform::default(),
            CanvasPaint {
                fill_rule: CanvasFillRule::EvenOdd,
                ..default()
            },
        )?;
    }
    let view=app.world_mut().spawn_scene(bsn! { @CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(40),top:px(40),width:px(200),height:px(160)} }).map_err(BevyError::error)?.id();
    let other_view=app.world_mut().spawn_scene(bsn! { @CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {position_type:PositionType::Absolute,left:px(300),top:px(40),width:px(200),height:px(160)} }).map_err(BevyError::error)?.id();
    frame(&mut app)?;
    frame(&mut app)?;
    let center = app
        .world()
        .get::<UiGlobalTransform>(view)
        .ok_or_else(|| error("Canvas view layout missing"))?
        .translation;
    let target = bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
        .normalize(None)
        .ok_or_else(|| error("Canvas benchmark target missing"))?;
    Ok(Fixture {
        app,
        view,
        other_view,
        bottom,
        top,
        location: Location {
            target,
            position: center,
        },
        pointer,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("canvas-selection-criterion")?;
    for count in [100, 1000] {
        run(
            &mut harness,
            &format!(
                "canvas_selection/views2/paths{count}/alternating_hole_and_edge_click_and_frame"
            ),
            false,
            || fixture(count),
            |f, index| {
                let center = f
                    .app
                    .world()
                    .get::<UiGlobalTransform>(f.view)
                    .ok_or_else(|| error("Canvas benchmark view missing"))?
                    .translation;
                let hole = index % 2 == 0;
                let at = Location {
                    position: center
                        + if hole {
                            Vec2::ZERO
                        } else {
                            Vec2::new(-30.0, 0.0)
                        },
                    ..f.location.clone()
                };
                for action in [
                    PointerAction::Move { delta: Vec2::ONE },
                    PointerAction::Press(PointerButton::Primary),
                    PointerAction::Release(PointerButton::Primary),
                ] {
                    queue_pointer(&mut f.app, f.pointer, at.clone(), action);
                }
                frame(&mut f.app)?;
                if CanvasView::single_selection(f.app.world(), f.view)?
                    != Some(if hole { f.bottom } else { f.top })
                    || CanvasView::single_selection(f.app.world(), f.other_view)?.is_some()
                {
                    return Err(error(
                        "Canvas click must select the actual hit and preserve the other view",
                    ));
                }
                Ok(())
            },
            |f| Ok(f.app.world().entities().count_spawned()),
        )?;
        run(
            &mut harness,
            &format!("canvas_selection/views2/paths{count}/multi_ctrl_toggle_and_reference_frames"),
            false,
            || {
                let mut f = fixture(count)?;
                let document = CanvasView::document(f.app.world(), f.view)?
                    .ok_or_else(|| error("Multi selection document missing"))?;
                let shapes = CanvasDocument::shape_order(f.app.world_mut(), document)?;
                CanvasView::set_selection_in_world(f.app.world_mut(), f.view, shapes)?;
                CanvasView::set_selection_in_world(f.app.world_mut(), f.other_view, [f.bottom])?;
                f.app
                    .world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::ControlLeft);
                frame(&mut f.app)?;
                Ok(f)
            },
            |f, index| {
                let at = Location {
                    position: f.location.position + Vec2::new(-30.0, 0.0),
                    ..f.location.clone()
                };
                for action in [
                    PointerAction::Move { delta: Vec2::ONE },
                    PointerAction::Press(PointerButton::Primary),
                    PointerAction::Release(PointerButton::Primary),
                ] {
                    queue_pointer(&mut f.app, f.pointer, at.clone(), action);
                }
                frame(&mut f.app)?;
                let expected = count + usize::from(index % 2 == 1);
                let selection = CanvasView::selection(f.app.world(), f.view)?;
                if selection.len() != expected
                    || selection.contains(&f.top) != (index % 2 == 1)
                    || CanvasView::selection(f.app.world(), f.other_view)? != [f.bottom]
                    || f.app
                        .world()
                        .resource::<bevy::picking::hover::PointerCaptureMap>()
                        .is_captured(&f.pointer)
                    || f.app.world().resource::<Assets<Mesh>>().len() != 2 * (count + 1) + 2
                {
                    return Err(error(
                        "Ctrl sample must toggle one real shape and preserve private bounded selections",
                    ));
                }
                let outlines = f
                    .app
                    .world_mut()
                    .query::<(&Name,&Mesh2d)>()
                    .iter(f.app.world())
                    .filter_map(|(name,mesh)| {
                        (name.as_str() == "CanvasSelectionOutline")
                            .then_some(mesh.0.clone())
                    })
                    .collect::<Vec<_>>();
                let meshes = f.app.world().resource::<Assets<Mesh>>();
                if !outlines.iter().any(|h| {
                    meshes
                        .get(h)
                        .is_some_and(|m| m.count_vertices() == expected * 8)
                }) {
                    return Err(error(
                        "Ctrl sample must refresh the real multi selection frame mesh",
                    ));
                }
                Ok(())
            },
            |f| Ok(f.app.world().entities().count_spawned()),
        )?;
    }
    harness.finish()
}
