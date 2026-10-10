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
            (PINK_DREAM.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.hovered.border, 0xCD6196FFu32),
            (PINK_DREAM.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.pressed.background, 0xF2CFE2FFu32),
            (PINK_DREAM.pressed.border, 0x913063FFu32),
            (PINK_DREAM.pressed.foreground, 0x422B3CFFu32),
            (PINK_DREAM.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.disabled.foreground, 0xAC98A3FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.normal.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.hovered.border, 0xA181D2FFu32),
            (KAMURI_VIOLET.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.pressed.background, 0xDBCBECFFu32),
            (KAMURI_VIOLET.pressed.border, 0x6E4EA3FFu32),
            (KAMURI_VIOLET.pressed.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.disabled.foreground, 0xB5A9C1FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

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
