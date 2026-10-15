use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTextStateColors {
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTextColors {
    pub normal: WidgetryTextStateColors,
    pub disabled: WidgetryTextStateColors,
}

pub const LIGHT: WidgetryTextColors = WidgetryTextColors {
    normal: WidgetryTextStateColors {
        foreground: light::TEXT,
    },
    disabled: WidgetryTextStateColors {
        foreground: light::DISABLED_TEXT,
    },
};

pub const DARK: WidgetryTextColors = WidgetryTextColors {
    normal: WidgetryTextStateColors {
        foreground: dark::TEXT,
    },
    disabled: WidgetryTextStateColors {
        foreground: dark::DISABLED_TEXT,
    },
};

pub const PINK_DREAM: WidgetryTextColors = WidgetryTextColors {
    normal: WidgetryTextStateColors {
        foreground: pink_dream::TEXT,
    },
    disabled: WidgetryTextStateColors {
        foreground: pink_dream::DISABLED_TEXT,
    },
};

pub const KAMURI_VIOLET: WidgetryTextColors = WidgetryTextColors {
    normal: WidgetryTextStateColors {
        foreground: kamuri_violet::TEXT,
    },
    disabled: WidgetryTextStateColors {
        foreground: kamuri_violet::DISABLED_TEXT,
    },
};

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn light_colors_are_finite() {
        let colors = [LIGHT.normal.foreground, LIGHT.disabled.foreground];
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
        let colors = [DARK.normal.foreground, DARK.disabled.foreground];
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
