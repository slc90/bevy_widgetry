use crate::common::{dark, kamuri_violet, light, pink_dream};
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

pub const PINK_DREAM: WidgetryTextFieldColors = WidgetryTextFieldColors {
    editable: WidgetryTextFieldInteractionColors {
        normal: WidgetryTextFieldStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
            caret: pink_dream::TEXT,
            selection_background: pink_dream::SELECTION_BACKGROUND,
            unfocused_selection_background: pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: pink_dream::TEXT,
        },
        hovered: WidgetryTextFieldStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::PRIMARY_HOVER,
            foreground: pink_dream::TEXT,
            caret: pink_dream::TEXT,
            selection_background: pink_dream::SELECTION_BACKGROUND,
            unfocused_selection_background: pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: pink_dream::TEXT,
        },
        focused: WidgetryTextFieldStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::FOCUS_BORDER,
            foreground: pink_dream::TEXT,
            caret: pink_dream::TEXT,
            selection_background: pink_dream::SELECTION_BACKGROUND,
            unfocused_selection_background: pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: pink_dream::TEXT,
        },
        disabled: WidgetryTextFieldStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
            caret: pink_dream::DISABLED_TEXT,
            selection_background: pink_dream::SELECTION_BACKGROUND,
            unfocused_selection_background: pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: pink_dream::DISABLED_TEXT,
        },
    },
    read_only: WidgetryTextFieldInteractionColors {
        normal: WidgetryTextFieldStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
            caret: pink_dream::TEXT,
            selection_background: pink_dream::SELECTION_BACKGROUND,
            unfocused_selection_background: pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: pink_dream::TEXT,
        },
        hovered: WidgetryTextFieldStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::PRIMARY_HOVER,
            foreground: pink_dream::TEXT,
            caret: pink_dream::TEXT,
            selection_background: pink_dream::SELECTION_BACKGROUND,
            unfocused_selection_background: pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: pink_dream::TEXT,
        },
        focused: WidgetryTextFieldStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::FOCUS_BORDER,
            foreground: pink_dream::TEXT,
            caret: pink_dream::TEXT,
            selection_background: pink_dream::SELECTION_BACKGROUND,
            unfocused_selection_background: pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: pink_dream::TEXT,
        },
        disabled: WidgetryTextFieldStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
            caret: pink_dream::DISABLED_TEXT,
            selection_background: pink_dream::SELECTION_BACKGROUND,
            unfocused_selection_background: pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: pink_dream::DISABLED_TEXT,
        },
    },
};

