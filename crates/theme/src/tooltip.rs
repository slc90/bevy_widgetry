use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
