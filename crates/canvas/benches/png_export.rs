use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        pipelined_rendering::PipelinedRenderingPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    window::ExitCondition,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, missing, run};
use std::time::{Duration, Instant};

#[derive(Resource, Default)]
struct Completion(Option<std::result::Result<UVec2, String>>);

struct Fixture {
    app: App,
    document: Entity,
    directory: tempfile::TempDir,
    assets: (usize, usize, usize),
}

fn assets(world: &World) -> (usize, usize, usize) {
    (
        world.resource::<Assets<Image>>().len(),
        world.resource::<Assets<Mesh>>().len(),
        world.resource::<Assets<ColorMaterial>>().len(),
    )
}

fn frame(app: &mut App) -> Result {
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(error(format!("PNG benchmark ECS errors: {errors:?}")))
    }
}

fn fixture() -> Result<Fixture> {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    backends: Some(Backends::DX12),
                    ..default()
                })),
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<PipelinedRenderingPlugin>(),
    )
    .add_plugins(CanvasPlugin)
    .init_resource::<Completion>()
    .add_observer(
        |event: On<CanvasPngExportFinished>, mut completion: ResMut<Completion>| {
            completion.0 = Some(event.result.as_ref().copied().map_err(ToString::to_string));
        },
    );
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.plugins_state() != bevy::app::PluginsState::Ready {
        if Instant::now() >= deadline {
            return Err(error("PNG benchmark DX12 initialization timed out"));
        }
        bevy::tasks::tick_global_task_pools_on_main_thread();
        std::thread::sleep(Duration::from_millis(1));
    }
    app.finish();
    app.cleanup();
    for _ in 0..4 {
        frame(&mut app)?;
    }
    let document = app.world_mut().spawn(CanvasDocument).id();
    for index in 0..64 {
        let transform = Transform::from_xyz(
            (index % 8) as f32 * 256.0 + 128.0,
            (index / 8) as f32 * 256.0 + 128.0,
            index as f32,
        )
        .with_rotation(Quat::from_rotation_z(index as f32 * 0.11));
        let paint = CanvasPaint {
            fill: Some(Color::srgba(index as f32 / 64.0, 0.5, 0.7, 0.6)),
            fill_rule: CanvasFillRule::EvenOdd,
            stroke: Some(CanvasStroke {
                color: Color::WHITE,
                width: 6.0,
                ..default()
            }),
        };
        match index % 3 {
            0 => CanvasDocument::create_rectangle(
                app.world_mut(),
                document,
                CanvasRectangle {
                    corner_radius: 15.0,
                    size: Vec2::splat(210.0),
                },
                transform,
                paint,
            )?,
            1 => CanvasDocument::create_ellipse(
                app.world_mut(),
                document,
                CanvasEllipse {
                    radii: Vec2::new(105.0, 75.0),
                },
                transform,
                paint,
            )?,
            _ => {
                use CanvasPathCommand::*;
                CanvasDocument::create_path(
                    app.world_mut(),
                    document,
                    CanvasPath {
                        commands: vec![
                            MoveTo(Vec2::new(-100.0, 0.0)),
                            CubicTo {
                                control1: Vec2::new(-100.0, -130.0),
                                control2: Vec2::new(100.0, -130.0),
                                to: Vec2::new(100.0, 0.0),
                            },
                            ArcTo {
                                radii: Vec2::splat(100.0),
                                rotation: 0.0,
                                large_arc: false,
                                sweep: true,
                                to: Vec2::new(-100.0, 0.0),
                            },
                            Close,
                            MoveTo(Vec2::new(-20.0, -20.0)),
                            LineTo(Vec2::new(20.0, -20.0)),
                            LineTo(Vec2::new(20.0, 20.0)),
                            LineTo(Vec2::new(-20.0, 20.0)),
                            Close,
                        ],
                    },
                    transform,
                    paint,
                )?
            }
        };
    }
    let baseline = assets(app.world());
    Ok(Fixture {
        app,
        document,
        directory: tempfile::tempdir().map_err(error)?,
        assets: baseline,
    })
}

fn export(fixture: &mut Fixture, size: UVec2) -> Result {
    let path = fixture.directory.path().join("export.png");
    fixture.app.world_mut().resource_mut::<Completion>().0 = None;
    let region = if size.x == size.y {
        Vec2::splat(2048.0)
    } else {
        Vec2::new(1920.0, 1080.0)
    };
    CanvasDocument::export_png(
        fixture.app.world_mut(),
        fixture.document,
        &path,
        CanvasPngOptions {
            region: Some(Rect::from_corners(Vec2::ZERO, region)),
            scale: size.x as f32 / region.x,
        },
    )
    .map_err(error)?;
    let deadline = Instant::now() + Duration::from_secs(35);
    loop {
        frame(&mut fixture.app)?;
        if let Some(result) = fixture
            .app
            .world_mut()
            .resource_mut::<Completion>()
            .0
            .take()
        {
            if result.map_err(error)? != size {
                return Err(missing("PNG output dimensions"));
            }
            break;
        }
        if Instant::now() >= deadline {
            return Err(error("PNG benchmark export timed out"));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if assets(fixture.app.world()) != fixture.assets {
        return Err(error("PNG export did not reclaim its render assets"));
    }
    Ok(())
}

fn main() -> Result {
    let mut harness = Harness::new("canvas-png-criterion")?;
    for size in [
        UVec2::splat(512),
        UVec2::splat(2048),
        UVec2::new(1920, 1080),
        UVec2::new(3840, 2160),
        UVec2::new(7680, 4320),
    ] {
        run(
            &mut harness,
            &format!(
                "canvas_png/dx12/mixed_shapes64/size{}x{}/render_readback_encode_write_cleanup",
                size.x, size.y
            ),
            false,
            fixture,
            |fixture, _| export(fixture, size),
            |fixture| Ok(fixture.app.world().entities().count_spawned()),
        )?;
    }
    harness.finish()
}
