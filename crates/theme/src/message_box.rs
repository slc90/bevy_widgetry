use crate::button::WidgetryButtonColors;
use crate::common::{dark, kamuri_violet, light, pink_dream};
use crate::window::WidgetryWindowColors;
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryMessageBoxBodyStateColors {
    pub background: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryMessageBoxBodyColors {
    pub normal: WidgetryMessageBoxBodyStateColors,
    pub disabled: WidgetryMessageBoxBodyStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryMessageBoxColors {
    pub window: WidgetryWindowColors,
    pub body: WidgetryMessageBoxBodyColors,
    pub action_button: WidgetryButtonColors,
}

pub const LIGHT: WidgetryMessageBoxColors = WidgetryMessageBoxColors {
    window: crate::window::LIGHT,
    body: WidgetryMessageBoxBodyColors {
        normal: WidgetryMessageBoxBodyStateColors {
            background: light::ELEVATED_SURFACE,
            foreground: light::TEXT,
        },
        disabled: WidgetryMessageBoxBodyStateColors {
            background: light::ELEVATED_SURFACE,
            foreground: light::DISABLED_TEXT,
        },
    },
    action_button: crate::button::LIGHT,
};

pub const DARK: WidgetryMessageBoxColors = WidgetryMessageBoxColors {
    window: crate::window::DARK,
    body: WidgetryMessageBoxBodyColors {
        normal: WidgetryMessageBoxBodyStateColors {
            background: dark::ELEVATED_SURFACE,
            foreground: dark::TEXT,
        },
        disabled: WidgetryMessageBoxBodyStateColors {
            background: dark::ELEVATED_SURFACE,
            foreground: dark::DISABLED_TEXT,
        },
    },
    action_button: crate::button::DARK,
};

pub const PINK_DREAM: WidgetryMessageBoxColors = WidgetryMessageBoxColors {
    window: crate::window::PINK_DREAM,
    body: WidgetryMessageBoxBodyColors {
        normal: WidgetryMessageBoxBodyStateColors {
            background: pink_dream::ELEVATED_SURFACE,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryMessageBoxBodyStateColors {
            background: pink_dream::ELEVATED_SURFACE,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    action_button: crate::button::PINK_DREAM,
};

pub const KAMURI_VIOLET: WidgetryMessageBoxColors = WidgetryMessageBoxColors {
    window: crate::window::KAMURI_VIOLET,
    body: WidgetryMessageBoxBodyColors {
        normal: WidgetryMessageBoxBodyStateColors {
            background: kamuri_violet::ELEVATED_SURFACE,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryMessageBoxBodyStateColors {
            background: kamuri_violet::ELEVATED_SURFACE,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    action_button: crate::button::KAMURI_VIOLET,
};

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn pink_dream_reuses_widget_colors() {
        assert_eq!(PINK_DREAM.window, crate::window::PINK_DREAM);
        assert_eq!(PINK_DREAM.action_button, crate::button::PINK_DREAM);
    }

    #[test]
    fn kamuri_violet_reuses_widget_colors() {
        assert_eq!(KAMURI_VIOLET.window, crate::window::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.action_button, crate::button::KAMURI_VIOLET);
    }

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.body.normal.background,
            LIGHT.body.normal.foreground,
            LIGHT.body.disabled.background,
            LIGHT.body.disabled.foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
        assert_eq!(LIGHT.window, crate::window::LIGHT);
        assert_eq!(LIGHT.action_button, crate::button::LIGHT);
    }
    #[test]
    fn dark_colors_are_finite() {
        let colors = [
            DARK.body.normal.background,
            DARK.body.normal.foreground,
            DARK.body.disabled.background,
            DARK.body.disabled.foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
        assert_eq!(DARK.window, crate::window::DARK);
        assert_eq!(DARK.action_button, crate::button::DARK);
    }
}
