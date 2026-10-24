use crate::common::{dark, kamuri_violet, light, pink_dream};
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

palette!(LIGHT, light);
palette!(DARK, dark);
palette!(PINK_DREAM, pink_dream);
palette!(KAMURI_VIOLET, kamuri_violet);
