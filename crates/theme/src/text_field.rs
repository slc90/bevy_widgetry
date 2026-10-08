use crate::common::{dark, light};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTextFieldStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
    pub caret: Color,
    pub selection_background: Color,
    pub unfocused_selection_background: Color,
    pub selection_foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTextFieldInteractionColors {
    pub normal: WidgetryTextFieldStateColors,
    pub hovered: WidgetryTextFieldStateColors,
    pub focused: WidgetryTextFieldStateColors,
    pub disabled: WidgetryTextFieldStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTextFieldColors {
    pub editable: WidgetryTextFieldInteractionColors,
    pub read_only: WidgetryTextFieldInteractionColors,
}

pub const LIGHT: WidgetryTextFieldColors = WidgetryTextFieldColors {
    editable: WidgetryTextFieldInteractionColors {
        normal: WidgetryTextFieldStateColors {
            background: light::SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
            caret: light::TEXT,
            selection_background: light::SELECTION_BACKGROUND,
            unfocused_selection_background: light::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: light::TEXT,
        },
        hovered: WidgetryTextFieldStateColors {
            background: light::SURFACE,
            border: light::PRIMARY_HOVER,
            foreground: light::TEXT,
            caret: light::TEXT,
            selection_background: light::SELECTION_BACKGROUND,
            unfocused_selection_background: light::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: light::TEXT,
        },
        focused: WidgetryTextFieldStateColors {
            background: light::SURFACE,
            border: light::FOCUS_BORDER,
            foreground: light::TEXT,
            caret: light::TEXT,
            selection_background: light::SELECTION_BACKGROUND,
            unfocused_selection_background: light::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: light::TEXT,
        },
        disabled: WidgetryTextFieldStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
            caret: light::DISABLED_TEXT,
            selection_background: light::SELECTION_BACKGROUND,
            unfocused_selection_background: light::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: light::DISABLED_TEXT,
        },
    },
    read_only: WidgetryTextFieldInteractionColors {
        normal: WidgetryTextFieldStateColors {
            background: light::SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
            caret: light::TEXT,
            selection_background: light::SELECTION_BACKGROUND,
            unfocused_selection_background: light::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: light::TEXT,
        },
        hovered: WidgetryTextFieldStateColors {
            background: light::SURFACE,
            border: light::PRIMARY_HOVER,
            foreground: light::TEXT,
            caret: light::TEXT,
            selection_background: light::SELECTION_BACKGROUND,
            unfocused_selection_background: light::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: light::TEXT,
        },
        focused: WidgetryTextFieldStateColors {
            background: light::SURFACE,
            border: light::FOCUS_BORDER,
            foreground: light::TEXT,
            caret: light::TEXT,
            selection_background: light::SELECTION_BACKGROUND,
            unfocused_selection_background: light::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: light::TEXT,
        },
        disabled: WidgetryTextFieldStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
            caret: light::DISABLED_TEXT,
            selection_background: light::SELECTION_BACKGROUND,
            unfocused_selection_background: light::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: light::DISABLED_TEXT,
        },
    },
};

