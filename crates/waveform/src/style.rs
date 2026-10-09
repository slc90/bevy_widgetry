use bevy::prelude::*;

#[derive(Component, Clone)]
pub struct WaveformStyle {
    pub line_width: f32,
}
impl Default for WaveformStyle {
    fn default() -> Self {
        Self { line_width: 1.0 }
    }
}
impl WaveformStyle {
    pub(crate) fn validate(&self) -> Result<(), BevyError> {
        if self.line_width.is_finite() && self.line_width > 0.0 {
            Ok(())
        } else {
            Err(BevyError::error("Invalid WaveformStyle"))
        }
    }
}
