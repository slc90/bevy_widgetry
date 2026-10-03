use bevy::prelude::*;

#[derive(Component, Clone)]
pub struct WaveformStyle {
    pub palette: Vec<Color>,
    pub line_width: f32,
    pub background: Color,
}

impl Default for WaveformStyle {
    fn default() -> Self {
        Self {
            palette: vec![
                Color::srgb(0.2, 0.8, 0.95),
                Color::srgb(1.0, 0.6, 0.3),
                Color::srgb(0.55, 0.9, 0.4),
                Color::srgb(0.85, 0.45, 0.95),
            ],
            line_width: 1.0,
            background: Color::srgb(0.03, 0.04, 0.06),
        }
    }
}

impl WaveformStyle {
    pub(crate) fn validate(&self) -> Result<(), BevyError> {
        let finite = |color: &Color| {
            color
                .to_linear()
                .to_f32_array()
                .iter()
                .all(|value| value.is_finite())
        };
        if self.palette.len() < 2
            || !self.line_width.is_finite()
            || self.line_width <= 0.0
            || !finite(&self.background)
            || self.palette.iter().any(|color| !finite(color))
            || self
                .palette
                .iter()
                .zip(self.palette.iter().cycle().skip(1))
                .any(|(a, b)| a.to_linear() == b.to_linear())
        {
            return Err(BevyError::error("Invalid WaveformStyle"));
        }
        Ok(())
    }
}
