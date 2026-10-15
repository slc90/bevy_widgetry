use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryIconStateColors {
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryIconColors {
    pub normal: WidgetryIconStateColors,
    pub disabled: WidgetryIconStateColors,
}

pub const LIGHT: WidgetryIconColors = WidgetryIconColors {
    normal: WidgetryIconStateColors {
        foreground: light::TEXT,
    },
    disabled: WidgetryIconStateColors {
        foreground: light::DISABLED_TEXT,
    },
};

pub const DARK: WidgetryIconColors = WidgetryIconColors {
    normal: WidgetryIconStateColors {
        foreground: dark::TEXT,
    },
    disabled: WidgetryIconStateColors {
        foreground: dark::DISABLED_TEXT,
    },
};

pub const PINK_DREAM: WidgetryIconColors = WidgetryIconColors {
    normal: WidgetryIconStateColors {
        foreground: pink_dream::TEXT,
    },
    disabled: WidgetryIconStateColors {
        foreground: pink_dream::DISABLED_TEXT,
    },
};

pub const KAMURI_VIOLET: WidgetryIconColors = WidgetryIconColors {
    normal: WidgetryIconStateColors {
        foreground: kamuri_violet::TEXT,
    },
    disabled: WidgetryIconStateColors {
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
