use crate::{
    WidgetrySplitter,
    splitter::{State, invalid},
};
use bevy::{prelude::*, ui_widgets::ValueChange};

impl WidgetrySplitter {
    pub fn set_split_ratio_in_world(
        world: &mut World,
        entity: Entity,
        ratio: f32,
    ) -> Result<bool, BevyError> {
        commit(world, entity, ratio, true)
    }

    pub fn set_split_ratio(commands: &mut Commands, entity: Entity, ratio: f32) {
        commands.queue(move |world: &mut World| {
            Self::set_split_ratio_in_world(world, entity, ratio).map(|_| ())
        });
    }
}

fn commit(
    world: &mut World,
    entity: Entity,
    ratio: f32,
    is_final: bool,
) -> Result<bool, BevyError> {
    let current = WidgetrySplitter::split_ratio(world, entity)?;
    if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
        return Err(invalid(
            "Splitter split_ratio must be finite and within [0, 1]",
        ));
    }
    let ratio = crate::layout::constrain_ratio(world, entity, ratio).map_err(|error| {
        bevy_widgetry_internal_log::widgetry_error!(?entity,%error,"Splitter 比例请求失败");
        error
    })?;
    if ratio == current {
        return Ok(false);
    }
    world
        .get_mut::<State>(entity)
        .ok_or_else(|| invalid("Splitter runtime state is missing"))?
        .ratio = ratio;
    world.trigger(ValueChange {
        source: entity,
        value: ratio,
        is_final,
    });
    Ok(true)
}
