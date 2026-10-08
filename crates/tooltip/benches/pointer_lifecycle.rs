use bevy::{
    camera::NormalizedRenderTarget,
    ecs::entity::EntityHashMap,
    picking::{
        backend::HitData,
        hover::HoverMap,
        pointer::{Location, PointerAction, PointerId, PointerInput, PointerLocation},
    },
    prelude::*,
    time::TimeUpdateStrategy,
    ui_widgets::popover::Popover,
};
use bevy_widgetry_test_utils::{
    benchmark::{Harness, missing, run},
    scene_app,
};
use bevy_widgetry_tooltip::{TooltipContentFactory, WidgetryTooltip, WidgetryTooltipPlugin};
use std::time::Duration;

struct Fixture {
    app: App,
    location: Location,
}

fn fixture(hits: usize, pointers: usize, pending: bool) -> Result<Fixture> {
    let mut app = scene_app();
    app.init_resource::<HoverMap>()
        .add_message::<PointerInput>()
        .add_plugins(WidgetryTooltipPlugin);
    *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
        TimeUpdateStrategy::ManualDuration(Duration::ZERO);
    let location = Location {
        target: NormalizedRenderTarget::None {
            width: 1920,
            height: 1080,
        },
        position: Vec2::ONE,
    };
    for index in 0..pointers {
        app.world_mut().spawn((
            PointerId::Touch(index as u64),
            PointerLocation::new(location.clone()),
        ));
    }
    let anchor = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTooltip { @content: { TooltipContentFactory::new(|| bsn_list![(Node)]) } }
            Node
        })
        .map_err(BevyError::error)?
        .id();
    let mut map = EntityHashMap::default();
    map.insert(anchor, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
    for index in 1..hits {
        let entity = app.world_mut().spawn_empty().id();
        map.insert(
            entity,
            HitData::new(Entity::PLACEHOLDER, index as f32, None, None),
        );
    }
    if pending {
        app.world_mut()
            .resource_mut::<HoverMap>()
            .insert(PointerId::Touch(0), map);
    }
    app.update();
    Ok(Fixture { app, location })
}

fn main() -> Result {
    let mut harness = Harness::new("tooltip-pointer-criterion")?;
    for hits in [10, 1000] {
        for pointers in [1, 2, 32] {
            for action in ["idle", "pending", "cancelled"] {
                let pending = action != "idle";
                run(
                    &mut harness,
                    &format!("tooltip-pointer/h{hits}/p{pointers}/{action}"),
                    false,
                    || fixture(hits, pointers, pending),
                    |fixture, _| {
                        if action == "cancelled" {
                            for index in 0..pointers {
                                fixture.app.world_mut().write_message(PointerInput::new(
                                    PointerId::Touch(index as u64),
                                    fixture.location.clone(),
                                    PointerAction::Cancel,
                                ));
                            }
                        } else if pending {
                            fixture.app.world_mut().write_message(PointerInput::new(
                                PointerId::Touch(0),
                                fixture.location.clone(),
                                PointerAction::Move { delta: Vec2::ZERO },
                            ));
                        }
                        fixture.app.update();
                        Ok(())
                    },
                    |fixture| {
                        if fixture
                            .app
                            .world_mut()
                            .query_filtered::<Entity, With<Popover>>()
                            .iter(fixture.app.world())
                            .next()
                            .is_some()
                        {
                            return Err(missing("waiting Tooltip must not construct a popup"));
                        }
                        Ok(fixture.app.world().entities().len())
                    },
                )?;
            }
        }
    }
    harness.finish()
}
