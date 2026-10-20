use crate::value::SliderState;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Gesture {
    anchor_value: f64,
    anchor_position: f64,
    previous_position: f64,
    direction: f64,
}

impl Gesture {
    pub(crate) fn new(value: f32, position: f64) -> Self {
        Self {
            anchor_value: f64::from(value),
            anchor_position: position,
            previous_position: position,
            direction: 0.0,
        }
    }

    pub(crate) fn advance(&mut self, state: &SliderState, position: f64) -> f32 {
        let movement = position - self.previous_position;
        if movement != 0.0 {
            if self.direction != 0.0 && movement.signum() != self.direction {
                self.anchor_value = f64::from(state.value);
                self.anchor_position = self.previous_position;
            }
            self.direction = movement.signum();
        }
        self.previous_position = position;
        // Keep endpoint identity when subtracting a large negative anchor loses a small max.
        if position <= f64::from(state.min) {
            return state.min;
        }
        if position >= f64::from(state.max) {
            return state.max;
        }
        directional(state, self.anchor_value, position - self.anchor_position)
    }
}

pub(crate) fn directional(state: &SliderState, anchor: f64, delta: f64) -> f32 {
    let min = f64::from(state.min);
    let max = f64::from(state.max);
    let target = (anchor + delta).clamp(min, max);
    if target == min || target == max {
        return target as f32;
    }
    match state.step {
        None => target as f32,
        Some(step) => {
            let step = f64::from(step);
            // Recover the mathematical grid point before adding another step.
            // Repeated f32 addition at large offsets can otherwise drift off both endpoint grids.
            let from_min = min + ((anchor - min) / step).round() * step;
            let from_max = max - ((max - anchor) / step).round() * step;
            let anchor = if (from_min - anchor).abs() <= (from_max - anchor).abs() {
                from_min
            } else {
                from_max
            };
            let steps = (delta.abs() / step + 1e-6).floor();
            (anchor + delta.signum() * steps * step).clamp(min, max) as f32
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::WidgetrySliderProps;
    use bevy::prelude::default;

    #[test]
    fn reversing_from_the_endpoint_uses_the_current_value_as_anchor() {
        let mut state = SliderState::new(&WidgetrySliderProps {
            max: 10.0,
            step: Some(3.0),
            ..default()
        })
        .unwrap();
        let mut gesture = Gesture::new(0.0, 0.0);
        for (position, expected) in [
            (3.0, 3.0),
            (6.0, 6.0),
            (9.0, 9.0),
            (10.0, 10.0),
            (7.0, 7.0),
            (4.0, 4.0),
            (1.0, 1.0),
            (0.0, 0.0),
        ] {
            state.value = gesture.advance(&state, position);
            assert_eq!(state.value, expected);
        }
    }
}
