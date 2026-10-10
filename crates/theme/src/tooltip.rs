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
            (PINK_DREAM.popup.normal.background, 0x493541FFu32),
            (PINK_DREAM.popup.normal.border, 0x00000000u32),
            (PINK_DREAM.popup.normal.foreground, 0xFFFFFFFFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.popup.normal.background, 0x4A3A5FFFu32),
            (KAMURI_VIOLET.popup.normal.border, 0x00000000u32),
            (KAMURI_VIOLET.popup.normal.foreground, 0xFFFFFFFFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.popup.normal.background, [38, 38, 38, 255]),
            (LIGHT.popup.normal.border, [0, 0, 0, 0]),
            (LIGHT.popup.normal.foreground, [255, 255, 255, 255]),
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
            (DARK.popup.normal.background, [66, 66, 66, 255]),
            (DARK.popup.normal.border, [0, 0, 0, 0]),
            (DARK.popup.normal.foreground, [255, 255, 255, 255]),
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
