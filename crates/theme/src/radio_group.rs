use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioGroupContainerStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioGroupContainerColors {
    pub normal: WidgetryRadioGroupContainerStateColors,
    pub focused: WidgetryRadioGroupContainerStateColors,
    pub disabled: WidgetryRadioGroupContainerStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioOptionStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
    pub dot: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioOptionInteractionColors {
    pub normal: WidgetryRadioOptionStateColors,
    pub hovered: WidgetryRadioOptionStateColors,
    pub disabled: WidgetryRadioOptionStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioOptionColors {
    pub unchecked: WidgetryRadioOptionInteractionColors,
    pub checked: WidgetryRadioOptionInteractionColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioGroupColors {
    pub container: WidgetryRadioGroupContainerColors,
    pub option: WidgetryRadioOptionColors,
}

pub const LIGHT: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
    container: WidgetryRadioGroupContainerColors {
        normal: WidgetryRadioGroupContainerStateColors {
            background: light::SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
        },
        focused: WidgetryRadioGroupContainerStateColors {
            background: light::SURFACE,
            border: light::FOCUS_BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryRadioGroupContainerStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
    },
    option: WidgetryRadioOptionColors {
        unchecked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: light::SURFACE,
                border: light::BORDER,
                foreground: light::TEXT,
                dot: light::TRANSPARENT,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: light::SURFACE,
                border: light::PRIMARY_HOVER,
                foreground: light::TEXT,
                dot: light::TRANSPARENT,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: light::DISABLED_SURFACE,
                border: light::DISABLED_BORDER,
                foreground: light::DISABLED_TEXT,
                dot: light::TRANSPARENT,
            },
        },
        checked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: light::SURFACE,
                border: light::PRIMARY,
                foreground: light::TEXT,
                dot: light::PRIMARY,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: light::SURFACE,
                border: light::PRIMARY_HOVER,
                foreground: light::TEXT,
                dot: light::PRIMARY_HOVER,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: light::DISABLED_SURFACE,
                border: light::DISABLED_BORDER,
                foreground: light::DISABLED_TEXT,
                dot: light::DISABLED_TEXT,
            },
        },
    },
};

pub const DARK: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
    container: WidgetryRadioGroupContainerColors {
        normal: WidgetryRadioGroupContainerStateColors {
            background: dark::SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
        },
        focused: WidgetryRadioGroupContainerStateColors {
            background: dark::SURFACE,
            border: dark::FOCUS_BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryRadioGroupContainerStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
    },
    option: WidgetryRadioOptionColors {
        unchecked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: dark::SURFACE,
                border: dark::BORDER,
                foreground: dark::TEXT,
                dot: dark::TRANSPARENT,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: dark::SURFACE,
                border: dark::PRIMARY_HOVER,
                foreground: dark::TEXT,
                dot: dark::TRANSPARENT,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: dark::DISABLED_SURFACE,
                border: dark::DISABLED_BORDER,
                foreground: dark::DISABLED_TEXT,
                dot: dark::TRANSPARENT,
            },
        },
        checked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: dark::SURFACE,
                border: dark::PRIMARY,
                foreground: dark::TEXT,
                dot: dark::PRIMARY,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: dark::SURFACE,
                border: dark::PRIMARY_HOVER,
                foreground: dark::TEXT,
                dot: dark::PRIMARY_HOVER,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: dark::DISABLED_SURFACE,
                border: dark::DISABLED_BORDER,
                foreground: dark::DISABLED_TEXT,
                dot: dark::DISABLED_TEXT,
            },
        },
    },
};

pub const PINK_DREAM: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
    container: WidgetryRadioGroupContainerColors {
        normal: WidgetryRadioGroupContainerStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
        },
        focused: WidgetryRadioGroupContainerStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::FOCUS_BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryRadioGroupContainerStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    option: WidgetryRadioOptionColors {
        unchecked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: pink_dream::SURFACE,
                border: pink_dream::BORDER,
                foreground: pink_dream::TEXT,
                dot: pink_dream::TRANSPARENT,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: pink_dream::SURFACE,
                border: pink_dream::PRIMARY_HOVER,
                foreground: pink_dream::TEXT,
                dot: pink_dream::TRANSPARENT,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: pink_dream::DISABLED_SURFACE,
                border: pink_dream::DISABLED_BORDER,
                foreground: pink_dream::DISABLED_TEXT,
                dot: pink_dream::TRANSPARENT,
            },
        },
        checked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: pink_dream::SURFACE,
                border: pink_dream::PRIMARY,
                foreground: pink_dream::TEXT,
                dot: pink_dream::PRIMARY,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: pink_dream::SURFACE,
                border: pink_dream::PRIMARY_HOVER,
                foreground: pink_dream::TEXT,
                dot: pink_dream::PRIMARY_HOVER,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: pink_dream::DISABLED_SURFACE,
                border: pink_dream::DISABLED_BORDER,
                foreground: pink_dream::DISABLED_TEXT,
                dot: pink_dream::DISABLED_TEXT,
            },
        },
    },
};

