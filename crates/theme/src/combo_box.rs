use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
