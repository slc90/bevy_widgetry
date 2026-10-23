use crate::WidgetryProgressState;
use bevy::prelude::*;

use std::time::Duration;

const DURATION: Duration = Duration::from_millis(150);
const SEGMENT: f32 = 0.3;
const CYCLE_SECONDS: f64 = 1.2;

#[derive(Component)]
pub(crate) struct Animation {
    pub(crate) visible: f32,
    target: WidgetryProgressState,
    start: f32,
    elapsed: Duration,
    phase: f64,
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
            phase: f64::from(SEGMENT) / (1.0 + f64::from(SEGMENT)) * CYCLE_SECONDS,
        }
    }

    pub(crate) fn active(&self) -> bool {
        self.target == WidgetryProgressState::Indeterminate || self.elapsed < DURATION
    }

    pub(crate) fn window(&self) -> (f32, f32) {
        if self.target == WidgetryProgressState::Indeterminate {
            let left = (self.phase / CYCLE_SECONDS) as f32 * (1.0 + SEGMENT) - SEGMENT;
            let start = left.clamp(0.0, 1.0);
            let end = (left + SEGMENT).clamp(start, 1.0);
            (start, end - start)
        } else {
            (0.0, self.visible)
        }
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
        if self.target == WidgetryProgressState::Indeterminate {
            self.phase = (self.phase + delta.as_secs_f64()).rem_euclid(CYCLE_SECONDS);
            return;
        }
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

    #[test]
    fn looping_window_stays_inside_track_and_mode_changes_discard_old_timing() {
        let mut animation = Animation::new(WidgetryProgressState::Indeterminate);
        let initial = animation.window();
        assert!(initial.1 > 0.0);
        animation.advance(Duration::from_millis(400));
        assert!(animation.window().0 > initial.0);
        assert!(!animation.retarget(WidgetryProgressState::Indeterminate));
        for _ in 0..100 {
            animation.advance(Duration::from_millis(100));
            let (left, width) = animation.window();
            assert!(left >= 0.0 && width >= 0.0 && left + width <= 1.00001);
        }
        animation.retarget(value(0.75));
        assert_eq!(animation.window(), (0.0, 0.75));
        assert!(!animation.active());
        animation.retarget(WidgetryProgressState::Indeterminate);
        assert_eq!(animation.window(), initial);
    }
}