pub const KAMURI_VIOLET: WidgetryTextFieldColors = WidgetryTextFieldColors {
    editable: WidgetryTextFieldInteractionColors {
        normal: WidgetryTextFieldStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
            caret: kamuri_violet::TEXT,
            selection_background: kamuri_violet::SELECTION_BACKGROUND,
            unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryTextFieldStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::PRIMARY_HOVER,
            foreground: kamuri_violet::TEXT,
            caret: kamuri_violet::TEXT,
            selection_background: kamuri_violet::SELECTION_BACKGROUND,
            unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: kamuri_violet::TEXT,
        },
        focused: WidgetryTextFieldStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::FOCUS_BORDER,
            foreground: kamuri_violet::TEXT,
            caret: kamuri_violet::TEXT,
            selection_background: kamuri_violet::SELECTION_BACKGROUND,
            unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryTextFieldStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
            caret: kamuri_violet::DISABLED_TEXT,
            selection_background: kamuri_violet::SELECTION_BACKGROUND,
            unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    read_only: WidgetryTextFieldInteractionColors {
        normal: WidgetryTextFieldStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
            caret: kamuri_violet::TEXT,
            selection_background: kamuri_violet::SELECTION_BACKGROUND,
            unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryTextFieldStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::PRIMARY_HOVER,
            foreground: kamuri_violet::TEXT,
            caret: kamuri_violet::TEXT,
            selection_background: kamuri_violet::SELECTION_BACKGROUND,
            unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: kamuri_violet::TEXT,
        },
        focused: WidgetryTextFieldStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::FOCUS_BORDER,
            foreground: kamuri_violet::TEXT,
            caret: kamuri_violet::TEXT,
            selection_background: kamuri_violet::SELECTION_BACKGROUND,
            unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryTextFieldStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
            caret: kamuri_violet::DISABLED_TEXT,
            selection_background: kamuri_violet::SELECTION_BACKGROUND,
            unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
            selection_foreground: kamuri_violet::DISABLED_TEXT,
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
            (PINK_DREAM.editable.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.editable.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.editable.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.editable.normal.caret, 0x422B3CFFu32),
            (
                PINK_DREAM.editable.normal.selection_background,
                0xF0BDD8FFu32,
            ),
            (
                PINK_DREAM.editable.normal.unfocused_selection_background,
                0xEBDEE5FFu32,
            ),
            (
                PINK_DREAM.editable.normal.selection_foreground,
                0x422B3CFFu32,
            ),
            (PINK_DREAM.editable.hovered.background, 0xFCEAF3FFu32),
            (PINK_DREAM.editable.hovered.border, 0xCD6196FFu32),
            (PINK_DREAM.editable.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.editable.hovered.caret, 0x422B3CFFu32),
            (
                PINK_DREAM.editable.hovered.selection_background,
                0xF0BDD8FFu32,
            ),
            (
                PINK_DREAM.editable.hovered.unfocused_selection_background,
                0xEBDEE5FFu32,
            ),
            (
                PINK_DREAM.editable.hovered.selection_foreground,
                0x422B3CFFu32,
            ),
            (PINK_DREAM.editable.focused.background, 0xFCEAF3FFu32),
            (PINK_DREAM.editable.focused.border, 0xB4437DFFu32),
            (PINK_DREAM.editable.focused.foreground, 0x422B3CFFu32),
            (PINK_DREAM.editable.focused.caret, 0x422B3CFFu32),
            (
                PINK_DREAM.editable.focused.selection_background,
                0xF0BDD8FFu32,
            ),
            (
                PINK_DREAM.editable.focused.unfocused_selection_background,
                0xEBDEE5FFu32,
            ),
            (
                PINK_DREAM.editable.focused.selection_foreground,
                0x422B3CFFu32,
            ),
            (PINK_DREAM.editable.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.editable.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.editable.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.editable.disabled.caret, 0xAC98A3FFu32),
            (
                PINK_DREAM.editable.disabled.selection_background,
                0xF0BDD8FFu32,
            ),
            (
                PINK_DREAM.editable.disabled.unfocused_selection_background,
                0xEBDEE5FFu32,
            ),
            (
                PINK_DREAM.editable.disabled.selection_foreground,
                0xAC98A3FFu32,
            ),
            (PINK_DREAM.read_only.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.read_only.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.read_only.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.read_only.normal.caret, 0x422B3CFFu32),
            (
                PINK_DREAM.read_only.normal.selection_background,
                0xF0BDD8FFu32,
            ),
            (
                PINK_DREAM.read_only.normal.unfocused_selection_background,
                0xEBDEE5FFu32,
            ),
            (
                PINK_DREAM.read_only.normal.selection_foreground,
                0x422B3CFFu32,
            ),
            (PINK_DREAM.read_only.hovered.background, 0xFCEAF3FFu32),
            (PINK_DREAM.read_only.hovered.border, 0xCD6196FFu32),
            (PINK_DREAM.read_only.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.read_only.hovered.caret, 0x422B3CFFu32),
            (
                PINK_DREAM.read_only.hovered.selection_background,
                0xF0BDD8FFu32,
            ),
            (
                PINK_DREAM.read_only.hovered.unfocused_selection_background,
                0xEBDEE5FFu32,
            ),
            (
                PINK_DREAM.read_only.hovered.selection_foreground,
                0x422B3CFFu32,
            ),
            (PINK_DREAM.read_only.focused.background, 0xFCEAF3FFu32),
            (PINK_DREAM.read_only.focused.border, 0xB4437DFFu32),
            (PINK_DREAM.read_only.focused.foreground, 0x422B3CFFu32),
            (PINK_DREAM.read_only.focused.caret, 0x422B3CFFu32),
            (
                PINK_DREAM.read_only.focused.selection_background,
                0xF0BDD8FFu32,
            ),
            (
                PINK_DREAM.read_only.focused.unfocused_selection_background,
                0xEBDEE5FFu32,
            ),
            (
                PINK_DREAM.read_only.focused.selection_foreground,
                0x422B3CFFu32,
            ),
            (PINK_DREAM.read_only.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.read_only.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.read_only.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.read_only.disabled.caret, 0xAC98A3FFu32),
            (
                PINK_DREAM.read_only.disabled.selection_background,
                0xF0BDD8FFu32,
            ),
            (
                PINK_DREAM.read_only.disabled.unfocused_selection_background,
                0xEBDEE5FFu32,
            ),
            (
                PINK_DREAM.read_only.disabled.selection_foreground,
                0xAC98A3FFu32,
            ),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.editable.normal.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.editable.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.editable.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.editable.normal.caret, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.editable.normal.selection_background,
                0xD9CAEFFFu32,
            ),
            (
                KAMURI_VIOLET.editable.normal.unfocused_selection_background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.editable.normal.selection_foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.editable.hovered.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.editable.hovered.border, 0xA181D2FFu32),
            (KAMURI_VIOLET.editable.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.editable.hovered.caret, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.editable.hovered.selection_background,
                0xD9CAEFFFu32,
            ),
            (
                KAMURI_VIOLET
                    .editable
                    .hovered
                    .unfocused_selection_background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.editable.hovered.selection_foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.editable.focused.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.editable.focused.border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.editable.focused.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.editable.focused.caret, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.editable.focused.selection_background,
                0xD9CAEFFFu32,
            ),
            (
                KAMURI_VIOLET
                    .editable
                    .focused
                    .unfocused_selection_background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.editable.focused.selection_foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.editable.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.editable.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.editable.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.editable.disabled.caret, 0xB5A9C1FFu32),
            (
                KAMURI_VIOLET.editable.disabled.selection_background,
                0xD9CAEFFFu32,
            ),
            (
                KAMURI_VIOLET
                    .editable
                    .disabled
                    .unfocused_selection_background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.editable.disabled.selection_foreground,
                0xB5A9C1FFu32,
            ),
            (KAMURI_VIOLET.read_only.normal.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.read_only.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.read_only.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.read_only.normal.caret, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.read_only.normal.selection_background,
                0xD9CAEFFFu32,
            ),
            (
                KAMURI_VIOLET
                    .read_only
                    .normal
                    .unfocused_selection_background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.read_only.normal.selection_foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.read_only.hovered.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.read_only.hovered.border, 0xA181D2FFu32),
            (KAMURI_VIOLET.read_only.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.read_only.hovered.caret, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.read_only.hovered.selection_background,
                0xD9CAEFFFu32,
            ),
            (
                KAMURI_VIOLET
                    .read_only
                    .hovered
                    .unfocused_selection_background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.read_only.hovered.selection_foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.read_only.focused.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.read_only.focused.border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.read_only.focused.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.read_only.focused.caret, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.read_only.focused.selection_background,
                0xD9CAEFFFu32,
            ),
            (
                KAMURI_VIOLET
                    .read_only
                    .focused
                    .unfocused_selection_background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.read_only.focused.selection_foreground,
                0x3D314AFFu32,
            ),
            (KAMURI_VIOLET.read_only.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.read_only.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.read_only.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.read_only.disabled.caret, 0xB5A9C1FFu32),
            (
                KAMURI_VIOLET.read_only.disabled.selection_background,
                0xD9CAEFFFu32,
            ),
            (
                KAMURI_VIOLET
                    .read_only
                    .disabled
                    .unfocused_selection_background,
                0xE8E0F0FFu32,
            ),
            (
                KAMURI_VIOLET.read_only.disabled.selection_foreground,
                0xB5A9C1FFu32,
            ),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

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
