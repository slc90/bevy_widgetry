use crate::{Waveform, WaveformRuntime, WaveformUnit};
use bevy::prelude::*;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformAmplitude {
    pub value: f32,
    pub unit: WaveformUnit,
}

impl Default for WaveformAmplitude {
    fn default() -> Self {
        Self {
            value: 1.0,
            unit: WaveformUnit::Volts,
        }
    }
}

impl WaveformAmplitude {
    pub fn volts(self) -> f64 {
        f64::from(self.value)
            * match self.unit {
                WaveformUnit::Volts => 1.0,
                WaveformUnit::Millivolts => 1e-3,
                WaveformUnit::Microvolts => 1e-6,
            }
    }

    pub fn label(self) -> String {
        let unit = match self.unit {
            WaveformUnit::Volts => "V",
            WaveformUnit::Millivolts => "mV",
            WaveformUnit::Microvolts => "µV",
        };
        format!("{} {unit}", self.value)
    }

    pub(crate) fn validate(self) -> Result {
        if !self.value.is_finite() || self.value <= 0.0 {
            widgetry_error!(?self, "Waveform 幅值上限必须是有限正数");
            return Err(logged_error(
                "Waveform amplitude must be finite and positive",
            ));
        }
        Ok(())
    }

    fn equivalent(self, other: Self) -> bool {
        (self.volts() - other.volts()).abs() <= self.volts().abs().max(other.volts().abs()) * 1e-7
    }
}

#[derive(Component)]
pub(crate) struct AmplitudeChoices(pub Vec<WaveformAmplitude>);

impl AmplitudeChoices {
    pub(crate) fn new(
        choices: Vec<WaveformAmplitude>,
        initial: WaveformAmplitude,
    ) -> Result<Self, BevyError> {
        for (index, &choice) in choices.iter().enumerate() {
            choice.validate()?;
            if choices[..index]
                .iter()
                .any(|&other| choice.equivalent(other))
            {
                widgetry_error!(?choices, "Waveform 幅值候选项物理值重复");
                return Err(logged_error("Duplicate Waveform amplitude choices"));
            }
        }
        if !choices.is_empty() && !choices.contains(&initial) {
            widgetry_error!(?initial, "Waveform 初始幅值不在候选项中");
            return Err(logged_error("Initial amplitude missing from choices"));
        }
        Ok(Self(choices))
    }
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformAmplitudeChanged {
    pub entity: Entity,
    pub amplitude: WaveformAmplitude,
}

impl Waveform {
    pub fn set_amplitude_in_world(
        world: &mut World,
        entity: Entity,
        amplitude: WaveformAmplitude,
    ) -> Result<bool, BevyError> {
        Self::validate_target(world, entity)?;
        amplitude.validate()?;
        if let Some(choices) = world.get::<AmplitudeChoices>(entity)
            && !choices.0.is_empty()
            && !choices.0.contains(&amplitude)
        {
            widgetry_error!(?entity, ?amplitude, "Waveform 幅值不在候选项中");
            return Err(logged_error(
                "Waveform amplitude is not a configured choice",
            ));
        }
        let changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| logged_error("Waveform runtime missing"))?
            .set_amplitude(amplitude);
        if changed {
            world.trigger(WaveformAmplitudeChanged { entity, amplitude });
        }
        Ok(changed)
    }

    pub fn set_amplitude(commands: &mut Commands, entity: Entity, amplitude: WaveformAmplitude) {
        commands.queue(move |world: &mut World| {
            Self::set_amplitude_in_world(world, entity, amplitude).map(|_| ())
        });
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_and_equivalent_presets() {
        let uv = WaveformAmplitude {
            value: 1000.0,
            unit: WaveformUnit::Microvolts,
        };
        let mv = WaveformAmplitude {
            value: 1.0,
            unit: WaveformUnit::Millivolts,
        };
        assert!(AmplitudeChoices::new(vec![uv, mv], uv).is_err());
        for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(
                WaveformAmplitude {
                    value,
                    unit: WaveformUnit::Volts
                }
                .validate()
                .is_err()
            );
        }
        assert!(AmplitudeChoices::new(vec![uv], mv).is_err());
        assert!(AmplitudeChoices::new(vec![uv], uv).is_ok());
    }
}
