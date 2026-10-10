use crate::common::{dark, kamuri_violet, light, pink_dream};
use crate::list_view::WidgetryListViewColors;
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryComboBoxFieldStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
    pub indicator_foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryComboBoxFieldColors {
    pub normal: WidgetryComboBoxFieldStateColors,
    pub hovered: WidgetryComboBoxFieldStateColors,
    pub pressed: WidgetryComboBoxFieldStateColors,
    pub open: WidgetryComboBoxFieldStateColors,
    pub disabled: WidgetryComboBoxFieldStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryComboBoxPopupStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryComboBoxPopupColors {
    pub normal: WidgetryComboBoxPopupStateColors,
    pub disabled: WidgetryComboBoxPopupStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryComboBoxColors {
    pub field: WidgetryComboBoxFieldColors,
    pub popup: WidgetryComboBoxPopupColors,
    pub list: WidgetryListViewColors,
}

pub const LIGHT: WidgetryComboBoxColors = WidgetryComboBoxColors {
    field: WidgetryComboBoxFieldColors {
        normal: WidgetryComboBoxFieldStateColors {
            background: light::SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
            indicator_foreground: light::TERTIARY_TEXT,
        },
        hovered: WidgetryComboBoxFieldStateColors {
            background: light::HOVER_SURFACE,
            border: light::PRIMARY_HOVER,
            foreground: light::TEXT,
            indicator_foreground: light::TERTIARY_TEXT,
        },
        pressed: WidgetryComboBoxFieldStateColors {
            background: light::PRESSED_SURFACE,
            border: light::PRIMARY_PRESSED,
            foreground: light::TEXT,
            indicator_foreground: light::TERTIARY_TEXT,
        },
        open: WidgetryComboBoxFieldStateColors {
            background: light::SURFACE,
            border: light::FOCUS_BORDER,
            foreground: light::TEXT,
            indicator_foreground: light::TEXT,
        },
        disabled: WidgetryComboBoxFieldStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
            indicator_foreground: light::DISABLED_TEXT,
        },
    },
    popup: WidgetryComboBoxPopupColors {
        normal: WidgetryComboBoxPopupStateColors {
            background: light::ELEVATED_SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryComboBoxPopupStateColors {
            background: light::ELEVATED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
    },
    list: crate::list_view::WidgetryListViewColors {
        container: crate::list_view::WidgetryListViewContainerColors {
            normal: crate::list_view::WidgetryListViewContainerStateColors {
                background: light::ELEVATED_SURFACE,
                border: light::BORDER,
                foreground: light::TEXT,
            },
            focused: crate::list_view::WidgetryListViewContainerStateColors {
                background: light::ELEVATED_SURFACE,
                border: light::FOCUS_BORDER,
                foreground: light::TEXT,
            },
            disabled: crate::list_view::WidgetryListViewContainerStateColors {
                background: light::ELEVATED_SURFACE,
                border: light::DISABLED_BORDER,
                foreground: light::DISABLED_TEXT,
            },
        },
        item: crate::list_view::LIGHT.item,
    },
};

pub const DARK: WidgetryComboBoxColors = WidgetryComboBoxColors {
    field: WidgetryComboBoxFieldColors {
        normal: WidgetryComboBoxFieldStateColors {
            background: dark::SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
            indicator_foreground: dark::TERTIARY_TEXT,
        },
        hovered: WidgetryComboBoxFieldStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::PRIMARY_HOVER,
            foreground: dark::TEXT,
            indicator_foreground: dark::TERTIARY_TEXT,
        },
        pressed: WidgetryComboBoxFieldStateColors {
            background: dark::PRESSED_SURFACE,
            border: dark::PRIMARY_PRESSED,
            foreground: dark::TEXT,
            indicator_foreground: dark::TERTIARY_TEXT,
        },
        open: WidgetryComboBoxFieldStateColors {
            background: dark::SURFACE,
            border: dark::FOCUS_BORDER,
            foreground: dark::TEXT,
            indicator_foreground: dark::TEXT,
        },
        disabled: WidgetryComboBoxFieldStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
            indicator_foreground: dark::DISABLED_TEXT,
        },
    },
    popup: WidgetryComboBoxPopupColors {
        normal: WidgetryComboBoxPopupStateColors {
            background: dark::ELEVATED_SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryComboBoxPopupStateColors {
            background: dark::ELEVATED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
    },
    list: crate::list_view::WidgetryListViewColors {
        container: crate::list_view::WidgetryListViewContainerColors {
            normal: crate::list_view::WidgetryListViewContainerStateColors {
                background: dark::ELEVATED_SURFACE,
                border: dark::BORDER,
                foreground: dark::TEXT,
            },
            focused: crate::list_view::WidgetryListViewContainerStateColors {
                background: dark::ELEVATED_SURFACE,
                border: dark::FOCUS_BORDER,
                foreground: dark::TEXT,
            },
            disabled: crate::list_view::WidgetryListViewContainerStateColors {
                background: dark::ELEVATED_SURFACE,
                border: dark::DISABLED_BORDER,
                foreground: dark::DISABLED_TEXT,
            },
        },
        item: crate::list_view::DARK.item,
    },
};

pub const PINK_DREAM: WidgetryComboBoxColors = WidgetryComboBoxColors {
    field: WidgetryComboBoxFieldColors {
        normal: WidgetryComboBoxFieldStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
            indicator_foreground: pink_dream::TERTIARY_TEXT,
        },
        hovered: WidgetryComboBoxFieldStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::PRIMARY_HOVER,
            foreground: pink_dream::TEXT,
            indicator_foreground: pink_dream::TERTIARY_TEXT,
        },
        pressed: WidgetryComboBoxFieldStateColors {
            background: pink_dream::PRESSED_SURFACE,
            border: pink_dream::PRIMARY_PRESSED,
            foreground: pink_dream::TEXT,
            indicator_foreground: pink_dream::TERTIARY_TEXT,
        },
        open: WidgetryComboBoxFieldStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::FOCUS_BORDER,
            foreground: pink_dream::TEXT,
            indicator_foreground: pink_dream::TEXT,
        },
        disabled: WidgetryComboBoxFieldStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
            indicator_foreground: pink_dream::DISABLED_TEXT,
        },
    },
    popup: WidgetryComboBoxPopupColors {
        normal: WidgetryComboBoxPopupStateColors {
            background: pink_dream::ELEVATED_SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryComboBoxPopupStateColors {
            background: pink_dream::ELEVATED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    list: crate::list_view::WidgetryListViewColors {
        container: crate::list_view::WidgetryListViewContainerColors {
            normal: crate::list_view::WidgetryListViewContainerStateColors {
                background: pink_dream::ELEVATED_SURFACE,
                border: pink_dream::BORDER,
                foreground: pink_dream::TEXT,
            },
            focused: crate::list_view::WidgetryListViewContainerStateColors {
                background: pink_dream::ELEVATED_SURFACE,
                border: pink_dream::FOCUS_BORDER,
                foreground: pink_dream::TEXT,
            },
            disabled: crate::list_view::WidgetryListViewContainerStateColors {
                background: pink_dream::ELEVATED_SURFACE,
                border: pink_dream::DISABLED_BORDER,
                foreground: pink_dream::DISABLED_TEXT,
            },
        },
        item: crate::list_view::PINK_DREAM.item,
    },
};

pub const KAMURI_VIOLET: WidgetryComboBoxColors = WidgetryComboBoxColors {
    field: WidgetryComboBoxFieldColors {
        normal: WidgetryComboBoxFieldStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
            indicator_foreground: kamuri_violet::TERTIARY_TEXT,
        },
        hovered: WidgetryComboBoxFieldStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::PRIMARY_HOVER,
            foreground: kamuri_violet::TEXT,
            indicator_foreground: kamuri_violet::TERTIARY_TEXT,
        },
        pressed: WidgetryComboBoxFieldStateColors {
            background: kamuri_violet::PRESSED_SURFACE,
            border: kamuri_violet::PRIMARY_PRESSED,
            foreground: kamuri_violet::TEXT,
            indicator_foreground: kamuri_violet::TERTIARY_TEXT,
        },
        open: WidgetryComboBoxFieldStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::FOCUS_BORDER,
            foreground: kamuri_violet::TEXT,
            indicator_foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryComboBoxFieldStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
            indicator_foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    popup: WidgetryComboBoxPopupColors {
        normal: WidgetryComboBoxPopupStateColors {
            background: kamuri_violet::ELEVATED_SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryComboBoxPopupStateColors {
            background: kamuri_violet::ELEVATED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    list: crate::list_view::WidgetryListViewColors {
        container: crate::list_view::WidgetryListViewContainerColors {
            normal: crate::list_view::WidgetryListViewContainerStateColors {
                background: kamuri_violet::ELEVATED_SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
            },
            focused: crate::list_view::WidgetryListViewContainerStateColors {
                background: kamuri_violet::ELEVATED_SURFACE,
                border: kamuri_violet::FOCUS_BORDER,
                foreground: kamuri_violet::TEXT,
            },
            disabled: crate::list_view::WidgetryListViewContainerStateColors {
                background: kamuri_violet::ELEVATED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        item: crate::list_view::KAMURI_VIOLET.item,
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
            (PINK_DREAM.field.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.field.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.field.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.field.normal.indicator_foreground, 0x927A88FFu32),
            (PINK_DREAM.field.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.field.hovered.border, 0xCD6196FFu32),
            (PINK_DREAM.field.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.field.hovered.indicator_foreground, 0x927A88FFu32),
            (PINK_DREAM.field.pressed.background, 0xF2CFE2FFu32),
            (PINK_DREAM.field.pressed.border, 0x913063FFu32),
            (PINK_DREAM.field.pressed.foreground, 0x422B3CFFu32),
            (PINK_DREAM.field.pressed.indicator_foreground, 0x927A88FFu32),
            (PINK_DREAM.field.open.background, 0xFCEAF3FFu32),
            (PINK_DREAM.field.open.border, 0xB4437DFFu32),
            (PINK_DREAM.field.open.foreground, 0x422B3CFFu32),
            (PINK_DREAM.field.open.indicator_foreground, 0x422B3CFFu32),
            (PINK_DREAM.field.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.field.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.field.disabled.foreground, 0xAC98A3FFu32),
            (
                PINK_DREAM.field.disabled.indicator_foreground,
                0xAC98A3FFu32,
            ),
            (PINK_DREAM.popup.normal.background, 0xFFF3F9FFu32),
            (PINK_DREAM.popup.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.popup.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.popup.disabled.background, 0xFFF3F9FFu32),
            (PINK_DREAM.popup.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.popup.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.list.container.normal.background, 0xFFF3F9FFu32),
            (PINK_DREAM.list.container.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.list.container.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.list.container.focused.background, 0xFFF3F9FFu32),
            (PINK_DREAM.list.container.focused.border, 0xB4437DFFu32),
            (PINK_DREAM.list.container.focused.foreground, 0x422B3CFFu32),
            (PINK_DREAM.list.container.disabled.background, 0xFFF3F9FFu32),
            (PINK_DREAM.list.container.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.list.container.disabled.foreground, 0xAC98A3FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(PINK_DREAM.list.item, crate::list_view::PINK_DREAM.item);
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.field.normal.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.field.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.field.normal.foreground, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.field.normal.indicator_foreground,
                0x9A8CA6FFu32,
            ),
            (KAMURI_VIOLET.field.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.field.hovered.border, 0xA181D2FFu32),
            (KAMURI_VIOLET.field.hovered.foreground, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.field.hovered.indicator_foreground,
                0x9A8CA6FFu32,
            ),
            (KAMURI_VIOLET.field.pressed.background, 0xDBCBECFFu32),
            (KAMURI_VIOLET.field.pressed.border, 0x6E4EA3FFu32),
            (KAMURI_VIOLET.field.pressed.foreground, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.field.pressed.indicator_foreground,
                0x9A8CA6FFu32,
            ),
            (KAMURI_VIOLET.field.open.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.field.open.border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.field.open.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.field.open.indicator_foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.field.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.field.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.field.disabled.foreground, 0xB5A9C1FFu32),
            (
                KAMURI_VIOLET.field.disabled.indicator_foreground,
                0xB5A9C1FFu32,
            ),
            (KAMURI_VIOLET.popup.normal.background, 0xF7F2FDFFu32),
            (KAMURI_VIOLET.popup.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.popup.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.popup.disabled.background, 0xF7F2FDFFu32),
            (KAMURI_VIOLET.popup.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.popup.disabled.foreground, 0xB5A9C1FFu32),
            (
                KAMURI_VIOLET.list.container.normal.background,
                0xF7F2FDFFu32,
            ),
            (KAMURI_VIOLET.list.container.normal.border, 0xD8CDE4FFu32),
            (
                KAMURI_VIOLET.list.container.normal.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.list.container.focused.background,
                0xF7F2FDFFu32,
            ),
            (KAMURI_VIOLET.list.container.focused.border, 0x8C6BC1FFu32),
            (
                KAMURI_VIOLET.list.container.focused.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.list.container.disabled.background,
                0xF7F2FDFFu32,
            ),
            (KAMURI_VIOLET.list.container.disabled.border, 0xE7DFF0FFu32),
            (
                KAMURI_VIOLET.list.container.disabled.foreground,
                0xB5A9C1FFu32,
            ),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(
            KAMURI_VIOLET.list.item,
            crate::list_view::KAMURI_VIOLET.item
        );
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.field.normal.background, [255, 255, 255, 255]),
            (LIGHT.field.normal.border, [217, 217, 217, 255]),
            (LIGHT.field.normal.foreground, [31, 31, 31, 255]),
            (
                LIGHT.field.normal.indicator_foreground,
                [140, 140, 140, 255],
            ),
            (LIGHT.field.hovered.background, [245, 245, 245, 255]),
            (LIGHT.field.hovered.border, [64, 150, 255, 255]),
            (LIGHT.field.hovered.foreground, [31, 31, 31, 255]),
            (
                LIGHT.field.hovered.indicator_foreground,
                [140, 140, 140, 255],
            ),
            (LIGHT.field.pressed.background, [235, 235, 235, 255]),
            (LIGHT.field.pressed.border, [9, 88, 217, 255]),
            (LIGHT.field.pressed.foreground, [31, 31, 31, 255]),
            (
                LIGHT.field.pressed.indicator_foreground,
                [140, 140, 140, 255],
            ),
            (LIGHT.field.open.background, [255, 255, 255, 255]),
            (LIGHT.field.open.border, [22, 119, 255, 255]),
            (LIGHT.field.open.foreground, [31, 31, 31, 255]),
            (LIGHT.field.open.indicator_foreground, [31, 31, 31, 255]),
            (LIGHT.field.disabled.background, [245, 245, 245, 255]),
            (LIGHT.field.disabled.border, [217, 217, 217, 255]),
            (LIGHT.field.disabled.foreground, [191, 191, 191, 255]),
            (
                LIGHT.field.disabled.indicator_foreground,
                [191, 191, 191, 255],
            ),
            (LIGHT.popup.normal.background, [255, 255, 255, 255]),
            (LIGHT.popup.normal.border, [217, 217, 217, 255]),
            (LIGHT.popup.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.popup.disabled.background, [255, 255, 255, 255]),
            (LIGHT.popup.disabled.border, [217, 217, 217, 255]),
            (LIGHT.popup.disabled.foreground, [191, 191, 191, 255]),
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
        assert_eq!(
            LIGHT.list,
            crate::list_view::WidgetryListViewColors {
                container: crate::list_view::WidgetryListViewContainerColors {
                    normal: crate::list_view::WidgetryListViewContainerStateColors {
                        background: light::ELEVATED_SURFACE,
                        border: light::BORDER,
                        foreground: light::TEXT,
                    },
                    focused: crate::list_view::WidgetryListViewContainerStateColors {
                        background: light::ELEVATED_SURFACE,
                        border: light::FOCUS_BORDER,
                        foreground: light::TEXT,
                    },
                    disabled: crate::list_view::WidgetryListViewContainerStateColors {
                        background: light::ELEVATED_SURFACE,
                        border: light::DISABLED_BORDER,
                        foreground: light::DISABLED_TEXT,
                    },
                },
                item: crate::list_view::LIGHT.item,
            }
        );
    }
    #[test]
    fn dark_matches_reference() {
        let slots = [
            (DARK.field.normal.background, [20, 20, 20, 255]),
            (DARK.field.normal.border, [66, 66, 66, 255]),
            (DARK.field.normal.foreground, [220, 220, 220, 255]),
            (DARK.field.normal.indicator_foreground, [126, 126, 126, 255]),
            (DARK.field.hovered.background, [38, 38, 38, 255]),
            (DARK.field.hovered.border, [60, 137, 232, 255]),
            (DARK.field.hovered.foreground, [220, 220, 220, 255]),
            (
                DARK.field.hovered.indicator_foreground,
                [126, 126, 126, 255],
            ),
            (DARK.field.pressed.background, [48, 48, 48, 255]),
            (DARK.field.pressed.border, [21, 84, 173, 255]),
            (DARK.field.pressed.foreground, [220, 220, 220, 255]),
            (
                DARK.field.pressed.indicator_foreground,
                [126, 126, 126, 255],
            ),
            (DARK.field.open.background, [20, 20, 20, 255]),
            (DARK.field.open.border, [60, 137, 232, 255]),
            (DARK.field.open.foreground, [220, 220, 220, 255]),
            (DARK.field.open.indicator_foreground, [220, 220, 220, 255]),
            (DARK.field.disabled.background, [31, 31, 31, 255]),
            (DARK.field.disabled.border, [66, 66, 66, 255]),
            (DARK.field.disabled.foreground, [89, 89, 89, 255]),
            (DARK.field.disabled.indicator_foreground, [89, 89, 89, 255]),
            (DARK.popup.normal.background, [31, 31, 31, 255]),
            (DARK.popup.normal.border, [66, 66, 66, 255]),
            (DARK.popup.normal.foreground, [220, 220, 220, 255]),
            (DARK.popup.disabled.background, [31, 31, 31, 255]),
            (DARK.popup.disabled.border, [66, 66, 66, 255]),
            (DARK.popup.disabled.foreground, [89, 89, 89, 255]),
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
        assert_eq!(
            DARK.list,
            crate::list_view::WidgetryListViewColors {
                container: crate::list_view::WidgetryListViewContainerColors {
                    normal: crate::list_view::WidgetryListViewContainerStateColors {
                        background: dark::ELEVATED_SURFACE,
                        border: dark::BORDER,
                        foreground: dark::TEXT,
                    },
                    focused: crate::list_view::WidgetryListViewContainerStateColors {
                        background: dark::ELEVATED_SURFACE,
                        border: dark::FOCUS_BORDER,
                        foreground: dark::TEXT,
                    },
                    disabled: crate::list_view::WidgetryListViewContainerStateColors {
                        background: dark::ELEVATED_SURFACE,
                        border: dark::DISABLED_BORDER,
                        foreground: dark::DISABLED_TEXT,
                    },
                },
                item: crate::list_view::DARK.item,
            }
        );
    }
}
