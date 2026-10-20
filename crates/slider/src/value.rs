use crate::{WidgetrySlider, WidgetrySliderOrientation, WidgetrySliderProps};
use bevy::{prelude::*, ui_widgets::ValueChange};
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Component)]
pub(crate) struct SliderState {
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) step: Option<f32>,
    pub(crate) orientation: WidgetrySliderOrientation,
    value: f32,
}

fn invalid(reason: &str) -> BevyError {
    widgetry_error!(reason, "Slider 数值请求无效");
    bevy_widgetry_core::scene::logged_error(reason)
}

impl SliderState {
    pub(crate) fn new(props: &WidgetrySliderProps) -> Result<Self, BevyError> {
        if !props.min.is_finite()
            || !props.max.is_finite()
            || props.min >= props.max
            || !(props.max - props.min).is_finite()
        {
            return Err(invalid("Slider requires finite min < max and finite span"));
        }
        if props
            .step
            .is_some_and(|step| !step.is_finite() || step <= 0.0)
        {
            return Err(invalid("Slider step must be finite and positive"));
        }
        let state = Self {
            min: props.min,
            max: props.max,
            step: props.step,
            orientation: props.orientation,
            value: props.value.unwrap_or(props.min),
        };
        state.validate(state.value)?;
        Ok(state)
    }

    fn validate(&self, value: f32) -> Result<(), BevyError> {
        if !value.is_finite() || value < self.min || value > self.max {
            return Err(invalid("Slider value must be finite and within [min, max]"));
        }
        if let Some(step) = self.step
            && value != self.min
            && value != self.max
            && !reachable(value, self.min, step)
            && !reachable(value, self.max, step)
        {
            return Err(invalid(
                "Slider value must be reachable from either endpoint by step",
            ));
        }
        Ok(())
    }
}

fn reachable(value: f32, endpoint: f32, step: f32) -> bool {
    let distance = (f64::from(value) - f64::from(endpoint)).abs();
    let step = f64::from(step);
    let expected = (distance / step).round() * step;
    let projected = if value >= endpoint {
        f64::from(endpoint) + expected
    } else {
        f64::from(endpoint) - expected
    };
    if projected as f32 == value {
        return true;
    }
    // Bound rounding allowance by the step so large offsets cannot accept arbitrary values.
    let magnitude = f64::from(value.abs().max(endpoint.abs())).max(distance);
    let tolerance = (4.0 * f64::from(f32::EPSILON) * magnitude).min(step * 0.001);
    (distance - expected).abs() <= tolerance
}

fn state(world: &World, entity: Entity) -> Result<&SliderState, BevyError> {
    if world.get::<WidgetrySlider>(entity).is_none() {
        return Err(invalid(
            "Slider target does not exist or is not a WidgetrySlider",
        ));
    }
    world
        .get::<SliderState>(entity)
        .ok_or_else(|| invalid("Slider runtime state is missing"))
}

impl WidgetrySlider {
    pub fn value(world: &World, entity: Entity) -> Result<f32, BevyError> {
        Ok(state(world, entity)?.value)
    }

    pub fn orientation(
        world: &World,
        entity: Entity,
    ) -> Result<WidgetrySliderOrientation, BevyError> {
        Ok(state(world, entity)?.orientation)
    }

    pub fn set_value_in_world(
        world: &mut World,
        entity: Entity,
        value: f32,
    ) -> Result<bool, BevyError> {
        let current = state(world, entity)?;
        current.validate(value)?;
        if current.value == value {
            return Ok(false);
        }
        world
            .get_mut::<SliderState>(entity)
            .ok_or_else(|| invalid("Slider runtime state is missing"))?
            .value = value;
        world.trigger(ValueChange {
            source: entity,
            value,
            is_final: true,
        });
        Ok(true)
    }

    pub fn set_value(commands: &mut Commands, entity: Entity, value: f32) {
        commands.queue(move |world: &mut World| {
            Self::set_value_in_world(world, entity, value).map(|_| ())
        });
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn stepped_values_are_reachable_from_both_endpoints() {
        let state = SliderState::new(&WidgetrySliderProps {
            max: 10.0,
            step: Some(3.0),
            ..default()
        })
        .unwrap();
        for value in [0.0, 3.0, 6.0, 9.0, 10.0, 7.0, 4.0, 1.0] {
            assert!(state.validate(value).is_ok(), "{value}");
        }
        assert!(state.validate(4.5).is_err());
    }

    #[test]
    fn decimal_rounding_and_large_offsets_preserve_reachable_values() {
        for (min, max, step) in [
            (0.0, 1.0, 0.1),
            (-1.0, 1.0, 0.1),
            (1_000_000.0, 1_000_010.0, 0.3),
        ] {
            let state = SliderState::new(&WidgetrySliderProps {
                min,
                max,
                step: Some(step),
                ..default()
            })
            .unwrap();
            for endpoint in [min, max] {
                for count in 0..10 {
                    let direction = if endpoint == min { 1.0 } else { -1.0 };
                    let value = (f64::from(endpoint)
                        + direction * f64::from(step) * f64::from(count))
                        as f32;
                    assert!(state.validate(value).is_ok(), "{min} {max} {step} {value}");
                }
            }
        }
        let state = SliderState::new(&WidgetrySliderProps {
            min: 1_000_000.0,
            max: 1_000_010.0,
            step: Some(3.0),
            ..default()
        })
        .unwrap();
        assert!(state.validate(1_000_004.5).is_err());
        let tiny = SliderState::new(&WidgetrySliderProps {
            max: f32::from_bits(10),
            step: Some(f32::from_bits(3)),
            ..default()
        })
        .unwrap();
        assert!(tiny.validate(f32::from_bits(4)).is_ok());
        assert!(tiny.validate(f32::from_bits(5)).is_err());
    }
}
