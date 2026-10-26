use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryDatePickerColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
    pub adjacent: Color,
    pub disabled: Color,
    pub selected: Color,
    pub hovered: Color,
    pub pressed: Color,
}

macro_rules! palette {
    ($name:ident, $base:ident, $selected:expr, $hovered:expr, $pressed:expr) => {
        pub const $name: WidgetryDatePickerColors = WidgetryDatePickerColors {
            background: $base::SURFACE,
            border: $base::BORDER,
            foreground: $base::TEXT,
            adjacent: $base::TERTIARY_TEXT,
            disabled: $base::DISABLED_TEXT,
            selected: $selected,
            hovered: $hovered,
            pressed: $pressed,
        };
    };
}

palette!(
    DARK,
    dark,
    Color::srgb_u8(62, 62, 62),
    Color::srgb_u8(40, 40, 40),
    Color::srgb_u8(50, 50, 50)
);
palette!(
    LIGHT,
    light,
    Color::srgb_u8(224, 224, 224),
    Color::srgb_u8(245, 245, 245),
    Color::srgb_u8(235, 235, 235)
);
palette!(
    PINK_DREAM,
    pink_dream,
    Color::srgb_u8(224, 224, 224),
    Color::srgb_u8(245, 245, 245),
    Color::srgb_u8(235, 235, 235)
);
palette!(
    KAMURI_VIOLET,
    kamuri_violet,
    Color::srgb_u8(224, 224, 224),
    Color::srgb_u8(245, 245, 245),
    Color::srgb_u8(235, 235, 235)
);
