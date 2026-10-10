use crate::common::{dark, kamuri_violet, light, pink_dream};
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

pub const PINK_DREAM: WidgetryCheckBoxColors = WidgetryCheckBoxColors {
    unchecked: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
            mark: pink_dream::TRANSPARENT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::PRIMARY_HOVER,
            foreground: pink_dream::TEXT,
            mark: pink_dream::TRANSPARENT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: pink_dream::PRESSED_SURFACE,
            border: pink_dream::PRIMARY_PRESSED,
            foreground: pink_dream::TEXT,
            mark: pink_dream::TRANSPARENT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
            mark: pink_dream::TRANSPARENT,
        },
    },
    checked: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: pink_dream::PRIMARY,
            border: pink_dream::PRIMARY,
            foreground: pink_dream::TEXT,
            mark: pink_dream::INVERSE_TEXT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: pink_dream::PRIMARY_HOVER,
            border: pink_dream::PRIMARY_HOVER,
            foreground: pink_dream::TEXT,
            mark: pink_dream::INVERSE_TEXT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: pink_dream::PRIMARY_PRESSED,
            border: pink_dream::PRIMARY_PRESSED,
            foreground: pink_dream::TEXT,
            mark: pink_dream::INVERSE_TEXT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
            mark: pink_dream::DISABLED_TEXT,
        },
    },
    indeterminate: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: pink_dream::PRIMARY,
            border: pink_dream::PRIMARY,
            foreground: pink_dream::TEXT,
            mark: pink_dream::INVERSE_TEXT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: pink_dream::PRIMARY_HOVER,
            border: pink_dream::PRIMARY_HOVER,
            foreground: pink_dream::TEXT,
            mark: pink_dream::INVERSE_TEXT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: pink_dream::PRIMARY_PRESSED,
            border: pink_dream::PRIMARY_PRESSED,
            foreground: pink_dream::TEXT,
            mark: pink_dream::INVERSE_TEXT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
            mark: pink_dream::DISABLED_TEXT,
        },
    },
};

