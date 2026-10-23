use crate::WidgetryProgressState;
use bevy::prelude::*;

use std::time::Duration;

const DURATION: Duration = Duration::from_millis(150);

#[derive(Component)]
pub(crate) struct Animation {
    pub(crate) visible: f32,
    target: WidgetryProgressState,
    start: f32,
    elapsed: Duration,
}

fn ratio(state: WidgetryProgressState) -> f32 {
    match state {
        WidgetryProgressState::Determinate { value } => value,
        WidgetryProgressState::Indeterminate => 0.0,
    }
}

impl Animation {
    pub(crate) fn new(state: WidgetryProgressState) -> Self {
        Self {
            visible: ratio(state),
            target: state,
            start: ratio(state),
            elapsed: DURATION,
        }
    }

    pub(crate) fn active(&self) -> bool {
        self.elapsed < DURATION
    }

    pub(crate) fn retarget(&mut self, next: WidgetryProgressState) -> bool {
        if self.target == next {
            return false;
        }
        if matches!(
            (self.target, next),
            (
                WidgetryProgressState::Determinate { .. },
                WidgetryProgressState::Determinate { .. }
            )
        ) {
            self.start = self.visible;
            self.elapsed = Duration::ZERO;
            self.target = next;
        } else {
            *self = Self::new(next);
        }
        true
    }

    pub(crate) fn advance(&mut self, delta: Duration) {
        self.elapsed = (self.elapsed + delta).min(DURATION);
        let t = self.elapsed.as_secs_f32() / DURATION.as_secs_f32();
        self.visible = if self.active() {
            self.start + (ratio(self.target) - self.start) * (1.0 - (1.0 - t).powi(3))
        } else {
            ratio(self.target)
        };
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    fn value(value: f32) -> WidgetryProgressState {
        WidgetryProgressState::Determinate { value }
    }

    #[test]
    fn cubic_ease_out_moves_faster_at_start_and_lands_exactly() {
        let mut animation = Animation::new(value(0.2));
        animation.retarget(value(1.0));
        animation.advance(Duration::from_millis(50));
        let first = animation.visible - 0.2;
        animation.advance(Duration::from_millis(50));
        let second = animation.visible - 0.2 - first;
        assert!(first > second && second > 0.0);
        animation.advance(Duration::from_millis(50));
        assert_eq!(animation.visible, 1.0);
        assert!(!animation.active());
        assert!(!animation.retarget(value(1.0)));
    }
}
