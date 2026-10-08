use crate::common::{dark, light};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryCheckBoxStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
    pub mark: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryCheckBoxInteractionColors {
    pub normal: WidgetryCheckBoxStateColors,
    pub hovered: WidgetryCheckBoxStateColors,
    pub pressed: WidgetryCheckBoxStateColors,
    pub disabled: WidgetryCheckBoxStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryCheckBoxColors {
    pub unchecked: WidgetryCheckBoxInteractionColors,
    pub checked: WidgetryCheckBoxInteractionColors,
    pub indeterminate: WidgetryCheckBoxInteractionColors,
}

pub const LIGHT: WidgetryCheckBoxColors = WidgetryCheckBoxColors {
    unchecked: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: light::SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
            mark: light::TRANSPARENT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: light::HOVER_SURFACE,
            border: light::PRIMARY_HOVER,
            foreground: light::TEXT,
            mark: light::TRANSPARENT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: light::PRESSED_SURFACE,
            border: light::PRIMARY_PRESSED,
            foreground: light::TEXT,
            mark: light::TRANSPARENT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
            mark: light::TRANSPARENT,
        },
    },
    checked: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: light::PRIMARY,
            border: light::PRIMARY,
            foreground: light::TEXT,
            mark: light::INVERSE_TEXT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: light::PRIMARY_HOVER,
            border: light::PRIMARY_HOVER,
            foreground: light::TEXT,
            mark: light::INVERSE_TEXT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: light::PRIMARY_PRESSED,
            border: light::PRIMARY_PRESSED,
            foreground: light::TEXT,
            mark: light::INVERSE_TEXT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
            mark: light::DISABLED_TEXT,
        },
    },
    indeterminate: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: light::PRIMARY,
            border: light::PRIMARY,
            foreground: light::TEXT,
            mark: light::INVERSE_TEXT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: light::PRIMARY_HOVER,
            border: light::PRIMARY_HOVER,
            foreground: light::TEXT,
            mark: light::INVERSE_TEXT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: light::PRIMARY_PRESSED,
            border: light::PRIMARY_PRESSED,
            foreground: light::TEXT,
            mark: light::INVERSE_TEXT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
            mark: light::DISABLED_TEXT,
        },
    },
};

