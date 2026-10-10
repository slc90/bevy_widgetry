use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::disabled::WidgetryEffectiveDisabled;
use bevy_widgetry_test_utils::benchmark::artifact::{Artifact, error};
use bevy_widgetry_test_utils::benchmark::{Harness, run};
use bevy_widgetry_test_utils::scene_app;
use std::fs::File;
use std::io::Write;

#[derive(Resource, Default, Clone, Copy)]
struct Writes {
    effective: usize,
    official: usize,
}

struct Fixture {
    app: App,
    root: Entity,
    leaf: Entity,
    count: usize,
    disabled: bool,
    expected_writes: usize,
}

fn fixture(count: usize, deep: bool) -> Result<Fixture> {
    let mut app = scene_app();
    app.init_resource::<Writes>()
        .add_observer(
            |_: On<Insert<WidgetryEffectiveDisabled>>, mut writes: ResMut<Writes>| {
                writes.effective += 1
            },
        )
        .add_observer(
            |_: On<Insert<InteractionDisabled>>, mut writes: ResMut<Writes>| writes.official += 1,
        )
        .add_observer(
            |_: On<Remove<InteractionDisabled>>, mut writes: ResMut<Writes>| writes.official += 1,
        );
    let root = app.world_mut().spawn(Node::default()).id();
    let mut leaf = root;
    for _ in 1..count {
        let parent = if deep { leaf } else { root };
        leaf = app
            .world_mut()
            .spawn((Node::default(), ChildOf(parent)))
            .id();
    }
    app.world_mut().flush();
    app.update();
    *app.world_mut().resource_mut::<Writes>() = Writes::default();
    Ok(Fixture {
        app,
        root,
        leaf,
        count,
        disabled: false,
        expected_writes: 0,
    })
}

fn operation(fixture: &mut Fixture, toggle: bool) -> Result {
    *fixture.app.world_mut().resource_mut::<Writes>() = Writes::default();
    fixture.expected_writes = if toggle { fixture.count } else { 0 };
    if toggle {
        fixture.disabled = !fixture.disabled;
        if fixture.disabled {
            fixture
                .app
                .world_mut()
                .entity_mut(fixture.root)
                .insert(InteractionDisabled);
        } else {
            fixture
                .app
                .world_mut()
                .entity_mut(fixture.root)
                .remove::<InteractionDisabled>();
        }
        fixture.app.world_mut().flush();
    }
    fixture.app.update();
    Ok(())
}

fn verify(fixture: &mut Fixture) -> Result<u32> {
    let world = fixture.app.world();
    let writes = *world.resource::<Writes>();
    let expected = fixture.expected_writes;
    if writes.effective != expected
        || writes.official != expected
        || world
            .get::<WidgetryEffectiveDisabled>(fixture.leaf)
            .is_none_or(|state| state.is_disabled() != fixture.disabled)
    {
        return Err(error(format!(
            "effective Disabled budget: expected={expected}, effective={}, official={}, leaf={:?}, disabled={}",
            writes.effective,
            writes.official,
            world.get::<WidgetryEffectiveDisabled>(fixture.leaf),
            fixture.disabled
        )));
    }
    Ok(world.entities().count_spawned())
}

fn main() -> Result {
    let mut harness = Harness::new("effective-disabled-criterion")?;
    let work = Artifact::new(
        "effective-disabled-work",
        "bench; Cargo defaults opt-level=3",
    )?;
    let mut csv = File::create(work.directory.join("writes.csv")).map_err(error)?;
    writeln!(
        csv,
        "nodes,shape,operation,effective_writes,official_writes"
    )
    .map_err(error)?;
    for count in [100, 1000, 10000] {
        for deep in [false, true] {
            let shape = if deep { "deep" } else { "shallow" };
            for toggle in [false, true] {
                let action = if toggle { "ancestor-toggle" } else { "idle" };
                let mut probe = fixture(count, deep)?;
                operation(&mut probe, toggle)?;
                verify(&mut probe)?;
                let writes = *probe.app.world().resource::<Writes>();
                writeln!(
                    csv,
                    "{count},{shape},{action},{},{}",
                    writes.effective, writes.official
                )
                .map_err(error)?;
                drop(probe);
                run(
                    &mut harness,
                    &format!("effective-disabled/n{count}/{shape}/{action}"),
                    false,
                    || fixture(count, deep),
                    |fixture, _| operation(fixture, toggle),
                    verify,
                )?;
            }
        }
    }
    harness.finish()
}
