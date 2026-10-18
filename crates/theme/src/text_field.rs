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
    pub hint_warning: Color,
    pub editable: WidgetryTextFieldInteractionColors,
    pub read_only: WidgetryTextFieldInteractionColors,
}

pub const LIGHT: WidgetryTextFieldColors = WidgetryTextFieldColors {
    hint_warning: Color::srgb_u8(128, 96, 0),
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
    hint_warning: Color::srgb_u8(250, 210, 80),
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
    hint_warning: Color::srgb_u8(126, 92, 0),
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
    hint_warning: Color::srgb_u8(116, 91, 0),
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

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn warning_text_has_readable_contrast_on_each_theme_background() {
        fn luminance(color: Color) -> f32 {
            let linear = color.to_linear();
            0.2126 * linear.red + 0.7152 * linear.green + 0.0722 * linear.blue
        }
        for (theme, background) in [
            (LIGHT, light::WINDOW_BACKGROUND),
            (DARK, dark::WINDOW_BACKGROUND),
            (PINK_DREAM, pink_dream::WINDOW_BACKGROUND),
            (KAMURI_VIOLET, kamuri_violet::WINDOW_BACKGROUND),
        ] {
            for surface in [
                background,
                theme.editable.normal.background,
                theme.editable.disabled.background,
            ] {
                let a = luminance(theme.hint_warning);
                let b = luminance(surface);
                let contrast = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                assert!(
                    contrast >= 4.5,
                    "warning contrast {contrast} on {surface:?}"
                );
            }
        }
    }

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.editable.normal.background,
            LIGHT.editable.normal.border,
            LIGHT.editable.normal.foreground,
            LIGHT.editable.normal.caret,
            LIGHT.editable.normal.selection_background,
            LIGHT.editable.normal.unfocused_selection_background,
            LIGHT.editable.normal.selection_foreground,
            LIGHT.editable.hovered.background,
            LIGHT.editable.hovered.border,
            LIGHT.editable.hovered.foreground,
            LIGHT.editable.hovered.caret,
            LIGHT.editable.hovered.selection_background,
            LIGHT.editable.hovered.unfocused_selection_background,
            LIGHT.editable.hovered.selection_foreground,
            LIGHT.editable.focused.background,
            LIGHT.editable.focused.border,
            LIGHT.editable.focused.foreground,
            LIGHT.editable.focused.caret,
            LIGHT.editable.focused.selection_background,
            LIGHT.editable.focused.unfocused_selection_background,
            LIGHT.editable.focused.selection_foreground,
            LIGHT.editable.disabled.background,
            LIGHT.editable.disabled.border,
            LIGHT.editable.disabled.foreground,
            LIGHT.editable.disabled.caret,
            LIGHT.editable.disabled.selection_background,
            LIGHT.editable.disabled.unfocused_selection_background,
            LIGHT.editable.disabled.selection_foreground,
            LIGHT.read_only.normal.background,
            LIGHT.read_only.normal.border,
            LIGHT.read_only.normal.foreground,
            LIGHT.read_only.normal.caret,
            LIGHT.read_only.normal.selection_background,
            LIGHT.read_only.normal.unfocused_selection_background,
            LIGHT.read_only.normal.selection_foreground,
            LIGHT.read_only.hovered.background,
            LIGHT.read_only.hovered.border,
            LIGHT.read_only.hovered.foreground,
            LIGHT.read_only.hovered.caret,
            LIGHT.read_only.hovered.selection_background,
            LIGHT.read_only.hovered.unfocused_selection_background,
            LIGHT.read_only.hovered.selection_foreground,
            LIGHT.read_only.focused.background,
            LIGHT.read_only.focused.border,
            LIGHT.read_only.focused.foreground,
            LIGHT.read_only.focused.caret,
            LIGHT.read_only.focused.selection_background,
            LIGHT.read_only.focused.unfocused_selection_background,
            LIGHT.read_only.focused.selection_foreground,
            LIGHT.read_only.disabled.background,
            LIGHT.read_only.disabled.border,
            LIGHT.read_only.disabled.foreground,
            LIGHT.read_only.disabled.caret,
            LIGHT.read_only.disabled.selection_background,
            LIGHT.read_only.disabled.unfocused_selection_background,
            LIGHT.read_only.disabled.selection_foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
    #[test]
    fn dark_colors_are_finite() {
        let colors = [
            DARK.editable.normal.background,
            DARK.editable.normal.border,
            DARK.editable.normal.foreground,
            DARK.editable.normal.caret,
            DARK.editable.normal.selection_background,
            DARK.editable.normal.unfocused_selection_background,
            DARK.editable.normal.selection_foreground,
            DARK.editable.hovered.background,
            DARK.editable.hovered.border,
            DARK.editable.hovered.foreground,
            DARK.editable.hovered.caret,
            DARK.editable.hovered.selection_background,
            DARK.editable.hovered.unfocused_selection_background,
            DARK.editable.hovered.selection_foreground,
            DARK.editable.focused.background,
            DARK.editable.focused.border,
            DARK.editable.focused.foreground,
            DARK.editable.focused.caret,
            DARK.editable.focused.selection_background,
            DARK.editable.focused.unfocused_selection_background,
            DARK.editable.focused.selection_foreground,
            DARK.editable.disabled.background,
            DARK.editable.disabled.border,
            DARK.editable.disabled.foreground,
            DARK.editable.disabled.caret,
            DARK.editable.disabled.selection_background,
            DARK.editable.disabled.unfocused_selection_background,
            DARK.editable.disabled.selection_foreground,
            DARK.read_only.normal.background,
            DARK.read_only.normal.border,
            DARK.read_only.normal.foreground,
            DARK.read_only.normal.caret,
            DARK.read_only.normal.selection_background,
            DARK.read_only.normal.unfocused_selection_background,
            DARK.read_only.normal.selection_foreground,
            DARK.read_only.hovered.background,
            DARK.read_only.hovered.border,
            DARK.read_only.hovered.foreground,
            DARK.read_only.hovered.caret,
            DARK.read_only.hovered.selection_background,
            DARK.read_only.hovered.unfocused_selection_background,
            DARK.read_only.hovered.selection_foreground,
            DARK.read_only.focused.background,
            DARK.read_only.focused.border,
            DARK.read_only.focused.foreground,
            DARK.read_only.focused.caret,
            DARK.read_only.focused.selection_background,
            DARK.read_only.focused.unfocused_selection_background,
            DARK.read_only.focused.selection_foreground,
            DARK.read_only.disabled.background,
            DARK.read_only.disabled.border,
            DARK.read_only.disabled.foreground,
            DARK.read_only.disabled.caret,
            DARK.read_only.disabled.selection_background,
            DARK.read_only.disabled.unfocused_selection_background,
            DARK.read_only.disabled.selection_foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
}
