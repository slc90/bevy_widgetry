use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryWindowStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryWindowSurfaceColors {
    pub normal: WidgetryWindowStateColors,
    pub disabled: WidgetryWindowStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryWindowButtonStateColors {
    pub background: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryWindowButtonColors {
    pub normal: WidgetryWindowButtonStateColors,
    pub hovered: WidgetryWindowButtonStateColors,
    pub pressed: WidgetryWindowButtonStateColors,
    pub disabled: WidgetryWindowButtonStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryWindowColors {
    pub frame: WidgetryWindowSurfaceColors,
    pub title_bar: WidgetryWindowSurfaceColors,
    pub minimize: WidgetryWindowButtonColors,
    pub maximize: WidgetryWindowButtonColors,
    pub close: WidgetryWindowButtonColors,
    pub image_tint: Color,
}

pub const LIGHT: WidgetryWindowColors = WidgetryWindowColors {
    frame: WidgetryWindowSurfaceColors {
        normal: WidgetryWindowStateColors {
            background: light::WINDOW_BACKGROUND,
            border: light::BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryWindowStateColors {
            background: light::WINDOW_BACKGROUND,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
    },
    title_bar: WidgetryWindowSurfaceColors {
        normal: WidgetryWindowStateColors {
            background: light::TRANSPARENT,
            border: light::BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryWindowStateColors {
            background: light::TRANSPARENT,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
    },
    minimize: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: light::HOVER_SURFACE,
            foreground: light::TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: light::PRESSED_SURFACE,
            foreground: light::TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: light::TRANSPARENT,
            foreground: light::DISABLED_TEXT,
        },
    },
    maximize: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: light::HOVER_SURFACE,
            foreground: light::TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: light::PRESSED_SURFACE,
            foreground: light::TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: light::TRANSPARENT,
            foreground: light::DISABLED_TEXT,
        },
    },
    close: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: light::DANGER_HOVER,
            foreground: light::INVERSE_TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: light::DANGER_PRESSED,
            foreground: light::INVERSE_TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: light::TRANSPARENT,
            foreground: light::DISABLED_TEXT,
        },
    },
    image_tint: light::IMAGE_TINT,
};

pub const DARK: WidgetryWindowColors = WidgetryWindowColors {
    frame: WidgetryWindowSurfaceColors {
        normal: WidgetryWindowStateColors {
            background: dark::WINDOW_BACKGROUND,
            border: dark::BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryWindowStateColors {
            background: dark::WINDOW_BACKGROUND,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
    },
    title_bar: WidgetryWindowSurfaceColors {
        normal: WidgetryWindowStateColors {
            background: dark::TRANSPARENT,
            border: dark::BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryWindowStateColors {
            background: dark::TRANSPARENT,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
    },
    minimize: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: dark::HOVER_SURFACE,
            foreground: dark::TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: dark::PRESSED_SURFACE,
            foreground: dark::TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: dark::TRANSPARENT,
            foreground: dark::DISABLED_TEXT,
        },
    },
    maximize: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: dark::HOVER_SURFACE,
            foreground: dark::TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: dark::PRESSED_SURFACE,
            foreground: dark::TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: dark::TRANSPARENT,
            foreground: dark::DISABLED_TEXT,
        },
    },
    close: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: dark::DANGER_HOVER,
            foreground: dark::INVERSE_TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: dark::DANGER_PRESSED,
            foreground: dark::INVERSE_TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: dark::TRANSPARENT,
            foreground: dark::DISABLED_TEXT,
        },
    },
    image_tint: dark::IMAGE_TINT,
};

