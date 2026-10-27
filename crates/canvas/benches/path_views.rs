use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, run};
use bevy_widgetry_test_utils::{ErrorCapture, add_ui_plugins, scene_app, spawn_ui_camera};

struct Fixture {
    app: App,
    document: Entity,
    shape: Entity,
    path: CanvasPath,
    count: usize,
}
fn frame(app: &mut App) -> Result {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(error(format!("path frame errors: {errors:?}")))
    }
}
fn fixture(count: usize) -> Result<Fixture> {
    use CanvasPathCommand::*;
    let path = CanvasPath {
        commands: vec![
            MoveTo(Vec2::new(-20.0, -15.0)),
            LineTo(Vec2::new(20.0, -15.0)),
            QuadTo {
                control: Vec2::new(30.0, 15.0),
                to: Vec2::new(0.0, 15.0),
            },
            Close,
            MoveTo(Vec2::new(-10.0, 25.0)),
            ArcTo {
                radii: Vec2::new(10.0, 5.0),
                rotation: 0.2,
                large_arc: false,
                sweep: true,
                to: Vec2::new(10.0, 25.0),
            },
        ],
    };
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((TransformPlugin, CanvasPlugin));
    app.set_error_handler(ErrorCapture::handler());
    let camera = spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
    let document = app.world_mut().spawn(CanvasDocument).id();
    let mut shape = None;
    for i in 0..count {
        shape = Some(CanvasDocument::create_path(
            app.world_mut(),
            document,
            path.clone(),
            Transform::from_xyz((i % 20) as f32 * 50.0, (i / 20) as f32 * 50.0, 0.0),
            CanvasPaint {
                fill: Some(Color::WHITE),
                stroke: Some(CanvasStroke {
                    color: Color::BLACK,
                    width: 3.0,
                }),
            },
        )?);
    }
    for _ in 0..2 {
        app.world_mut().spawn_scene(bsn!{@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {width:px(350),height:px(250)}}).map_err(BevyError::error)?;
    }
    frame(&mut app)?;
    frame(&mut app)?;
    Ok(Fixture {
        app,
        document,
        shape: shape.ok_or_else(|| error("requires paths"))?,
        path,
        count,
    })
}
fn main() -> Result {
    let mut harness = Harness::new("canvas-path-criterion")?;
    for count in [100, 1000] {
        for change in [false, true] {
            let mode = if change {
                "commands_update"
            } else {
                "transform_only"
            };
            run(
                &mut harness,
                &format!("canvas_path/views2/shapes{count}/{mode}_and_frame"),
                false,
                || fixture(count),
                |f, i| {
                    let shift = (i % 2) as f32;
                    let mut path = f.path.clone();
                    if change {
                        path.commands[1] =
                            CanvasPathCommand::LineTo(Vec2::new(21.0 + shift, -15.0));
                    }
                    CanvasDocument::update_path(
                        f.app.world_mut(),
                        f.document,
                        f.shape,
                        path,
                        Transform::from_xyz(shift, 0.0, 0.0),
                        CanvasPaint {
                            fill: Some(Color::WHITE),
                            stroke: Some(CanvasStroke {
                                color: Color::BLACK,
                                width: 3.0,
                            }),
                        },
                    )?;
                    frame(&mut f.app)?;
                    if f.app.world().resource::<Assets<Mesh>>().len() != f.count * 2 {
                        return Err(error("Path meshes must stay shared and bounded"));
                    }
                    Ok(())
                },
                |f| Ok(f.app.world().entities().count_spawned()),
            )?;
        }
    }
    harness.finish()
}
