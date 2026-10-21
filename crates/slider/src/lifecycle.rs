use bevy::prelude::*;

/// Emitted after capture is established, before a track press changes the value.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct SliderDragStarted {
    pub entity: Entity,
    pub value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderDragOutcome {
    Completed,
    Cancelled,
}

/// Emitted once after gesture cleanup, including gestures with no value changes.
/// Cancellation retains the committed value and does not imply rollback.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct SliderDragEnded {
    pub entity: Entity,
    pub value: f32,
    pub outcome: SliderDragOutcome,
}
