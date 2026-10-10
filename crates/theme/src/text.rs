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
            (PINK_DREAM.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.disabled.foreground, 0xAC98A3FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.disabled.foreground, 0xB5A9C1FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.normal.foreground, [31, 31, 31, 255]),
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
            (DARK.normal.foreground, [220, 220, 220, 255]),
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
