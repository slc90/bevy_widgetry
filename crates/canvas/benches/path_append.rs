const PATH_COLUMNS: usize = 100;
use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, run};

struct Fixture {
    world: World,
    document: Entity,
    shape: Entity,
    original: CanvasPath,
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let mut commands = vec![CanvasPathCommand::MoveTo(Vec2::ZERO)];
    commands.extend((1..count).map(|i| {
        CanvasPathCommand::LineTo(Vec2::new(
            (i % PATH_COLUMNS) as f32,
            (i / PATH_COLUMNS) as f32,
        ))
    }));
    let original = CanvasPath { commands };
    let shape = CanvasDocument::create_path(
        &mut world,
        document,
        original.clone(),
        Transform::default(),
        CanvasPaint::default(),
    )
    .map_err(|error| error.into_bevy_error())?;
    Ok(Fixture {
        world,
        document,
        shape,
        original,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("canvas-path-append-criterion")?;
    let command_counts = [100, 1000, 10000];
    for count in command_counts {
        run(
            &mut harness,
            &format!("canvas_path_append/commands{count}/append_rectangle_and_restore"),
            false,
            || fixture(count),
            |f, _| {
                CanvasDocument::append_rectangle(
                    &mut f.world,
                    f.document,
                    f.shape,
                    Rect::new(-20.0, -10.0, 20.0, 10.0),
                )
                .map_err(|error| error.into_bevy_error())?;
                if !matches!(f.world.get::<CanvasGeometry>(f.shape), Some(CanvasGeometry::Path(path)) if path.commands.len() == count + 5)
                {
                    return Err(error("contour append must preserve the original commands"));
                }
                // Restore through the public API so each measured operation has the same workload.
                CanvasDocument::update_path(
                    &mut f.world,
                    f.document,
                    f.shape,
                    f.original.clone(),
                    Transform::default(),
                    CanvasPaint::default(),
                )
                .map_err(|error| error.into_bevy_error())
            },
            |f| Ok(f.world.entities().count_spawned()),
        )?;
    }
    harness.finish()
}
