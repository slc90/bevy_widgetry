use crate::button::WidgetryButtonColors;
use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
