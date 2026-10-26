use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, run};

struct Fixture {
    world: World,
    document: Entity,
    shape: Entity,
    count: usize,
    front: bool,
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let mut shape = None;
    for _ in 0..count {
        shape = Some(CanvasDocument::create_rectangle(
            &mut world,
            document,
            CanvasRectangle {
                size: Vec2::splat(20.0),
            },
            Transform::default(),
            CanvasPaint::default(),
        )?);
    }
    Ok(Fixture {
        world,
        document,
        shape: shape.ok_or_else(|| error("benchmark requires shapes"))?,
        count,
        front: true,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("canvas-order-criterion")?;
    for count in [100, 1000, 10000] {
        run(
            &mut harness,
            &format!("canvas_order/shapes{count}/reorder_and_query"),
            false,
            || fixture(count),
            |fixture, _| {
                let operation = if fixture.front {
                    CanvasOrder::Back
                } else {
                    CanvasOrder::Front
                };
                if !CanvasDocument::reorder_shape(
                    &mut fixture.world,
                    fixture.document,
                    fixture.shape,
                    operation,
                )? {
                    return Err(error("benchmark reorder unexpectedly did no work"));
                }
                fixture.front = !fixture.front;
                let result = CanvasDocument::shape_order(&mut fixture.world, fixture.document)?;
                if result.len() != fixture.count {
                    return Err(error("benchmark lost shapes"));
                }
                std::hint::black_box(result);
                Ok(())
            },
            |fixture| Ok(fixture.world.entities().count_spawned()),
        )?;
    }
    harness.finish()
}