pub const KAMURI_VIOLET: WidgetryCheckBoxColors = WidgetryCheckBoxColors {
    unchecked: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::TRANSPARENT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::PRIMARY_HOVER,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::TRANSPARENT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: kamuri_violet::PRESSED_SURFACE,
            border: kamuri_violet::PRIMARY_PRESSED,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::TRANSPARENT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
            mark: kamuri_violet::TRANSPARENT,
        },
    },
    checked: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: kamuri_violet::PRIMARY,
            border: kamuri_violet::PRIMARY,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::INVERSE_TEXT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: kamuri_violet::PRIMARY_HOVER,
            border: kamuri_violet::PRIMARY_HOVER,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::INVERSE_TEXT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: kamuri_violet::PRIMARY_PRESSED,
            border: kamuri_violet::PRIMARY_PRESSED,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::INVERSE_TEXT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
            mark: kamuri_violet::DISABLED_TEXT,
        },
    },
    indeterminate: WidgetryCheckBoxInteractionColors {
        normal: WidgetryCheckBoxStateColors {
            background: kamuri_violet::PRIMARY,
            border: kamuri_violet::PRIMARY,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::INVERSE_TEXT,
        },
        hovered: WidgetryCheckBoxStateColors {
            background: kamuri_violet::PRIMARY_HOVER,
            border: kamuri_violet::PRIMARY_HOVER,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::INVERSE_TEXT,
        },
        pressed: WidgetryCheckBoxStateColors {
            background: kamuri_violet::PRIMARY_PRESSED,
            border: kamuri_violet::PRIMARY_PRESSED,
            foreground: kamuri_violet::TEXT,
            mark: kamuri_violet::INVERSE_TEXT,
        },
        disabled: WidgetryCheckBoxStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
            mark: kamuri_violet::DISABLED_TEXT,
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
            (PINK_DREAM.unchecked.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.unchecked.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.unchecked.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.unchecked.normal.mark, 0x00000000u32),
            (PINK_DREAM.unchecked.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.unchecked.hovered.border, 0xCD6196FFu32),
            (PINK_DREAM.unchecked.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.unchecked.hovered.mark, 0x00000000u32),
            (PINK_DREAM.unchecked.pressed.background, 0xF2CFE2FFu32),
            (PINK_DREAM.unchecked.pressed.border, 0x913063FFu32),
            (PINK_DREAM.unchecked.pressed.foreground, 0x422B3CFFu32),
            (PINK_DREAM.unchecked.pressed.mark, 0x00000000u32),
            (PINK_DREAM.unchecked.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.unchecked.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.unchecked.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.unchecked.disabled.mark, 0x00000000u32),
            (PINK_DREAM.checked.normal.background, 0xB4437DFFu32),
            (PINK_DREAM.checked.normal.border, 0xB4437DFFu32),
            (PINK_DREAM.checked.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.checked.normal.mark, 0xFFFFFFFFu32),
            (PINK_DREAM.checked.hovered.background, 0xCD6196FFu32),
            (PINK_DREAM.checked.hovered.border, 0xCD6196FFu32),
            (PINK_DREAM.checked.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.checked.hovered.mark, 0xFFFFFFFFu32),
            (PINK_DREAM.checked.pressed.background, 0x913063FFu32),
            (PINK_DREAM.checked.pressed.border, 0x913063FFu32),
            (PINK_DREAM.checked.pressed.foreground, 0x422B3CFFu32),
            (PINK_DREAM.checked.pressed.mark, 0xFFFFFFFFu32),
            (PINK_DREAM.checked.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.checked.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.checked.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.checked.disabled.mark, 0xAC98A3FFu32),
            (PINK_DREAM.indeterminate.normal.background, 0xB4437DFFu32),
            (PINK_DREAM.indeterminate.normal.border, 0xB4437DFFu32),
            (PINK_DREAM.indeterminate.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.indeterminate.normal.mark, 0xFFFFFFFFu32),
            (PINK_DREAM.indeterminate.hovered.background, 0xCD6196FFu32),
            (PINK_DREAM.indeterminate.hovered.border, 0xCD6196FFu32),
            (PINK_DREAM.indeterminate.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.indeterminate.hovered.mark, 0xFFFFFFFFu32),
            (PINK_DREAM.indeterminate.pressed.background, 0x913063FFu32),
            (PINK_DREAM.indeterminate.pressed.border, 0x913063FFu32),
            (PINK_DREAM.indeterminate.pressed.foreground, 0x422B3CFFu32),
            (PINK_DREAM.indeterminate.pressed.mark, 0xFFFFFFFFu32),
            (PINK_DREAM.indeterminate.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.indeterminate.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.indeterminate.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.indeterminate.disabled.mark, 0xAC98A3FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.unchecked.normal.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.unchecked.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.unchecked.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.unchecked.normal.mark, 0x00000000u32),
            (KAMURI_VIOLET.unchecked.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.unchecked.hovered.border, 0xA181D2FFu32),
            (KAMURI_VIOLET.unchecked.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.unchecked.hovered.mark, 0x00000000u32),
            (KAMURI_VIOLET.unchecked.pressed.background, 0xDBCBECFFu32),
            (KAMURI_VIOLET.unchecked.pressed.border, 0x6E4EA3FFu32),
            (KAMURI_VIOLET.unchecked.pressed.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.unchecked.pressed.mark, 0x00000000u32),
            (KAMURI_VIOLET.unchecked.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.unchecked.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.unchecked.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.unchecked.disabled.mark, 0x00000000u32),
            (KAMURI_VIOLET.checked.normal.background, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.checked.normal.border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.checked.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.checked.normal.mark, 0xFFFFFFFFu32),
            (KAMURI_VIOLET.checked.hovered.background, 0xA181D2FFu32),
            (KAMURI_VIOLET.checked.hovered.border, 0xA181D2FFu32),
            (KAMURI_VIOLET.checked.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.checked.hovered.mark, 0xFFFFFFFFu32),
            (KAMURI_VIOLET.checked.pressed.background, 0x6E4EA3FFu32),
            (KAMURI_VIOLET.checked.pressed.border, 0x6E4EA3FFu32),
            (KAMURI_VIOLET.checked.pressed.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.checked.pressed.mark, 0xFFFFFFFFu32),
            (KAMURI_VIOLET.checked.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.checked.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.checked.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.checked.disabled.mark, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.indeterminate.normal.background, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.indeterminate.normal.border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.indeterminate.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.indeterminate.normal.mark, 0xFFFFFFFFu32),
            (
                KAMURI_VIOLET.indeterminate.hovered.background,
                0xA181D2FFu32,
            ),
            (KAMURI_VIOLET.indeterminate.hovered.border, 0xA181D2FFu32),
            (
                KAMURI_VIOLET.indeterminate.hovered.foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.indeterminate.hovered.mark, 0xFFFFFFFFu32),
            (
                KAMURI_VIOLET.indeterminate.pressed.background,
                0x6E4EA3FFu32,
            ),
            (KAMURI_VIOLET.indeterminate.pressed.border, 0x6E4EA3FFu32),
            (
                KAMURI_VIOLET.indeterminate.pressed.foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.indeterminate.pressed.mark, 0xFFFFFFFFu32),
            (
                KAMURI_VIOLET.indeterminate.disabled.background,
                0xE8E0F0FFu32,
            ),
            (KAMURI_VIOLET.indeterminate.disabled.border, 0xE7DFF0FFu32),
            (
                KAMURI_VIOLET.indeterminate.disabled.foreground,
                0xB5A9C1FFu32,
            ),
            (KAMURI_VIOLET.indeterminate.disabled.mark, 0xB5A9C1FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

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
