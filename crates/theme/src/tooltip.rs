use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTooltipStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTooltipPopupColors {
    pub normal: WidgetryTooltipStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTooltipColors {
    pub popup: WidgetryTooltipPopupColors,
}

pub const LIGHT: WidgetryTooltipColors = WidgetryTooltipColors {
    popup: WidgetryTooltipPopupColors {
        normal: WidgetryTooltipStateColors {
            background: light::TOOLTIP_BACKGROUND,
            border: light::TRANSPARENT,
            foreground: light::INVERSE_TEXT,
        },
    },
};

pub const DARK: WidgetryTooltipColors = WidgetryTooltipColors {
    popup: WidgetryTooltipPopupColors {
        normal: WidgetryTooltipStateColors {
            background: dark::TOOLTIP_BACKGROUND,
            border: dark::TRANSPARENT,
            foreground: dark::INVERSE_TEXT,
        },
    },
};

pub const PINK_DREAM: WidgetryTooltipColors = WidgetryTooltipColors {
    popup: WidgetryTooltipPopupColors {
        normal: WidgetryTooltipStateColors {
            background: pink_dream::TOOLTIP_BACKGROUND,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::INVERSE_TEXT,
        },
    },
};

pub const KAMURI_VIOLET: WidgetryTooltipColors = WidgetryTooltipColors {
    popup: WidgetryTooltipPopupColors {
        normal: WidgetryTooltipStateColors {
            background: kamuri_violet::TOOLTIP_BACKGROUND,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::INVERSE_TEXT,
        },
    },
};

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.popup.normal.background,
            LIGHT.popup.normal.border,
            LIGHT.popup.normal.foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
    #[test]
    fn dark_colors_are_finite() {
        let colors = [
            DARK.popup.normal.background,
            DARK.popup.normal.border,
            DARK.popup.normal.foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
}