pub const DARK: WidgetryTextFieldColors = WidgetryTextFieldColors {
    editable: WidgetryTextFieldInteractionColors {
        normal: WidgetryTextFieldStateColors {
            background: dark::SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
            caret: dark::TEXT,
            selection_background: dark::SELECTION_BACKGROUND,
            unfocused_selection_background: dark::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: dark::TEXT,
        },
        hovered: WidgetryTextFieldStateColors {
            background: dark::SURFACE,
            border: dark::PRIMARY_HOVER,
            foreground: dark::TEXT,
            caret: dark::TEXT,
            selection_background: dark::SELECTION_BACKGROUND,
            unfocused_selection_background: dark::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: dark::TEXT,
        },
        focused: WidgetryTextFieldStateColors {
            background: dark::SURFACE,
            border: dark::FOCUS_BORDER,
            foreground: dark::TEXT,
            caret: dark::TEXT,
            selection_background: dark::SELECTION_BACKGROUND,
            unfocused_selection_background: dark::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: dark::TEXT,
        },
        disabled: WidgetryTextFieldStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
            caret: dark::DISABLED_TEXT,
            selection_background: dark::SELECTION_BACKGROUND,
            unfocused_selection_background: dark::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: dark::DISABLED_TEXT,
        },
    },
    read_only: WidgetryTextFieldInteractionColors {
        normal: WidgetryTextFieldStateColors {
            background: dark::SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
            caret: dark::TEXT,
            selection_background: dark::SELECTION_BACKGROUND,
            unfocused_selection_background: dark::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: dark::TEXT,
        },
        hovered: WidgetryTextFieldStateColors {
            background: dark::SURFACE,
            border: dark::PRIMARY_HOVER,
            foreground: dark::TEXT,
            caret: dark::TEXT,
            selection_background: dark::SELECTION_BACKGROUND,
            unfocused_selection_background: dark::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: dark::TEXT,
        },
        focused: WidgetryTextFieldStateColors {
            background: dark::SURFACE,
            border: dark::FOCUS_BORDER,
            foreground: dark::TEXT,
            caret: dark::TEXT,
            selection_background: dark::SELECTION_BACKGROUND,
            unfocused_selection_background: dark::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: dark::TEXT,
        },
        disabled: WidgetryTextFieldStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
            caret: dark::DISABLED_TEXT,
            selection_background: dark::SELECTION_BACKGROUND,
            unfocused_selection_background: dark::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: dark::DISABLED_TEXT,
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
            (LIGHT.editable.normal.background, [255, 255, 255, 255]),
            (LIGHT.editable.normal.border, [217, 217, 217, 255]),
            (LIGHT.editable.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.editable.normal.caret, [31, 31, 31, 255]),
            (
                LIGHT.editable.normal.selection_background,
                [186, 224, 255, 255],
            ),
            (
                LIGHT.editable.normal.unfocused_selection_background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.editable.normal.selection_foreground,
                [31, 31, 31, 255],
            ),
            (LIGHT.editable.hovered.background, [255, 255, 255, 255]),
            (LIGHT.editable.hovered.border, [64, 150, 255, 255]),
            (LIGHT.editable.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.editable.hovered.caret, [31, 31, 31, 255]),
            (
                LIGHT.editable.hovered.selection_background,
                [186, 224, 255, 255],
            ),
            (
                LIGHT.editable.hovered.unfocused_selection_background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.editable.hovered.selection_foreground,
                [31, 31, 31, 255],
            ),
            (LIGHT.editable.focused.background, [255, 255, 255, 255]),
            (LIGHT.editable.focused.border, [22, 119, 255, 255]),
            (LIGHT.editable.focused.foreground, [31, 31, 31, 255]),
            (LIGHT.editable.focused.caret, [31, 31, 31, 255]),
            (
                LIGHT.editable.focused.selection_background,
                [186, 224, 255, 255],
            ),
            (
                LIGHT.editable.focused.unfocused_selection_background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.editable.focused.selection_foreground,
                [31, 31, 31, 255],
            ),
            (LIGHT.editable.disabled.background, [245, 245, 245, 255]),
            (LIGHT.editable.disabled.border, [217, 217, 217, 255]),
            (LIGHT.editable.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.editable.disabled.caret, [191, 191, 191, 255]),
            (
                LIGHT.editable.disabled.selection_background,
                [186, 224, 255, 255],
            ),
            (
                LIGHT.editable.disabled.unfocused_selection_background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.editable.disabled.selection_foreground,
                [191, 191, 191, 255],
            ),
            (LIGHT.read_only.normal.background, [255, 255, 255, 255]),
            (LIGHT.read_only.normal.border, [217, 217, 217, 255]),
            (LIGHT.read_only.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.read_only.normal.caret, [31, 31, 31, 255]),
            (
                LIGHT.read_only.normal.selection_background,
                [186, 224, 255, 255],
            ),
            (
                LIGHT.read_only.normal.unfocused_selection_background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.read_only.normal.selection_foreground,
                [31, 31, 31, 255],
            ),
            (LIGHT.read_only.hovered.background, [255, 255, 255, 255]),
            (LIGHT.read_only.hovered.border, [64, 150, 255, 255]),
            (LIGHT.read_only.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.read_only.hovered.caret, [31, 31, 31, 255]),
            (
                LIGHT.read_only.hovered.selection_background,
                [186, 224, 255, 255],
            ),
            (
                LIGHT.read_only.hovered.unfocused_selection_background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.read_only.hovered.selection_foreground,
                [31, 31, 31, 255],
            ),
            (LIGHT.read_only.focused.background, [255, 255, 255, 255]),
            (LIGHT.read_only.focused.border, [22, 119, 255, 255]),
            (LIGHT.read_only.focused.foreground, [31, 31, 31, 255]),
            (LIGHT.read_only.focused.caret, [31, 31, 31, 255]),
            (
                LIGHT.read_only.focused.selection_background,
                [186, 224, 255, 255],
            ),
            (
                LIGHT.read_only.focused.unfocused_selection_background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.read_only.focused.selection_foreground,
                [31, 31, 31, 255],
            ),
            (LIGHT.read_only.disabled.background, [245, 245, 245, 255]),
            (LIGHT.read_only.disabled.border, [217, 217, 217, 255]),
            (LIGHT.read_only.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.read_only.disabled.caret, [191, 191, 191, 255]),
            (
                LIGHT.read_only.disabled.selection_background,
                [186, 224, 255, 255],
            ),
            (
                LIGHT.read_only.disabled.unfocused_selection_background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.read_only.disabled.selection_foreground,
                [191, 191, 191, 255],
            ),
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
            (DARK.editable.normal.background, [20, 20, 20, 255]),
            (DARK.editable.normal.border, [66, 66, 66, 255]),
            (DARK.editable.normal.foreground, [220, 220, 220, 255]),
            (DARK.editable.normal.caret, [220, 220, 220, 255]),
            (DARK.editable.normal.selection_background, [21, 57, 91, 255]),
            (
                DARK.editable.normal.unfocused_selection_background,
                [48, 48, 48, 255],
            ),
            (
                DARK.editable.normal.selection_foreground,
                [220, 220, 220, 255],
            ),
            (DARK.editable.hovered.background, [20, 20, 20, 255]),
            (DARK.editable.hovered.border, [60, 137, 232, 255]),
            (DARK.editable.hovered.foreground, [220, 220, 220, 255]),
            (DARK.editable.hovered.caret, [220, 220, 220, 255]),
            (
                DARK.editable.hovered.selection_background,
                [21, 57, 91, 255],
            ),
            (
                DARK.editable.hovered.unfocused_selection_background,
                [48, 48, 48, 255],
            ),
            (
                DARK.editable.hovered.selection_foreground,
                [220, 220, 220, 255],
            ),
            (DARK.editable.focused.background, [20, 20, 20, 255]),
            (DARK.editable.focused.border, [60, 137, 232, 255]),
            (DARK.editable.focused.foreground, [220, 220, 220, 255]),
            (DARK.editable.focused.caret, [220, 220, 220, 255]),
            (
                DARK.editable.focused.selection_background,
                [21, 57, 91, 255],
            ),
            (
                DARK.editable.focused.unfocused_selection_background,
                [48, 48, 48, 255],
            ),
            (
                DARK.editable.focused.selection_foreground,
                [220, 220, 220, 255],
            ),
            (DARK.editable.disabled.background, [31, 31, 31, 255]),
            (DARK.editable.disabled.border, [66, 66, 66, 255]),
            (DARK.editable.disabled.foreground, [89, 89, 89, 255]),
            (DARK.editable.disabled.caret, [89, 89, 89, 255]),
            (
                DARK.editable.disabled.selection_background,
                [21, 57, 91, 255],
            ),
            (
                DARK.editable.disabled.unfocused_selection_background,
                [48, 48, 48, 255],
            ),
            (
                DARK.editable.disabled.selection_foreground,
                [89, 89, 89, 255],
            ),
            (DARK.read_only.normal.background, [20, 20, 20, 255]),
            (DARK.read_only.normal.border, [66, 66, 66, 255]),
            (DARK.read_only.normal.foreground, [220, 220, 220, 255]),
            (DARK.read_only.normal.caret, [220, 220, 220, 255]),
            (
                DARK.read_only.normal.selection_background,
                [21, 57, 91, 255],
            ),
            (
                DARK.read_only.normal.unfocused_selection_background,
                [48, 48, 48, 255],
            ),
            (
                DARK.read_only.normal.selection_foreground,
                [220, 220, 220, 255],
            ),
            (DARK.read_only.hovered.background, [20, 20, 20, 255]),
            (DARK.read_only.hovered.border, [60, 137, 232, 255]),
            (DARK.read_only.hovered.foreground, [220, 220, 220, 255]),
            (DARK.read_only.hovered.caret, [220, 220, 220, 255]),
            (
                DARK.read_only.hovered.selection_background,
                [21, 57, 91, 255],
            ),
            (
                DARK.read_only.hovered.unfocused_selection_background,
                [48, 48, 48, 255],
            ),
            (
                DARK.read_only.hovered.selection_foreground,
                [220, 220, 220, 255],
            ),
            (DARK.read_only.focused.background, [20, 20, 20, 255]),
            (DARK.read_only.focused.border, [60, 137, 232, 255]),
            (DARK.read_only.focused.foreground, [220, 220, 220, 255]),
            (DARK.read_only.focused.caret, [220, 220, 220, 255]),
            (
                DARK.read_only.focused.selection_background,
                [21, 57, 91, 255],
            ),
            (
                DARK.read_only.focused.unfocused_selection_background,
                [48, 48, 48, 255],
            ),
            (
                DARK.read_only.focused.selection_foreground,
                [220, 220, 220, 255],
            ),
            (DARK.read_only.disabled.background, [31, 31, 31, 255]),
            (DARK.read_only.disabled.border, [66, 66, 66, 255]),
            (DARK.read_only.disabled.foreground, [89, 89, 89, 255]),
            (DARK.read_only.disabled.caret, [89, 89, 89, 255]),
            (
                DARK.read_only.disabled.selection_background,
                [21, 57, 91, 255],
            ),
            (
                DARK.read_only.disabled.unfocused_selection_background,
                [48, 48, 48, 255],
            ),
            (
                DARK.read_only.disabled.selection_foreground,
                [89, 89, 89, 255],
            ),
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
