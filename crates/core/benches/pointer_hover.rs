use bevy::{
    camera::NormalizedRenderTarget,
    ecs::entity::EntityHashMap,
    picking::{
        backend::HitData,
        hover::{DirectlyHovered, HoverMap, Hovered},
        pointer::{Location, PointerLocation},
    },
    prelude::*,
};
use bevy_widgetry_core::pointer::WidgetryPointerPlugin;
use bevy_widgetry_test_utils::{
    benchmark::{Harness, missing, run},
    pointer_ids,
};

struct Fixture {
    app: App,
    hits: [Entity; 2],
    active: usize,
}

fn fixture(count: usize, pointers: usize) -> Result<Fixture> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, WidgetryPointerPlugin))
        .init_resource::<HoverMap>();
    let root = app.world_mut().spawn(Hovered(false)).id();
    let hits = [0, 1].map(|_| {
        app.world_mut()
            .spawn((Hovered(false), DirectlyHovered(false), ChildOf(root)))
            .id()
    });
    for _ in 2..count {
        app.world_mut()
            .spawn((Hovered(false), DirectlyHovered(false), ChildOf(root)));
    }
    for id in pointer_ids().into_iter().take(pointers) {
        app.world_mut().spawn((
            id,
            PointerLocation::new(Location {
                target: NormalizedRenderTarget::None {
                    width: 1920,
                    height: 1080,
                },
                position: Vec2::ONE,
            }),
        ));
    }
    for id in pointer_ids().into_iter().take(pointers) {
        let mut map = EntityHashMap::default();
        map.insert(hits[0], HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
        app.world_mut().resource_mut::<HoverMap>().insert(id, map);
    }
    app.update();
    Ok(Fixture {
        app,
        hits,
        active: 0,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("pointer-hover-criterion")?;
    for count in [10, 1000, 10000] {
        for pointers in [1, 2] {
            for action in ["idle", "move"] {
                run(
                    &mut harness,
                    &format!("pointer-hover/n{count}/p{pointers}/{action}"),
                    false,
                    || fixture(count, pointers),
                    |fixture, index| {
                        fixture.active = if action == "move" { index % 2 } else { 0 };
                        let mut map = fixture.app.world_mut().resource_mut::<HoverMap>();
                        map.clear();
                        for id in pointer_ids().into_iter().take(pointers) {
                            let mut hits = EntityHashMap::default();
                            hits.insert(
                                fixture.hits[fixture.active],
                                HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                            );
                            map.insert(id, hits);
                        }
                        fixture.app.update();
                        Ok(())
                    },
                    |fixture| {
                        let world = fixture.app.world();
                        if world
                            .get::<Hovered>(fixture.hits[fixture.active])
                            .is_none_or(|hover| !hover.0)
                            || world
                                .get::<DirectlyHovered>(fixture.hits[1 - fixture.active])
                                .is_none_or(|hover| hover.0)
                        {
                            return Err(missing("expected Pointer hover projection"));
                        }
                        Ok(world.entities().len())
                    },
                )?;
            }
        }
    }
    harness.finish()
}
