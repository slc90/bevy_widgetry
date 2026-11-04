use crate::{Waveform, WaveformRuntime, WaveformUnit};
use bevy::prelude::*;
use bevy_widgetry_job::JobFailure;

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

    pub(crate) fn validate(self) -> Result<(), crate::WaveformFailure> {
        if !self.value.is_finite() || self.value <= 0.0 {
            return Err(crate::error::failure(
                crate::WaveformError::Amplitude { amplitude: self },
                "waveform.amplitude",
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
    ) -> Result<Self, JobFailure> {
        for (index, &choice) in choices.iter().enumerate() {
            choice.validate().map_err(JobFailure::from)?;
            if choices[..index]
                .iter()
                .any(|&other| choice.equivalent(other))
            {
                return Err(crate::error::request(
                    crate::WaveformError::AmplitudeChoices {
                        choices,
                        amplitude: initial,
                        duplicate: true,
                    },
                    None,
                    "waveform.amplitude.choices",
                ));
            }
        }
        if !choices.is_empty() && !choices.contains(&initial) {
            return Err(crate::error::request(
                crate::WaveformError::AmplitudeChoices {
                    choices,
                    amplitude: initial,
                    duplicate: false,
                },
                None,
                "waveform.amplitude.choices",
            ));
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
    ) -> Result<bool, JobFailure> {
        Self::validate_target(world, entity)?;
        amplitude.validate().map_err(|error| {
            JobFailure::from(error).with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("waveform.amplitude.set")
                    .field("root", entity),
            )
        })?;
        if let Some(choices) = world.get::<AmplitudeChoices>(entity)
            && !choices.0.is_empty()
            && !choices.0.contains(&amplitude)
        {
            return Err(crate::error::request(
                crate::WaveformError::AmplitudeChoices {
                    choices: choices.0.clone(),
                    amplitude,
                    duplicate: false,
                },
                Some(entity),
                "waveform.amplitude.set",
            ));
        }
        let changed = world
            .get_mut::<WaveformRuntime>(entity)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::Runtime)
                    .at(entity)
                    .into_failure(Some(entity), "waveform.request")
            })?
            .set_amplitude(amplitude);
        if changed {
            world.trigger(WaveformAmplitudeChanged { entity, amplitude });
        }
        Ok(changed)
    }

    pub fn set_amplitude(commands: &mut Commands, entity: Entity, amplitude: WaveformAmplitude) {
        commands.queue(move |world: &mut World| {
            Self::set_amplitude_in_world(world, entity, amplitude)
                .map(|_| ())
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
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
