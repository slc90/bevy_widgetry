use crate::common::dark;

#[cfg(feature = "light")]
use crate::common::light;
#[cfg(feature = "pink_dream")]
use crate::common::pink_dream;
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryProgressBarColors {
    pub track: Color,
    pub fill: Color,
    pub border: Color,
}

pub const DARK: WidgetryProgressBarColors = WidgetryProgressBarColors {
    track: dark::SURFACE,
    fill: dark::PRIMARY,
    border: dark::BORDER,
};
#[cfg(feature = "light")]
pub const LIGHT: WidgetryProgressBarColors = WidgetryProgressBarColors {
    track: light::SURFACE,
    fill: light::PRIMARY,
    border: light::BORDER,
};
#[cfg(feature = "pink_dream")]
pub const PINK_DREAM: WidgetryProgressBarColors = WidgetryProgressBarColors {
    track: pink_dream::SURFACE,
    fill: pink_dream::PRIMARY,
    border: pink_dream::BORDER,
};
