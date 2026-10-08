use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
