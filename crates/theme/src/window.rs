use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.frame.normal.background, [255, 255, 255, 255]),
            (LIGHT.frame.normal.border, [217, 217, 217, 255]),
            (LIGHT.frame.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.frame.disabled.background, [255, 255, 255, 255]),
            (LIGHT.frame.disabled.border, [217, 217, 217, 255]),
            (LIGHT.frame.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.title_bar.normal.background, [0, 0, 0, 0]),
            (LIGHT.title_bar.normal.border, [217, 217, 217, 255]),
            (LIGHT.title_bar.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.title_bar.disabled.background, [0, 0, 0, 0]),
            (LIGHT.title_bar.disabled.border, [217, 217, 217, 255]),
            (LIGHT.title_bar.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.minimize.normal.background, [0, 0, 0, 0]),
            (LIGHT.minimize.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.minimize.hovered.background, [245, 245, 245, 255]),
            (LIGHT.minimize.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.minimize.pressed.background, [235, 235, 235, 255]),
            (LIGHT.minimize.pressed.foreground, [31, 31, 31, 255]),
            (LIGHT.minimize.disabled.background, [0, 0, 0, 0]),
            (LIGHT.minimize.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.maximize.normal.background, [0, 0, 0, 0]),
            (LIGHT.maximize.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.maximize.hovered.background, [245, 245, 245, 255]),
            (LIGHT.maximize.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.maximize.pressed.background, [235, 235, 235, 255]),
            (LIGHT.maximize.pressed.foreground, [31, 31, 31, 255]),
            (LIGHT.maximize.disabled.background, [0, 0, 0, 0]),
            (LIGHT.maximize.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.close.normal.background, [0, 0, 0, 0]),
            (LIGHT.close.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.close.hovered.background, [217, 54, 62, 255]),
            (LIGHT.close.hovered.foreground, [255, 255, 255, 255]),
            (LIGHT.close.pressed.background, [168, 7, 26, 255]),
            (LIGHT.close.pressed.foreground, [255, 255, 255, 255]),
            (LIGHT.close.disabled.background, [0, 0, 0, 0]),
            (LIGHT.close.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.image_tint, [255, 255, 255, 255]),
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
            (DARK.frame.normal.background, [20, 20, 20, 255]),
            (DARK.frame.normal.border, [66, 66, 66, 255]),
            (DARK.frame.normal.foreground, [220, 220, 220, 255]),
            (DARK.frame.disabled.background, [20, 20, 20, 255]),
            (DARK.frame.disabled.border, [66, 66, 66, 255]),
            (DARK.frame.disabled.foreground, [89, 89, 89, 255]),
            (DARK.title_bar.normal.background, [0, 0, 0, 0]),
            (DARK.title_bar.normal.border, [66, 66, 66, 255]),
            (DARK.title_bar.normal.foreground, [220, 220, 220, 255]),
            (DARK.title_bar.disabled.background, [0, 0, 0, 0]),
            (DARK.title_bar.disabled.border, [66, 66, 66, 255]),
            (DARK.title_bar.disabled.foreground, [89, 89, 89, 255]),
            (DARK.minimize.normal.background, [0, 0, 0, 0]),
            (DARK.minimize.normal.foreground, [220, 220, 220, 255]),
            (DARK.minimize.hovered.background, [38, 38, 38, 255]),
            (DARK.minimize.hovered.foreground, [220, 220, 220, 255]),
            (DARK.minimize.pressed.background, [48, 48, 48, 255]),
            (DARK.minimize.pressed.foreground, [220, 220, 220, 255]),
            (DARK.minimize.disabled.background, [0, 0, 0, 0]),
            (DARK.minimize.disabled.foreground, [89, 89, 89, 255]),
            (DARK.maximize.normal.background, [0, 0, 0, 0]),
            (DARK.maximize.normal.foreground, [220, 220, 220, 255]),
            (DARK.maximize.hovered.background, [38, 38, 38, 255]),
            (DARK.maximize.hovered.foreground, [220, 220, 220, 255]),
            (DARK.maximize.pressed.background, [48, 48, 48, 255]),
            (DARK.maximize.pressed.foreground, [220, 220, 220, 255]),
            (DARK.maximize.disabled.background, [0, 0, 0, 0]),
            (DARK.maximize.disabled.foreground, [89, 89, 89, 255]),
            (DARK.close.normal.background, [0, 0, 0, 0]),
            (DARK.close.normal.foreground, [220, 220, 220, 255]),
            (DARK.close.hovered.background, [217, 54, 62, 255]),
            (DARK.close.hovered.foreground, [255, 255, 255, 255]),
            (DARK.close.pressed.background, [168, 7, 26, 255]),
            (DARK.close.pressed.foreground, [255, 255, 255, 255]),
            (DARK.close.disabled.background, [0, 0, 0, 0]),
            (DARK.close.disabled.foreground, [89, 89, 89, 255]),
            (DARK.image_tint, [255, 255, 255, 255]),
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
