use crate::common::dark;

#[cfg(feature = "light")]
use crate::common::light;
#[cfg(feature = "pink_dream")]
use crate::common::pink_dream;
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryLinkButtonColors {
    pub normal: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub disabled: Color,
}

#[cfg(feature = "light")]
pub const LIGHT: WidgetryLinkButtonColors = WidgetryLinkButtonColors {
    normal: light::PRIMARY,
    hovered: light::PRIMARY_HOVER,
    pressed: light::PRIMARY_PRESSED,
    disabled: light::DISABLED_TEXT,
};

pub const DARK: WidgetryLinkButtonColors = WidgetryLinkButtonColors {
    normal: dark::PRIMARY,
    hovered: dark::PRIMARY_HOVER,
    pressed: dark::PRIMARY_PRESSED,
    disabled: dark::DISABLED_TEXT,
};

#[cfg(feature = "pink_dream")]
pub const PINK_DREAM: WidgetryLinkButtonColors = WidgetryLinkButtonColors {
    normal: pink_dream::PRIMARY,
    hovered: pink_dream::PRIMARY_HOVER,
    pressed: pink_dream::PRIMARY_PRESSED,
    disabled: pink_dream::DISABLED_TEXT,
};
