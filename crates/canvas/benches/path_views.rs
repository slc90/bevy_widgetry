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
                    ..Default::default()
                }),
                ..Default::default()
            },
        )?);
    }
    let shape = shape.ok_or_else(|| error("requires paths"))?;
    for _ in 0..2 {
        let view=app.world_mut().spawn_scene(bsn!{@CanvasView {@document:{Some(document)}} template(move |_|Ok(UiTargetCamera(camera))) Node {width:px(350),height:px(250)}}).map_err(BevyError::error)?.id();
        CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape))?;
    }
    frame(&mut app)?;
    frame(&mut app)?;
    Ok(Fixture {
        app,
        document,
        shape,
        path,
        count,
    })
}
fn main() -> Result {
    let mut harness = Harness::new("canvas-path-criterion")?;
    for count in [100, 1000] {
        for mode in ["transform_only", "commands_update", "paint_update"] {
            run(
                &mut harness,
                &format!("canvas_path/views2/shapes{count}/{mode}_and_frame"),
                false,
                || fixture(count),
                |f, i| {
                    let shift = (i % 2) as f32;
                    let mut path = f.path.clone();
                    if mode == "commands_update" {
                        path.commands[1] =
                            CanvasPathCommand::LineTo(Vec2::new(21.0 + shift, -15.0));
                    }
                    if mode == "transform_only" {
                        CanvasDocument::update_transform(
                            f.app.world_mut(),
                            f.document,
                            f.shape,
                            Transform::from_xyz(shift, 0.0, 0.0)
                                .with_rotation(Quat::from_rotation_z(shift * 0.3))
                                .with_scale(if i % 2 == 0 {
                                    Vec3::new(-1.2, 0.7, 0.0)
                                } else {
                                    Vec3::new(0.9, 1.1, 0.0)
                                }),
                        )?;
                    } else {
                        CanvasDocument::update_path(
                            f.app.world_mut(),
                            f.document,
                            f.shape,
                            path,
                            Transform::from_xyz(shift, 0.0, 0.0),
                            CanvasPaint {
                                fill: Some(Color::WHITE),
                                fill_rule: if mode == "paint_update" && i % 2 == 0 {
                                    CanvasFillRule::EvenOdd
                                } else {
                                    CanvasFillRule::NonZero
                                },
                                stroke: Some(CanvasStroke {
                                    color: Color::BLACK,
                                    width: 3.0,
                                    cap: if mode == "paint_update" {
                                        if i % 2 == 0 {
                                            CanvasLineCap::Round
                                        } else {
                                            CanvasLineCap::Square
                                        }
                                    } else {
                                        CanvasLineCap::Butt
                                    },
                                    join: if mode == "paint_update" {
                                        if i % 2 == 0 {
                                            CanvasLineJoin::Round
                                        } else {
                                            CanvasLineJoin::Bevel
                                        }
                                    } else {
                                        CanvasLineJoin::Miter
                                    },
                                }),
                            },
                        )?;
                    }
                    frame(&mut f.app)?;
                    if f.app.world().resource::<Assets<Mesh>>().len() != f.count * 2 + 2 {
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
