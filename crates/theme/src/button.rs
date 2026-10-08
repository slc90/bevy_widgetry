use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.normal.background, [255, 255, 255, 255]),
            (LIGHT.normal.border, [217, 217, 217, 255]),
            (LIGHT.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.hovered.background, [245, 245, 245, 255]),
            (LIGHT.hovered.border, [64, 150, 255, 255]),
            (LIGHT.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.pressed.background, [235, 235, 235, 255]),
            (LIGHT.pressed.border, [9, 88, 217, 255]),
            (LIGHT.pressed.foreground, [31, 31, 31, 255]),
            (LIGHT.disabled.background, [245, 245, 245, 255]),
            (LIGHT.disabled.border, [217, 217, 217, 255]),
            (LIGHT.disabled.foreground, [191, 191, 191, 255]),
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
    }
    #[test]
    fn dark_matches_reference() {
        let slots = [
            (DARK.normal.background, [20, 20, 20, 255]),
            (DARK.normal.border, [66, 66, 66, 255]),
            (DARK.normal.foreground, [220, 220, 220, 255]),
            (DARK.hovered.background, [38, 38, 38, 255]),
            (DARK.hovered.border, [60, 137, 232, 255]),
            (DARK.hovered.foreground, [220, 220, 220, 255]),
            (DARK.pressed.background, [48, 48, 48, 255]),
            (DARK.pressed.border, [21, 84, 173, 255]),
            (DARK.pressed.foreground, [220, 220, 220, 255]),
            (DARK.disabled.background, [31, 31, 31, 255]),
            (DARK.disabled.border, [66, 66, 66, 255]),
            (DARK.disabled.foreground, [89, 89, 89, 255]),
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
    }
}
