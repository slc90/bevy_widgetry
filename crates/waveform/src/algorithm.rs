use crate::{LttbReducer, MinMaxReducer, Waveform, WaveformReducer, WaveformRuntime};
use bevy::prelude::*;
use bevy_widgetry_core::scene::logged_error;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaveformAlgorithm {
    #[default]
    MinMax,
    Lttb,
}

impl WaveformAlgorithm {
    pub(crate) fn reducer(self) -> Box<dyn WaveformReducer> {
        match self {
            Self::MinMax => Box::<MinMaxReducer>::default(),
            Self::Lttb => Box::<LttbReducer>::default(),
        }
    }
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformAlgorithmChanged {
    pub entity: Entity,
    pub algorithm: WaveformAlgorithm,
}

impl Waveform {
    pub fn set_algorithm_in_world(
        world: &mut World,
        entity: Entity,
        algorithm: WaveformAlgorithm,
    ) -> Result<bool, BevyError> {
        Self::validate_target(world, entity)?;
        let changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| logged_error("Waveform runtime missing"))?
            .set_algorithm(algorithm)?;
        if changed {
            world.trigger(WaveformAlgorithmChanged { entity, algorithm });
        }
        Ok(changed)
    }

    pub fn set_algorithm(commands: &mut Commands, entity: Entity, algorithm: WaveformAlgorithm) {
        commands.queue(move |world: &mut World| {
            Self::set_algorithm_in_world(world, entity, algorithm).map(|_| ())
        });
    }
}
