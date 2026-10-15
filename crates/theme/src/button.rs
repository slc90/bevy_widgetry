use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryButtonStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryButtonColors {
    pub normal: WidgetryButtonStateColors,
    pub hovered: WidgetryButtonStateColors,
    pub pressed: WidgetryButtonStateColors,
    pub disabled: WidgetryButtonStateColors,
}

pub const LIGHT: WidgetryButtonColors = WidgetryButtonColors {
    normal: WidgetryButtonStateColors {
        background: light::SURFACE,
        border: light::BORDER,
        foreground: light::TEXT,
    },
    hovered: WidgetryButtonStateColors {
        background: light::HOVER_SURFACE,
        border: light::PRIMARY_HOVER,
        foreground: light::TEXT,
    },
    pressed: WidgetryButtonStateColors {
        background: light::PRESSED_SURFACE,
        border: light::PRIMARY_PRESSED,
        foreground: light::TEXT,
    },
    disabled: WidgetryButtonStateColors {
        background: light::DISABLED_SURFACE,
        border: light::DISABLED_BORDER,
        foreground: light::DISABLED_TEXT,
    },
};

pub const DARK: WidgetryButtonColors = WidgetryButtonColors {
    normal: WidgetryButtonStateColors {
        background: dark::SURFACE,
        border: dark::BORDER,
        foreground: dark::TEXT,
    },
    hovered: WidgetryButtonStateColors {
        background: dark::HOVER_SURFACE,
        border: dark::PRIMARY_HOVER,
        foreground: dark::TEXT,
    },
    pressed: WidgetryButtonStateColors {
        background: dark::PRESSED_SURFACE,
        border: dark::PRIMARY_PRESSED,
        foreground: dark::TEXT,
    },
    disabled: WidgetryButtonStateColors {
        background: dark::DISABLED_SURFACE,
        border: dark::DISABLED_BORDER,
        foreground: dark::DISABLED_TEXT,
    },
};

pub const PINK_DREAM: WidgetryButtonColors = WidgetryButtonColors {
    normal: WidgetryButtonStateColors {
        background: pink_dream::SURFACE,
        border: pink_dream::BORDER,
        foreground: pink_dream::TEXT,
    },
    hovered: WidgetryButtonStateColors {
        background: pink_dream::HOVER_SURFACE,
        border: pink_dream::PRIMARY_HOVER,
        foreground: pink_dream::TEXT,
    },
    pressed: WidgetryButtonStateColors {
        background: pink_dream::PRESSED_SURFACE,
        border: pink_dream::PRIMARY_PRESSED,
        foreground: pink_dream::TEXT,
    },
    disabled: WidgetryButtonStateColors {
        background: pink_dream::DISABLED_SURFACE,
        border: pink_dream::DISABLED_BORDER,
        foreground: pink_dream::DISABLED_TEXT,
    },
};

pub const KAMURI_VIOLET: WidgetryButtonColors = WidgetryButtonColors {
    normal: WidgetryButtonStateColors {
        background: kamuri_violet::SURFACE,
        border: kamuri_violet::BORDER,
        foreground: kamuri_violet::TEXT,
    },
    hovered: WidgetryButtonStateColors {
        background: kamuri_violet::HOVER_SURFACE,
        border: kamuri_violet::PRIMARY_HOVER,
        foreground: kamuri_violet::TEXT,
    },
    pressed: WidgetryButtonStateColors {
        background: kamuri_violet::PRESSED_SURFACE,
        border: kamuri_violet::PRIMARY_PRESSED,
        foreground: kamuri_violet::TEXT,
    },
    disabled: WidgetryButtonStateColors {
        background: kamuri_violet::DISABLED_SURFACE,
        border: kamuri_violet::DISABLED_BORDER,
        foreground: kamuri_violet::DISABLED_TEXT,
    },
};

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.normal.background,
            LIGHT.normal.border,
            LIGHT.normal.foreground,
            LIGHT.hovered.background,
            LIGHT.hovered.border,
            LIGHT.hovered.foreground,
            LIGHT.pressed.background,
            LIGHT.pressed.border,
            LIGHT.pressed.foreground,
            LIGHT.disabled.background,
            LIGHT.disabled.border,
            LIGHT.disabled.foreground,
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
            DARK.normal.background,
            DARK.normal.border,
            DARK.normal.foreground,
            DARK.hovered.background,
            DARK.hovered.border,
            DARK.hovered.foreground,
            DARK.pressed.background,
            DARK.pressed.border,
            DARK.pressed.foreground,
            DARK.disabled.background,
            DARK.disabled.border,
            DARK.disabled.foreground,
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