pub const DARK: WidgetryCheckBoxColors = WidgetryCheckBoxColors {
    unchecked: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: dark::SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
            mark: dark::TRANSPARENT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::PRIMARY_HOVER,
            foreground: dark::TEXT,
            mark: dark::TRANSPARENT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: dark::PRESSED_SURFACE,
            border: dark::PRIMARY_PRESSED,
            foreground: dark::TEXT,
            mark: dark::TRANSPARENT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
            mark: dark::TRANSPARENT,
        },
    },
    checked: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: dark::PRIMARY,
            border: dark::PRIMARY,
            foreground: dark::TEXT,
            mark: dark::INVERSE_TEXT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: dark::PRIMARY_HOVER,
            border: dark::PRIMARY_HOVER,
            foreground: dark::TEXT,
            mark: dark::INVERSE_TEXT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: dark::PRIMARY_PRESSED,
            border: dark::PRIMARY_PRESSED,
            foreground: dark::TEXT,
            mark: dark::INVERSE_TEXT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
            mark: dark::DISABLED_TEXT,
        },
    },
    indeterminate: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: dark::PRIMARY,
            border: dark::PRIMARY,
            foreground: dark::TEXT,
            mark: dark::INVERSE_TEXT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: dark::PRIMARY_HOVER,
            border: dark::PRIMARY_HOVER,
            foreground: dark::TEXT,
            mark: dark::INVERSE_TEXT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: dark::PRIMARY_PRESSED,
            border: dark::PRIMARY_PRESSED,
            foreground: dark::TEXT,
            mark: dark::INVERSE_TEXT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
            mark: dark::DISABLED_TEXT,
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
            (LIGHT.unchecked.normal.background, [255, 255, 255, 255]),
            (LIGHT.unchecked.normal.border, [217, 217, 217, 255]),
            (LIGHT.unchecked.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.unchecked.normal.mark, [0, 0, 0, 0]),
            (LIGHT.unchecked.hovered.background, [245, 245, 245, 255]),
            (LIGHT.unchecked.hovered.border, [64, 150, 255, 255]),
            (LIGHT.unchecked.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.unchecked.hovered.mark, [0, 0, 0, 0]),
            (LIGHT.unchecked.pressed.background, [235, 235, 235, 255]),
            (LIGHT.unchecked.pressed.border, [9, 88, 217, 255]),
            (LIGHT.unchecked.pressed.foreground, [31, 31, 31, 255]),
            (LIGHT.unchecked.pressed.mark, [0, 0, 0, 0]),
            (LIGHT.unchecked.disabled.background, [245, 245, 245, 255]),
            (LIGHT.unchecked.disabled.border, [217, 217, 217, 255]),
            (LIGHT.unchecked.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.unchecked.disabled.mark, [0, 0, 0, 0]),
            (LIGHT.checked.normal.background, [22, 119, 255, 255]),
            (LIGHT.checked.normal.border, [22, 119, 255, 255]),
            (LIGHT.checked.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.checked.normal.mark, [255, 255, 255, 255]),
            (LIGHT.checked.hovered.background, [64, 150, 255, 255]),
            (LIGHT.checked.hovered.border, [64, 150, 255, 255]),
            (LIGHT.checked.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.checked.hovered.mark, [255, 255, 255, 255]),
            (LIGHT.checked.pressed.background, [9, 88, 217, 255]),
            (LIGHT.checked.pressed.border, [9, 88, 217, 255]),
            (LIGHT.checked.pressed.foreground, [31, 31, 31, 255]),
            (LIGHT.checked.pressed.mark, [255, 255, 255, 255]),
            (LIGHT.checked.disabled.background, [245, 245, 245, 255]),
            (LIGHT.checked.disabled.border, [217, 217, 217, 255]),
            (LIGHT.checked.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.checked.disabled.mark, [191, 191, 191, 255]),
            (LIGHT.indeterminate.normal.background, [22, 119, 255, 255]),
            (LIGHT.indeterminate.normal.border, [22, 119, 255, 255]),
            (LIGHT.indeterminate.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.indeterminate.normal.mark, [255, 255, 255, 255]),
            (LIGHT.indeterminate.hovered.background, [64, 150, 255, 255]),
            (LIGHT.indeterminate.hovered.border, [64, 150, 255, 255]),
            (LIGHT.indeterminate.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.indeterminate.hovered.mark, [255, 255, 255, 255]),
            (LIGHT.indeterminate.pressed.background, [9, 88, 217, 255]),
            (LIGHT.indeterminate.pressed.border, [9, 88, 217, 255]),
            (LIGHT.indeterminate.pressed.foreground, [31, 31, 31, 255]),
            (LIGHT.indeterminate.pressed.mark, [255, 255, 255, 255]),
            (
                LIGHT.indeterminate.disabled.background,
                [245, 245, 245, 255],
            ),
            (LIGHT.indeterminate.disabled.border, [217, 217, 217, 255]),
            (
                LIGHT.indeterminate.disabled.foreground,
                [191, 191, 191, 255],
            ),
            (LIGHT.indeterminate.disabled.mark, [191, 191, 191, 255]),
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
            (DARK.unchecked.normal.background, [20, 20, 20, 255]),
            (DARK.unchecked.normal.border, [66, 66, 66, 255]),
            (DARK.unchecked.normal.foreground, [220, 220, 220, 255]),
            (DARK.unchecked.normal.mark, [0, 0, 0, 0]),
            (DARK.unchecked.hovered.background, [38, 38, 38, 255]),
            (DARK.unchecked.hovered.border, [60, 137, 232, 255]),
            (DARK.unchecked.hovered.foreground, [220, 220, 220, 255]),
            (DARK.unchecked.hovered.mark, [0, 0, 0, 0]),
            (DARK.unchecked.pressed.background, [48, 48, 48, 255]),
            (DARK.unchecked.pressed.border, [21, 84, 173, 255]),
            (DARK.unchecked.pressed.foreground, [220, 220, 220, 255]),
            (DARK.unchecked.pressed.mark, [0, 0, 0, 0]),
            (DARK.unchecked.disabled.background, [31, 31, 31, 255]),
            (DARK.unchecked.disabled.border, [66, 66, 66, 255]),
            (DARK.unchecked.disabled.foreground, [89, 89, 89, 255]),
            (DARK.unchecked.disabled.mark, [0, 0, 0, 0]),
            (DARK.checked.normal.background, [22, 104, 220, 255]),
            (DARK.checked.normal.border, [22, 104, 220, 255]),
            (DARK.checked.normal.foreground, [220, 220, 220, 255]),
            (DARK.checked.normal.mark, [255, 255, 255, 255]),
            (DARK.checked.hovered.background, [60, 137, 232, 255]),
            (DARK.checked.hovered.border, [60, 137, 232, 255]),
            (DARK.checked.hovered.foreground, [220, 220, 220, 255]),
            (DARK.checked.hovered.mark, [255, 255, 255, 255]),
            (DARK.checked.pressed.background, [21, 84, 173, 255]),
            (DARK.checked.pressed.border, [21, 84, 173, 255]),
            (DARK.checked.pressed.foreground, [220, 220, 220, 255]),
            (DARK.checked.pressed.mark, [255, 255, 255, 255]),
            (DARK.checked.disabled.background, [31, 31, 31, 255]),
            (DARK.checked.disabled.border, [66, 66, 66, 255]),
            (DARK.checked.disabled.foreground, [89, 89, 89, 255]),
            (DARK.checked.disabled.mark, [89, 89, 89, 255]),
            (DARK.indeterminate.normal.background, [22, 104, 220, 255]),
            (DARK.indeterminate.normal.border, [22, 104, 220, 255]),
            (DARK.indeterminate.normal.foreground, [220, 220, 220, 255]),
            (DARK.indeterminate.normal.mark, [255, 255, 255, 255]),
            (DARK.indeterminate.hovered.background, [60, 137, 232, 255]),
            (DARK.indeterminate.hovered.border, [60, 137, 232, 255]),
            (DARK.indeterminate.hovered.foreground, [220, 220, 220, 255]),
            (DARK.indeterminate.hovered.mark, [255, 255, 255, 255]),
            (DARK.indeterminate.pressed.background, [21, 84, 173, 255]),
            (DARK.indeterminate.pressed.border, [21, 84, 173, 255]),
            (DARK.indeterminate.pressed.foreground, [220, 220, 220, 255]),
            (DARK.indeterminate.pressed.mark, [255, 255, 255, 255]),
            (DARK.indeterminate.disabled.background, [31, 31, 31, 255]),
            (DARK.indeterminate.disabled.border, [66, 66, 66, 255]),
            (DARK.indeterminate.disabled.foreground, [89, 89, 89, 255]),
            (DARK.indeterminate.disabled.mark, [89, 89, 89, 255]),
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