pub const KAMURI_VIOLET: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
    container: WidgetryRadioGroupContainerColors {
        normal: WidgetryRadioGroupContainerStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
        },
        focused: WidgetryRadioGroupContainerStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::FOCUS_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryRadioGroupContainerStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    option: WidgetryRadioOptionColors {
        unchecked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
                dot: kamuri_violet::TRANSPARENT,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                dot: kamuri_violet::TRANSPARENT,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                dot: kamuri_violet::TRANSPARENT,
            },
        },
        checked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::PRIMARY,
                foreground: kamuri_violet::TEXT,
                dot: kamuri_violet::PRIMARY,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                dot: kamuri_violet::PRIMARY_HOVER,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                dot: kamuri_violet::DISABLED_TEXT,
            },
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
            (PINK_DREAM.container.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.container.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.container.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.container.focused.background, 0xFCEAF3FFu32),
            (PINK_DREAM.container.focused.border, 0xB4437DFFu32),
            (PINK_DREAM.container.focused.foreground, 0x422B3CFFu32),
            (PINK_DREAM.container.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.container.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.container.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.option.unchecked.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.option.unchecked.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.option.unchecked.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.option.unchecked.normal.dot, 0x00000000u32),
            (
                PINK_DREAM.option.unchecked.hovered.background,
                0xFCEAF3FFu32,
            ),
            (PINK_DREAM.option.unchecked.hovered.border, 0xCD6196FFu32),
            (
                PINK_DREAM.option.unchecked.hovered.foreground,
                0x422B3CFFu32,
            ),
            (PINK_DREAM.option.unchecked.hovered.dot, 0x00000000u32),
            (
                PINK_DREAM.option.unchecked.disabled.background,
                0xF1E1E9FFu32,
            ),
            (PINK_DREAM.option.unchecked.disabled.border, 0xE5D9E0FFu32),
            (
                PINK_DREAM.option.unchecked.disabled.foreground,
                0xAC98A3FFu32,
            ),
            (PINK_DREAM.option.unchecked.disabled.dot, 0x00000000u32),
            (PINK_DREAM.option.checked.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.option.checked.normal.border, 0xB4437DFFu32),
            (PINK_DREAM.option.checked.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.option.checked.normal.dot, 0xB4437DFFu32),
            (PINK_DREAM.option.checked.hovered.background, 0xFCEAF3FFu32),
            (PINK_DREAM.option.checked.hovered.border, 0xCD6196FFu32),
            (PINK_DREAM.option.checked.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.option.checked.hovered.dot, 0xCD6196FFu32),
            (PINK_DREAM.option.checked.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.option.checked.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.option.checked.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.option.checked.disabled.dot, 0xAC98A3FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.container.normal.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.container.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.container.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.container.focused.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.container.focused.border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.container.focused.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.container.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.container.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.container.disabled.foreground, 0xB5A9C1FFu32),
            (
                KAMURI_VIOLET.option.unchecked.normal.background,
                0xF0E8FAFFu32,
            ),
            (KAMURI_VIOLET.option.unchecked.normal.border, 0xD8CDE4FFu32),
            (
                KAMURI_VIOLET.option.unchecked.normal.foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.option.unchecked.normal.dot, 0x00000000u32),
            (
                KAMURI_VIOLET.option.unchecked.hovered.background,
                0xF0E8FAFFu32,
            ),
            (KAMURI_VIOLET.option.unchecked.hovered.border, 0xA181D2FFu32),
            (
                KAMURI_VIOLET.option.unchecked.hovered.foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.option.unchecked.hovered.dot, 0x00000000u32),
            (
                KAMURI_VIOLET.option.unchecked.disabled.background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.option.unchecked.disabled.border,
                0xE7DFF0FFu32,
            ),
            (
                KAMURI_VIOLET.option.unchecked.disabled.foreground,
                0xB5A9C1FFu32,
            ),
            (KAMURI_VIOLET.option.unchecked.disabled.dot, 0x00000000u32),
            (
                KAMURI_VIOLET.option.checked.normal.background,
                0xF0E8FAFFu32,
            ),
            (KAMURI_VIOLET.option.checked.normal.border, 0x8C6BC1FFu32),
            (
                KAMURI_VIOLET.option.checked.normal.foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.option.checked.normal.dot, 0x8C6BC1FFu32),
            (
                KAMURI_VIOLET.option.checked.hovered.background,
                0xF0E8FAFFu32,
            ),
            (KAMURI_VIOLET.option.checked.hovered.border, 0xA181D2FFu32),
            (
                KAMURI_VIOLET.option.checked.hovered.foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.option.checked.hovered.dot, 0xA181D2FFu32),
            (
                KAMURI_VIOLET.option.checked.disabled.background,
                0xE8E0F0FFu32,
            ),
            (KAMURI_VIOLET.option.checked.disabled.border, 0xE7DFF0FFu32),
            (
                KAMURI_VIOLET.option.checked.disabled.foreground,
                0xB5A9C1FFu32,
            ),
            (KAMURI_VIOLET.option.checked.disabled.dot, 0xB5A9C1FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.container.normal.background, [255, 255, 255, 255]),
            (LIGHT.container.normal.border, [217, 217, 217, 255]),
            (LIGHT.container.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.container.focused.background, [255, 255, 255, 255]),
            (LIGHT.container.focused.border, [22, 119, 255, 255]),
            (LIGHT.container.focused.foreground, [31, 31, 31, 255]),
            (LIGHT.container.disabled.background, [245, 245, 245, 255]),
            (LIGHT.container.disabled.border, [217, 217, 217, 255]),
            (LIGHT.container.disabled.foreground, [191, 191, 191, 255]),
            (
                LIGHT.option.unchecked.normal.background,
                [255, 255, 255, 255],
            ),
            (LIGHT.option.unchecked.normal.border, [217, 217, 217, 255]),
            (LIGHT.option.unchecked.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.option.unchecked.normal.dot, [0, 0, 0, 0]),
            (
                LIGHT.option.unchecked.hovered.background,
                [255, 255, 255, 255],
            ),
            (LIGHT.option.unchecked.hovered.border, [64, 150, 255, 255]),
            (LIGHT.option.unchecked.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.option.unchecked.hovered.dot, [0, 0, 0, 0]),
            (
                LIGHT.option.unchecked.disabled.background,
                [245, 245, 245, 255],
            ),
            (LIGHT.option.unchecked.disabled.border, [217, 217, 217, 255]),
            (
                LIGHT.option.unchecked.disabled.foreground,
                [191, 191, 191, 255],
            ),
            (LIGHT.option.unchecked.disabled.dot, [0, 0, 0, 0]),
            (LIGHT.option.checked.normal.background, [255, 255, 255, 255]),
            (LIGHT.option.checked.normal.border, [22, 119, 255, 255]),
            (LIGHT.option.checked.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.option.checked.normal.dot, [22, 119, 255, 255]),
            (
                LIGHT.option.checked.hovered.background,
                [255, 255, 255, 255],
            ),
            (LIGHT.option.checked.hovered.border, [64, 150, 255, 255]),
            (LIGHT.option.checked.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.option.checked.hovered.dot, [64, 150, 255, 255]),
            (
                LIGHT.option.checked.disabled.background,
                [245, 245, 245, 255],
            ),
            (LIGHT.option.checked.disabled.border, [217, 217, 217, 255]),
            (
                LIGHT.option.checked.disabled.foreground,
                [191, 191, 191, 255],
            ),
            (LIGHT.option.checked.disabled.dot, [191, 191, 191, 255]),
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
            (DARK.container.normal.background, [20, 20, 20, 255]),
            (DARK.container.normal.border, [66, 66, 66, 255]),
            (DARK.container.normal.foreground, [220, 220, 220, 255]),
            (DARK.container.focused.background, [20, 20, 20, 255]),
            (DARK.container.focused.border, [60, 137, 232, 255]),
            (DARK.container.focused.foreground, [220, 220, 220, 255]),
            (DARK.container.disabled.background, [31, 31, 31, 255]),
            (DARK.container.disabled.border, [66, 66, 66, 255]),
            (DARK.container.disabled.foreground, [89, 89, 89, 255]),
            (DARK.option.unchecked.normal.background, [20, 20, 20, 255]),
            (DARK.option.unchecked.normal.border, [66, 66, 66, 255]),
            (
                DARK.option.unchecked.normal.foreground,
                [220, 220, 220, 255],
            ),
            (DARK.option.unchecked.normal.dot, [0, 0, 0, 0]),
            (DARK.option.unchecked.hovered.background, [20, 20, 20, 255]),
            (DARK.option.unchecked.hovered.border, [60, 137, 232, 255]),
            (
                DARK.option.unchecked.hovered.foreground,
                [220, 220, 220, 255],
            ),
            (DARK.option.unchecked.hovered.dot, [0, 0, 0, 0]),
            (DARK.option.unchecked.disabled.background, [31, 31, 31, 255]),
            (DARK.option.unchecked.disabled.border, [66, 66, 66, 255]),
            (DARK.option.unchecked.disabled.foreground, [89, 89, 89, 255]),
            (DARK.option.unchecked.disabled.dot, [0, 0, 0, 0]),
            (DARK.option.checked.normal.background, [20, 20, 20, 255]),
            (DARK.option.checked.normal.border, [22, 104, 220, 255]),
            (DARK.option.checked.normal.foreground, [220, 220, 220, 255]),
            (DARK.option.checked.normal.dot, [22, 104, 220, 255]),
            (DARK.option.checked.hovered.background, [20, 20, 20, 255]),
            (DARK.option.checked.hovered.border, [60, 137, 232, 255]),
            (DARK.option.checked.hovered.foreground, [220, 220, 220, 255]),
            (DARK.option.checked.hovered.dot, [60, 137, 232, 255]),
            (DARK.option.checked.disabled.background, [31, 31, 31, 255]),
            (DARK.option.checked.disabled.border, [66, 66, 66, 255]),
            (DARK.option.checked.disabled.foreground, [89, 89, 89, 255]),
            (DARK.option.checked.disabled.dot, [89, 89, 89, 255]),
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
