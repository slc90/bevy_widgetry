use crate::{
    SplitterError, SplitterFailure,
    error::{failure, missing},
};
use crate::{WidgetrySplitter, splitter::State};
use bevy::{prelude::*, ui_widgets::ValueChange};
use bevy_widgetry_diagnostics::ErrorFrame;

impl WidgetrySplitter {
    pub fn set_split_ratio_in_world(
        world: &mut World,
        entity: Entity,
        ratio: f32,
    ) -> Result<bool, SplitterFailure> {
        commit(world, entity, ratio, true)
    }

    pub fn set_split_ratio(commands: &mut Commands, entity: Entity, ratio: f32) {
        commands.queue(move |world: &mut World| {
            Self::set_split_ratio_in_world(world, entity, ratio)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
}

pub(crate) fn commit(
    world: &mut World,
    entity: Entity,
    ratio: f32,
    is_final: bool,
) -> Result<bool, SplitterFailure> {
    let frame = || {
        ErrorFrame::new("splitter.commit_ratio")
            .field("root", entity)
            .field("ratio", ratio)
    };
    let current = WidgetrySplitter::split_ratio(world, entity)
        .map_err(|error| error.with_context(frame()))?;
    if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
        return Err(failure(
            SplitterError::InvalidRatio {
                root: entity,
                ratio,
            },
            frame(),
        ));
    }
    let ratio = crate::layout::constrain_ratio(world, entity, ratio)
        .map_err(|error| error.with_context(frame()))?;
    if ratio == current {
        return Ok(false);
    }
    world
        .get_mut::<State>(entity)
        .ok_or_else(|| missing(Some(entity), entity, "State"))?
        .ratio = ratio;
    world.trigger(ValueChange {
        source: entity,
        value: ratio,
        is_final,
    });
    Ok(true)
}
