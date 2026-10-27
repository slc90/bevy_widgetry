use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, run};
use bevy_widgetry_test_utils::{ErrorCapture, add_ui_plugins, scene_app, spawn_ui_camera};

struct Fixture {
    app: App,
    document: Entity,
    shape: Entity,
    count: usize,
    radii: Vec2,
    stroke_width: f32,
    scale: Vec3,
}

fn frame(app: &mut App) -> Result {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(error(format!("ellipse benchmark ECS errors: {errors:?}")))
    }
}

fn fixture(count: usize, contour: &str) -> Result<Fixture> {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((TransformPlugin, CanvasPlugin));
    app.set_error_handler(ErrorCapture::handler());
    let camera = spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
    let document = app.world_mut().spawn(CanvasDocument).id();
    let (radii, stroke_width, scale) = match contour {
        "wide_eccentric" => (Vec2::new(20.0, 0.2), 20.0, Vec3::ONE),
        "thin_eccentric" => (Vec2::new(1.0, 1e8), 1.0, Vec3::new(60.0, 6e-7, 1.0)),
        _ => (Vec2::new(20.0, 12.0), 3.0, Vec3::ONE),
    };
    let mut shape = None;
    for index in 0..count {
        shape = Some(CanvasDocument::create_ellipse(
            app.world_mut(),
            document,
            CanvasEllipse { radii },
            Transform::from_xyz((index % 20) as f32 * 50.0, (index / 20) as f32 * 30.0, 0.0)
                .with_scale(scale),
            CanvasPaint {
                fill: Some(Color::WHITE),
                stroke: Some(CanvasStroke {
                    color: Color::BLACK,
                    width: stroke_width,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )?);
    }
    for _ in 0..2 {
        app.world_mut().spawn_scene(bsn! { @CanvasView { @document: {Some(document)} } template(move |_|Ok(UiTargetCamera(camera))) Node {width:px(350),height:px(250)} }).map_err(BevyError::error)?;
    }
    frame(&mut app)?;
    frame(&mut app)?;
    Ok(Fixture {
        app,
        document,
        shape: shape.ok_or_else(|| error("benchmark requires ellipses"))?,
        count,
        radii,
        stroke_width,
        scale,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("canvas-ellipse-criterion")?;
    for count in [100, 1000] {
        for contour in ["regular", "wide_eccentric", "thin_eccentric"] {
            for update_radii in [false, true] {
                let operation = if update_radii {
                    "radii_update"
                } else {
                    "transform_only"
                };
                run(
                    &mut harness,
                    &format!("canvas_ellipse/views2/shapes{count}/{contour}_{operation}_and_frame"),
                    false,
                    || fixture(count, contour),
                    |fixture, index| {
                        let shift = (index % 2) as f32;
                        let ellipse = CanvasEllipse {
                            radii: fixture.radii
                                + Vec2::X * if update_radii { 1.0 + shift } else { 0.0 },
                        };
                        CanvasDocument::update_ellipse(
                            fixture.app.world_mut(),
                            fixture.document,
                            fixture.shape,
                            ellipse,
                            Transform::from_xyz(shift, 0.0, 0.0).with_scale(fixture.scale),
                            CanvasPaint {
                                fill: Some(Color::WHITE),
                                stroke: Some(CanvasStroke {
                                    color: Color::BLACK,
                                    width: fixture.stroke_width,
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        )?;
                        frame(&mut fixture.app)?;
                        if fixture.app.world().resource::<Assets<Mesh>>().len() != fixture.count * 2
                        {
                            return Err(error(
                                "ellipse mesh resources must remain shared and bounded",
                            ));
                        }
                        Ok(())
                    },
                    |fixture| Ok(fixture.app.world().entities().count_spawned()),
                )?;
            }
        }
    }
    harness.finish()
}