pub const PINK_DREAM: WidgetryWindowColors = WidgetryWindowColors {
    frame: WidgetryWindowSurfaceColors {
        normal: WidgetryWindowStateColors {
            background: pink_dream::WINDOW_BACKGROUND,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryWindowStateColors {
            background: pink_dream::WINDOW_BACKGROUND,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    title_bar: WidgetryWindowSurfaceColors {
        normal: WidgetryWindowStateColors {
            background: pink_dream::TRANSPARENT,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryWindowStateColors {
            background: pink_dream::TRANSPARENT,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    minimize: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: pink_dream::HOVER_SURFACE,
            foreground: pink_dream::TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: pink_dream::PRESSED_SURFACE,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: pink_dream::TRANSPARENT,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    maximize: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: pink_dream::HOVER_SURFACE,
            foreground: pink_dream::TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: pink_dream::PRESSED_SURFACE,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: pink_dream::TRANSPARENT,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    close: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: pink_dream::DANGER_HOVER,
            foreground: pink_dream::INVERSE_TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: pink_dream::DANGER_PRESSED,
            foreground: pink_dream::INVERSE_TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: pink_dream::TRANSPARENT,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    image_tint: pink_dream::IMAGE_TINT,
};

pub const KAMURI_VIOLET: WidgetryWindowColors = WidgetryWindowColors {
    frame: WidgetryWindowSurfaceColors {
        normal: WidgetryWindowStateColors {
            background: kamuri_violet::WINDOW_BACKGROUND,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryWindowStateColors {
            background: kamuri_violet::WINDOW_BACKGROUND,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    title_bar: WidgetryWindowSurfaceColors {
        normal: WidgetryWindowStateColors {
            background: kamuri_violet::TRANSPARENT,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryWindowStateColors {
            background: kamuri_violet::TRANSPARENT,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    minimize: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            foreground: kamuri_violet::TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: kamuri_violet::PRESSED_SURFACE,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    maximize: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            foreground: kamuri_violet::TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: kamuri_violet::PRESSED_SURFACE,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    close: WidgetryWindowButtonColors {
        normal: WidgetryWindowButtonStateColors {
            background: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryWindowButtonStateColors {
            background: kamuri_violet::DANGER_HOVER,
            foreground: kamuri_violet::INVERSE_TEXT,
        },
        pressed: WidgetryWindowButtonStateColors {
            background: kamuri_violet::DANGER_PRESSED,
            foreground: kamuri_violet::INVERSE_TEXT,
        },
        disabled: WidgetryWindowButtonStateColors {
            background: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    image_tint: kamuri_violet::IMAGE_TINT,
};

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.frame.normal.background,
            LIGHT.frame.normal.border,
            LIGHT.frame.normal.foreground,
            LIGHT.frame.disabled.background,
            LIGHT.frame.disabled.border,
            LIGHT.frame.disabled.foreground,
            LIGHT.title_bar.normal.background,
            LIGHT.title_bar.normal.border,
            LIGHT.title_bar.normal.foreground,
            LIGHT.title_bar.disabled.background,
            LIGHT.title_bar.disabled.border,
            LIGHT.title_bar.disabled.foreground,
            LIGHT.minimize.normal.background,
            LIGHT.minimize.normal.foreground,
            LIGHT.minimize.hovered.background,
            LIGHT.minimize.hovered.foreground,
            LIGHT.minimize.pressed.background,
            LIGHT.minimize.pressed.foreground,
            LIGHT.minimize.disabled.background,
            LIGHT.minimize.disabled.foreground,
            LIGHT.maximize.normal.background,
            LIGHT.maximize.normal.foreground,
            LIGHT.maximize.hovered.background,
            LIGHT.maximize.hovered.foreground,
            LIGHT.maximize.pressed.background,
            LIGHT.maximize.pressed.foreground,
            LIGHT.maximize.disabled.background,
            LIGHT.maximize.disabled.foreground,
            LIGHT.close.normal.background,
            LIGHT.close.normal.foreground,
            LIGHT.close.hovered.background,
            LIGHT.close.hovered.foreground,
            LIGHT.close.pressed.background,
            LIGHT.close.pressed.foreground,
            LIGHT.close.disabled.background,
            LIGHT.close.disabled.foreground,
            LIGHT.image_tint,
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
            DARK.frame.normal.background,
            DARK.frame.normal.border,
            DARK.frame.normal.foreground,
            DARK.frame.disabled.background,
            DARK.frame.disabled.border,
            DARK.frame.disabled.foreground,
            DARK.title_bar.normal.background,
            DARK.title_bar.normal.border,
            DARK.title_bar.normal.foreground,
            DARK.title_bar.disabled.background,
            DARK.title_bar.disabled.border,
            DARK.title_bar.disabled.foreground,
            DARK.minimize.normal.background,
            DARK.minimize.normal.foreground,
            DARK.minimize.hovered.background,
            DARK.minimize.hovered.foreground,
            DARK.minimize.pressed.background,
            DARK.minimize.pressed.foreground,
            DARK.minimize.disabled.background,
            DARK.minimize.disabled.foreground,
            DARK.maximize.normal.background,
            DARK.maximize.normal.foreground,
            DARK.maximize.hovered.background,
            DARK.maximize.hovered.foreground,
            DARK.maximize.pressed.background,
            DARK.maximize.pressed.foreground,
            DARK.maximize.disabled.background,
            DARK.maximize.disabled.foreground,
            DARK.close.normal.background,
            DARK.close.normal.foreground,
            DARK.close.hovered.background,
            DARK.close.hovered.foreground,
            DARK.close.pressed.background,
            DARK.close.pressed.foreground,
            DARK.close.disabled.background,
            DARK.close.disabled.foreground,
            DARK.image_tint,
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
