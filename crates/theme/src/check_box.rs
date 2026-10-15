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

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.unchecked.normal.background,
            LIGHT.unchecked.normal.border,
            LIGHT.unchecked.normal.foreground,
            LIGHT.unchecked.normal.mark,
            LIGHT.unchecked.hovered.background,
            LIGHT.unchecked.hovered.border,
            LIGHT.unchecked.hovered.foreground,
            LIGHT.unchecked.hovered.mark,
            LIGHT.unchecked.pressed.background,
            LIGHT.unchecked.pressed.border,
            LIGHT.unchecked.pressed.foreground,
            LIGHT.unchecked.pressed.mark,
            LIGHT.unchecked.disabled.background,
            LIGHT.unchecked.disabled.border,
            LIGHT.unchecked.disabled.foreground,
            LIGHT.unchecked.disabled.mark,
            LIGHT.checked.normal.background,
            LIGHT.checked.normal.border,
            LIGHT.checked.normal.foreground,
            LIGHT.checked.normal.mark,
            LIGHT.checked.hovered.background,
            LIGHT.checked.hovered.border,
            LIGHT.checked.hovered.foreground,
            LIGHT.checked.hovered.mark,
            LIGHT.checked.pressed.background,
            LIGHT.checked.pressed.border,
            LIGHT.checked.pressed.foreground,
            LIGHT.checked.pressed.mark,
            LIGHT.checked.disabled.background,
            LIGHT.checked.disabled.border,
            LIGHT.checked.disabled.foreground,
            LIGHT.checked.disabled.mark,
            LIGHT.indeterminate.normal.background,
            LIGHT.indeterminate.normal.border,
            LIGHT.indeterminate.normal.foreground,
            LIGHT.indeterminate.normal.mark,
            LIGHT.indeterminate.hovered.background,
            LIGHT.indeterminate.hovered.border,
            LIGHT.indeterminate.hovered.foreground,
            LIGHT.indeterminate.hovered.mark,
            LIGHT.indeterminate.pressed.background,
            LIGHT.indeterminate.pressed.border,
            LIGHT.indeterminate.pressed.foreground,
            LIGHT.indeterminate.pressed.mark,
            LIGHT.indeterminate.disabled.background,
            LIGHT.indeterminate.disabled.border,
            LIGHT.indeterminate.disabled.foreground,
            LIGHT.indeterminate.disabled.mark,
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
            DARK.unchecked.normal.background,
            DARK.unchecked.normal.border,
            DARK.unchecked.normal.foreground,
            DARK.unchecked.normal.mark,
            DARK.unchecked.hovered.background,
            DARK.unchecked.hovered.border,
            DARK.unchecked.hovered.foreground,
            DARK.unchecked.hovered.mark,
            DARK.unchecked.pressed.background,
            DARK.unchecked.pressed.border,
            DARK.unchecked.pressed.foreground,
            DARK.unchecked.pressed.mark,
            DARK.unchecked.disabled.background,
            DARK.unchecked.disabled.border,
            DARK.unchecked.disabled.foreground,
            DARK.unchecked.disabled.mark,
            DARK.checked.normal.background,
            DARK.checked.normal.border,
            DARK.checked.normal.foreground,
            DARK.checked.normal.mark,
            DARK.checked.hovered.background,
            DARK.checked.hovered.border,
            DARK.checked.hovered.foreground,
            DARK.checked.hovered.mark,
            DARK.checked.pressed.background,
            DARK.checked.pressed.border,
            DARK.checked.pressed.foreground,
            DARK.checked.pressed.mark,
            DARK.checked.disabled.background,
            DARK.checked.disabled.border,
            DARK.checked.disabled.foreground,
            DARK.checked.disabled.mark,
            DARK.indeterminate.normal.background,
            DARK.indeterminate.normal.border,
            DARK.indeterminate.normal.foreground,
            DARK.indeterminate.normal.mark,
            DARK.indeterminate.hovered.background,
            DARK.indeterminate.hovered.border,
            DARK.indeterminate.hovered.foreground,
            DARK.indeterminate.hovered.mark,
            DARK.indeterminate.pressed.background,
            DARK.indeterminate.pressed.border,
            DARK.indeterminate.pressed.foreground,
            DARK.indeterminate.pressed.mark,
            DARK.indeterminate.disabled.background,
            DARK.indeterminate.disabled.border,
            DARK.indeterminate.disabled.foreground,
            DARK.indeterminate.disabled.mark,
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
