use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryListViewContainerStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryListViewContainerColors {
    pub normal: WidgetryListViewContainerStateColors,
    pub focused: WidgetryListViewContainerStateColors,
    pub disabled: WidgetryListViewContainerStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryListViewItemStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryListViewItemColors {
    pub normal: WidgetryListViewItemStateColors,
    pub hovered: WidgetryListViewItemStateColors,
    pub pressed: WidgetryListViewItemStateColors,
    pub selected: WidgetryListViewItemStateColors,
    pub disabled: WidgetryListViewItemStateColors,
    pub active_border: Color,
    pub disabled_active_border: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryListViewColors {
    pub container: WidgetryListViewContainerColors,
    pub item: WidgetryListViewItemColors,
}

pub const LIGHT: WidgetryListViewColors = WidgetryListViewColors {
    container: WidgetryListViewContainerColors {
        normal: WidgetryListViewContainerStateColors {
            background: light::SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
        },
        focused: WidgetryListViewContainerStateColors {
            background: light::SURFACE,
            border: light::FOCUS_BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryListViewContainerStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
    },
    item: WidgetryListViewItemColors {
        normal: WidgetryListViewItemStateColors {
            background: light::TRANSPARENT,
            border: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        hovered: WidgetryListViewItemStateColors {
            background: light::HOVER_SURFACE,
            border: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        pressed: WidgetryListViewItemStateColors {
            background: light::PRESSED_SURFACE,
            border: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        selected: WidgetryListViewItemStateColors {
            background: light::SELECTED_SURFACE,
            border: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        disabled: WidgetryListViewItemStateColors {
            background: light::TRANSPARENT,
            border: light::TRANSPARENT,
            foreground: light::DISABLED_TEXT,
        },
        active_border: light::FOCUS_BORDER,
        disabled_active_border: light::TRANSPARENT,
    },
};

pub const DARK: WidgetryListViewColors = WidgetryListViewColors {
    container: WidgetryListViewContainerColors {
        normal: WidgetryListViewContainerStateColors {
            background: dark::SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
        },
        focused: WidgetryListViewContainerStateColors {
            background: dark::SURFACE,
            border: dark::FOCUS_BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryListViewContainerStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
    },
    item: WidgetryListViewItemColors {
        normal: WidgetryListViewItemStateColors {
            background: dark::TRANSPARENT,
            border: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        hovered: WidgetryListViewItemStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        pressed: WidgetryListViewItemStateColors {
            background: dark::PRESSED_SURFACE,
            border: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        selected: WidgetryListViewItemStateColors {
            background: dark::SELECTED_SURFACE,
            border: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        disabled: WidgetryListViewItemStateColors {
            background: dark::TRANSPARENT,
            border: dark::TRANSPARENT,
            foreground: dark::DISABLED_TEXT,
        },
        active_border: dark::FOCUS_BORDER,
        disabled_active_border: dark::TRANSPARENT,
    },
};

pub const PINK_DREAM: WidgetryListViewColors = WidgetryListViewColors {
    container: WidgetryListViewContainerColors {
        normal: WidgetryListViewContainerStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
        },
        focused: WidgetryListViewContainerStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::FOCUS_BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryListViewContainerStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    item: WidgetryListViewItemColors {
        normal: WidgetryListViewItemStateColors {
            background: pink_dream::TRANSPARENT,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryListViewItemStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        pressed: WidgetryListViewItemStateColors {
            background: pink_dream::PRESSED_SURFACE,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        selected: WidgetryListViewItemStateColors {
            background: pink_dream::SELECTED_SURFACE,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryListViewItemStateColors {
            background: pink_dream::TRANSPARENT,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::DISABLED_TEXT,
        },
        active_border: pink_dream::FOCUS_BORDER,
        disabled_active_border: pink_dream::TRANSPARENT,
    },
};

pub const KAMURI_VIOLET: WidgetryListViewColors = WidgetryListViewColors {
    container: WidgetryListViewContainerColors {
        normal: WidgetryListViewContainerStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
        },
        focused: WidgetryListViewContainerStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::FOCUS_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryListViewContainerStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    item: WidgetryListViewItemColors {
        normal: WidgetryListViewItemStateColors {
            background: kamuri_violet::TRANSPARENT,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryListViewItemStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        pressed: WidgetryListViewItemStateColors {
            background: kamuri_violet::PRESSED_SURFACE,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        selected: WidgetryListViewItemStateColors {
            background: kamuri_violet::SELECTED_SURFACE,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryListViewItemStateColors {
            background: kamuri_violet::TRANSPARENT,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
        active_border: kamuri_violet::FOCUS_BORDER,
        disabled_active_border: kamuri_violet::TRANSPARENT,
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
            (PINK_DREAM.item.normal.background, 0x00000000u32),
            (PINK_DREAM.item.normal.border, 0x00000000u32),
            (PINK_DREAM.item.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.item.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.item.hovered.border, 0x00000000u32),
            (PINK_DREAM.item.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.item.pressed.background, 0xF2CFE2FFu32),
            (PINK_DREAM.item.pressed.border, 0x00000000u32),
            (PINK_DREAM.item.pressed.foreground, 0x422B3CFFu32),
            (PINK_DREAM.item.selected.background, 0xF5D3E6FFu32),
            (PINK_DREAM.item.selected.border, 0x00000000u32),
            (PINK_DREAM.item.selected.foreground, 0x422B3CFFu32),
            (PINK_DREAM.item.disabled.background, 0x00000000u32),
            (PINK_DREAM.item.disabled.border, 0x00000000u32),
            (PINK_DREAM.item.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.item.active_border, 0xB4437DFFu32),
            (PINK_DREAM.item.disabled_active_border, 0x00000000u32),
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
            (KAMURI_VIOLET.item.normal.background, 0x00000000u32),
            (KAMURI_VIOLET.item.normal.border, 0x00000000u32),
            (KAMURI_VIOLET.item.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.item.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.item.hovered.border, 0x00000000u32),
            (KAMURI_VIOLET.item.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.item.pressed.background, 0xDBCBECFFu32),
            (KAMURI_VIOLET.item.pressed.border, 0x00000000u32),
            (KAMURI_VIOLET.item.pressed.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.item.selected.background, 0xDFD2F2FFu32),
            (KAMURI_VIOLET.item.selected.border, 0x00000000u32),
            (KAMURI_VIOLET.item.selected.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.item.disabled.background, 0x00000000u32),
            (KAMURI_VIOLET.item.disabled.border, 0x00000000u32),
            (KAMURI_VIOLET.item.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.item.active_border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.item.disabled_active_border, 0x00000000u32),
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
            (LIGHT.item.normal.background, [0, 0, 0, 0]),
            (LIGHT.item.normal.border, [0, 0, 0, 0]),
            (LIGHT.item.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.item.hovered.background, [245, 245, 245, 255]),
            (LIGHT.item.hovered.border, [0, 0, 0, 0]),
            (LIGHT.item.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.item.pressed.background, [235, 235, 235, 255]),
            (LIGHT.item.pressed.border, [0, 0, 0, 0]),
            (LIGHT.item.pressed.foreground, [31, 31, 31, 255]),
            (LIGHT.item.selected.background, [230, 244, 255, 255]),
            (LIGHT.item.selected.border, [0, 0, 0, 0]),
            (LIGHT.item.selected.foreground, [31, 31, 31, 255]),
            (LIGHT.item.disabled.background, [0, 0, 0, 0]),
            (LIGHT.item.disabled.border, [0, 0, 0, 0]),
            (LIGHT.item.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.item.active_border, [22, 119, 255, 255]),
            (LIGHT.item.disabled_active_border, [0, 0, 0, 0]),
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
            (DARK.item.normal.background, [0, 0, 0, 0]),
            (DARK.item.normal.border, [0, 0, 0, 0]),
            (DARK.item.normal.foreground, [220, 220, 220, 255]),
            (DARK.item.hovered.background, [38, 38, 38, 255]),
            (DARK.item.hovered.border, [0, 0, 0, 0]),
            (DARK.item.hovered.foreground, [220, 220, 220, 255]),
            (DARK.item.pressed.background, [48, 48, 48, 255]),
            (DARK.item.pressed.border, [0, 0, 0, 0]),
            (DARK.item.pressed.foreground, [220, 220, 220, 255]),
            (DARK.item.selected.background, [17, 26, 44, 255]),
            (DARK.item.selected.border, [0, 0, 0, 0]),
            (DARK.item.selected.foreground, [220, 220, 220, 255]),
            (DARK.item.disabled.background, [0, 0, 0, 0]),
            (DARK.item.disabled.border, [0, 0, 0, 0]),
            (DARK.item.disabled.foreground, [89, 89, 89, 255]),
            (DARK.item.active_border, [60, 137, 232, 255]),
            (DARK.item.disabled_active_border, [0, 0, 0, 0]),
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
