use crate::common::dark;

#[cfg(feature = "light")]
use crate::common::light;
#[cfg(feature = "pink_dream")]
use crate::common::pink_dream;
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryCollapsibleColors {
    pub background: Color,
    pub header_background: Color,
    pub content_background: Color,
    pub border: Color,
    pub separator: Color,
}

macro_rules! palette {
    ($name:ident, $base:ident) => {
        pub const $name: WidgetryCollapsibleColors = WidgetryCollapsibleColors {
            background: $base::SURFACE,
            header_background: $base::SURFACE,
            content_background: $base::SURFACE,
            border: $base::BORDER,
            separator: $base::BORDER,
        };
    };
}

#[cfg(feature = "light")]
palette!(LIGHT, light);
palette!(DARK, dark);
#[cfg(feature = "pink_dream")]
palette!(PINK_DREAM, pink_dream);
