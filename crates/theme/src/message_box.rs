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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::color::ColorToPacked;

    #[test]
    fn pink_dream_matches_approved_palette() {
        let slots = [
            (PINK_DREAM.body.normal.background, 0xFFF3F9FFu32),
            (PINK_DREAM.body.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.body.disabled.background, 0xFFF3F9FFu32),
            (PINK_DREAM.body.disabled.foreground, 0xAC98A3FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(PINK_DREAM.window, crate::window::PINK_DREAM);
        assert_eq!(PINK_DREAM.action_button, crate::button::PINK_DREAM);
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.body.normal.background, 0xF7F2FDFFu32),
            (KAMURI_VIOLET.body.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.body.disabled.background, 0xF7F2FDFFu32),
            (KAMURI_VIOLET.body.disabled.foreground, 0xB5A9C1FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(KAMURI_VIOLET.window, crate::window::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.action_button, crate::button::KAMURI_VIOLET);
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.body.normal.background, [255, 255, 255, 255]),
            (LIGHT.body.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.body.disabled.background, [255, 255, 255, 255]),
            (LIGHT.body.disabled.foreground, [191, 191, 191, 255]),
        ];
        for (color, rgba) in slots {
            let actual = color.to_srgba();
            let expected = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3]).to_srgba();
            assert_eq!(actual, expected);
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
    fn dark_matches_reference() {
        let slots = [
            (DARK.body.normal.background, [31, 31, 31, 255]),
            (DARK.body.normal.foreground, [220, 220, 220, 255]),
            (DARK.body.disabled.background, [31, 31, 31, 255]),
            (DARK.body.disabled.foreground, [89, 89, 89, 255]),
        ];
        for (color, rgba) in slots {
            let actual = color.to_srgba();
            let expected = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3]).to_srgba();
            assert_eq!(actual, expected);
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
