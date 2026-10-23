use crate::common::{dark, kamuri_violet, light, pink_dream};
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
pub const LIGHT: WidgetryProgressBarColors = WidgetryProgressBarColors {
    track: light::SURFACE,
    fill: light::PRIMARY,
    border: light::BORDER,
};
pub const PINK_DREAM: WidgetryProgressBarColors = WidgetryProgressBarColors {
    track: pink_dream::SURFACE,
    fill: pink_dream::PRIMARY,
    border: pink_dream::BORDER,
};
pub const KAMURI_VIOLET: WidgetryProgressBarColors = WidgetryProgressBarColors {
    track: kamuri_violet::SURFACE,
    fill: kamuri_violet::PRIMARY,
    border: kamuri_violet::BORDER,
};
