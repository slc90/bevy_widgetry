use bevy::{
    picking::{
        InteractionPlugin, PickingPlugin,
        backend::{HitData, PointerHits},
        cursor::EntityCursor,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    window::{CursorIcon, SystemCursorIcon, WindowRef},
};
use bevy_widgetry_cursor::WidgetryCursorPlugin;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run};
use std::hint::black_box;

struct Fixture {
    app: App,
    windows: Vec<Entity>,
    camera: Entity,
    candidates: Vec<Entity>,
}

fn fixture(windows: usize, candidates: usize, depth: usize) -> Result<Fixture> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        PickingPlugin,
        InteractionPlugin,
        WidgetryCursorPlugin,
    ));
    app.world_mut().spawn(PointerId::Mouse);
    let windows = (0..windows)
        .map(|_| app.world_mut().spawn(Window::default()).id())
        .collect();
    let camera = app.world_mut().spawn_empty().id();
    let mut entities = Vec::new();
    for _ in 0..candidates {
        let mut parent = app
            .world_mut()
            .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
            .id();
        for _ in 1..depth {
            parent = app.world_mut().spawn(ChildOf(parent)).id();
        }
        app.world_mut().entity_mut(parent).insert(Pickable {
            should_block_lower: false,
            is_hoverable: true,
        });
        entities.push(parent);
    }
    app.update();
    Ok(Fixture {
        app,
        windows,
        camera,
        candidates: entities,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("cursor-criterion")?;
    for (windows, candidates, depth) in [
        (1, 1, 1),
        (2, 1, 1),
        (8, 1, 1),
        (1, 32, 1),
        (1, 256, 1),
        (1, 1, 16),
        (1, 1, 64),
        (8, 256, 64),
    ] {
        run(
            &mut harness,
            &format!("cursor/w{windows}_h{candidates}_d{depth}/mouse_update"),
            false,
            || fixture(windows, candidates, depth),
            |fixture, index| {
                let window = fixture.windows[index % fixture.windows.len()];
                let location = Location {
                    target: bevy::camera::RenderTarget::Window(WindowRef::Entity(window))
                        .normalize(None)
                        .ok_or_else(|| missing("window render target"))?,
                    position: Vec2::splat(20.0),
                };
                fixture.app.world_mut().write_message(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Move { delta: Vec2::ONE },
                ));
                fixture.app.world_mut().write_message(PointerHits::new(
                    PointerId::Mouse,
                    fixture
                        .candidates
                        .iter()
                        .enumerate()
                        .map(|(depth, entity)| {
                            (
                                *entity,
                                HitData::new(fixture.camera, depth as f32, None, None),
                            )
                        })
                        .collect(),
                    0.0,
                ));
                fixture.app.update();
                let cursor = fixture
                    .app
                    .world()
                    .get::<CursorIcon>(window)
                    .ok_or_else(|| missing("window cursor"))?;
                if *cursor != CursorIcon::System(SystemCursorIcon::Pointer) {
                    return Err(BevyError::error(
                        "benchmark Mouse hover did not resolve cursor",
                    ));
                }
                black_box(cursor);
                Ok(())
            },
            |fixture| Ok(fixture.app.world().entities().len()),
        )?;
    }
    harness.finish()
}
