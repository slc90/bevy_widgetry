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
            (PINK_DREAM.frame.normal.background, 0xF6DCE9FFu32),
            (PINK_DREAM.frame.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.frame.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.frame.disabled.background, 0xF6DCE9FFu32),
            (PINK_DREAM.frame.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.frame.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.title_bar.normal.background, 0x00000000u32),
            (PINK_DREAM.title_bar.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.title_bar.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.title_bar.disabled.background, 0x00000000u32),
            (PINK_DREAM.title_bar.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.title_bar.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.minimize.normal.background, 0x00000000u32),
            (PINK_DREAM.minimize.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.minimize.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.minimize.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.minimize.pressed.background, 0xF2CFE2FFu32),
            (PINK_DREAM.minimize.pressed.foreground, 0x422B3CFFu32),
            (PINK_DREAM.minimize.disabled.background, 0x00000000u32),
            (PINK_DREAM.minimize.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.maximize.normal.background, 0x00000000u32),
            (PINK_DREAM.maximize.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.maximize.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.maximize.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.maximize.pressed.background, 0xF2CFE2FFu32),
            (PINK_DREAM.maximize.pressed.foreground, 0x422B3CFFu32),
            (PINK_DREAM.maximize.disabled.background, 0x00000000u32),
            (PINK_DREAM.maximize.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.close.normal.background, 0x00000000u32),
            (PINK_DREAM.close.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.close.hovered.background, 0xD4495DFFu32),
            (PINK_DREAM.close.hovered.foreground, 0xFFFFFFFFu32),
            (PINK_DREAM.close.pressed.background, 0xAA263EFFu32),
            (PINK_DREAM.close.pressed.foreground, 0xFFFFFFFFu32),
            (PINK_DREAM.close.disabled.background, 0x00000000u32),
            (PINK_DREAM.close.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.image_tint, 0xFFFFFFFFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.frame.normal.background, 0xE3D9F1FFu32),
            (KAMURI_VIOLET.frame.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.frame.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.frame.disabled.background, 0xE3D9F1FFu32),
            (KAMURI_VIOLET.frame.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.frame.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.title_bar.normal.background, 0x00000000u32),
            (KAMURI_VIOLET.title_bar.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.title_bar.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.title_bar.disabled.background, 0x00000000u32),
            (KAMURI_VIOLET.title_bar.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.title_bar.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.minimize.normal.background, 0x00000000u32),
            (KAMURI_VIOLET.minimize.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.minimize.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.minimize.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.minimize.pressed.background, 0xDBCBECFFu32),
            (KAMURI_VIOLET.minimize.pressed.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.minimize.disabled.background, 0x00000000u32),
            (KAMURI_VIOLET.minimize.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.maximize.normal.background, 0x00000000u32),
            (KAMURI_VIOLET.maximize.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.maximize.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.maximize.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.maximize.pressed.background, 0xDBCBECFFu32),
            (KAMURI_VIOLET.maximize.pressed.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.maximize.disabled.background, 0x00000000u32),
            (KAMURI_VIOLET.maximize.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.close.normal.background, 0x00000000u32),
            (KAMURI_VIOLET.close.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.close.hovered.background, 0x9B5C86FFu32),
            (KAMURI_VIOLET.close.hovered.foreground, 0xFFFFFFFFu32),
            (KAMURI_VIOLET.close.pressed.background, 0x7A456BFFu32),
            (KAMURI_VIOLET.close.pressed.foreground, 0xFFFFFFFFu32),
            (KAMURI_VIOLET.close.disabled.background, 0x00000000u32),
            (KAMURI_VIOLET.close.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.image_tint, 0xFFFFFFFFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

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
