use crate::{SliderFailure, WidgetrySliderState};
use bevy::{input::keyboard::KeyboardInput, prelude::*};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct SliderPointerInput {
    pub entity: Entity,
    pub position: Vec2,
    pub state: WidgetrySliderState,
}

pub enum SliderProjection {
    Position(f64),
    Wait,
    Cancel,
}

/// Positions are continuous coordinates in the numeric range, before library stepping.
/// A session owns grab offsets and trajectory history, and ends with the library gesture.
pub trait SliderDragStrategy: Send + Sync + 'static {
    fn project(
        &mut self,
        world: &World,
        input: SliderPointerInput,
    ) -> Result<SliderProjection, SliderFailure>;
}

pub struct SliderInteractionStart {
    pub position: f64,
    pub drag: Box<dyn SliderDragStrategy>,
}

#[derive(Clone, Copy)]
pub enum SliderDirection {
    Increase,
    Decrease,
}

/// Returning None declines a press or leaves a key unhandled.
/// Layout that is not ready may decline a press or return Wait during a gesture.
pub trait SliderInteractionStrategy: Send + Sync + 'static {
    fn begin(
        &self,
        world: &World,
        input: SliderPointerInput,
    ) -> Result<Option<SliderInteractionStart>, SliderFailure>;

    fn key(
        &self,
        _world: &World,
        _entity: Entity,
        _input: &KeyboardInput,
    ) -> Result<Option<SliderDirection>, SliderFailure> {
        Ok(None)
    }
}

#[derive(Component, Clone)]
#[require(crate::input::DragState, Node, bevy::picking::hover::Hovered)]
pub struct WidgetrySliderInteraction(pub(crate) Arc<dyn SliderInteractionStrategy>);

impl WidgetrySliderInteraction {
    pub fn new(strategy: impl SliderInteractionStrategy) -> Self {
        Self(Arc::new(strategy))
    }

    pub(crate) fn linear() -> Self {
        Self::new(crate::linear::LinearStrategy)
    }
}
