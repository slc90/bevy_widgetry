use crate::{UiScaleError, UiScaleFailure};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct WidgetryUiScale(f32);

impl Default for WidgetryUiScale {
    fn default() -> Self {
        Self(1.0)
    }
}

impl WidgetryUiScale {
    #[track_caller]
    pub fn new(scale: f32) -> Result<Self, UiScaleFailure> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(UiScaleFailure::new(
                UiScaleError::InvalidScale { requested: scale },
                ErrorFrame::new("ui_scale.validate").field("requested", scale),
            ));
        }
        Ok(Self(scale))
    }

    pub fn get(&self) -> f32 {
        self.0
    }

    #[track_caller]
    pub fn set(&mut self, scale: f32) -> Result<bool, UiScaleFailure> {
        let next = Self::new(scale)?;
        if *self == next {
            return Ok(false);
        }
        *self = next;
        Ok(true)
    }
}
