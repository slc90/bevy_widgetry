use crate::common::{dark, kamuri_violet, light, pink_dream};
use crate::list_view::WidgetryListViewContainerColors;
use crate::list_view::WidgetryListViewItemColors;
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTreeExpanderStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTreeExpanderInteractionColors {
    pub normal: WidgetryTreeExpanderStateColors,
    pub hovered: WidgetryTreeExpanderStateColors,
    pub pressed: WidgetryTreeExpanderStateColors,
    pub disabled: WidgetryTreeExpanderStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTreeExpanderColors {
    pub collapsed: WidgetryTreeExpanderInteractionColors,
    pub expanded: WidgetryTreeExpanderInteractionColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTreeColors {
    pub container: WidgetryListViewContainerColors,
    pub item: WidgetryListViewItemColors,
    pub expander: WidgetryTreeExpanderColors,
}

pub const LIGHT: WidgetryTreeColors = WidgetryTreeColors {
    container: crate::list_view::LIGHT.container,
    item: crate::list_view::LIGHT.item,
    expander: WidgetryTreeExpanderColors {
        collapsed: WidgetryTreeExpanderInteractionColors {
            normal: WidgetryTreeExpanderStateColors {
                background: light::TRANSPARENT,
                border: light::TRANSPARENT,
                foreground: light::TEXT,
            },
            hovered: WidgetryTreeExpanderStateColors {
                background: light::HOVER_SURFACE,
                border: light::TRANSPARENT,
                foreground: light::TEXT,
            },
            pressed: WidgetryTreeExpanderStateColors {
                background: light::PRESSED_SURFACE,
                border: light::TRANSPARENT,
                foreground: light::TEXT,
            },
            disabled: WidgetryTreeExpanderStateColors {
                background: light::TRANSPARENT,
                border: light::TRANSPARENT,
                foreground: light::DISABLED_TEXT,
            },
        },
        expanded: WidgetryTreeExpanderInteractionColors {
            normal: WidgetryTreeExpanderStateColors {
                background: light::TRANSPARENT,
                border: light::TRANSPARENT,
                foreground: light::TEXT,
            },
            hovered: WidgetryTreeExpanderStateColors {
                background: light::HOVER_SURFACE,
                border: light::TRANSPARENT,
                foreground: light::TEXT,
            },
            pressed: WidgetryTreeExpanderStateColors {
                background: light::PRESSED_SURFACE,
                border: light::TRANSPARENT,
                foreground: light::TEXT,
            },
            disabled: WidgetryTreeExpanderStateColors {
                background: light::TRANSPARENT,
                border: light::TRANSPARENT,
                foreground: light::DISABLED_TEXT,
            },
        },
    },
};

pub const DARK: WidgetryTreeColors = WidgetryTreeColors {
    container: crate::list_view::DARK.container,
    item: crate::list_view::DARK.item,
    expander: WidgetryTreeExpanderColors {
        collapsed: WidgetryTreeExpanderInteractionColors {
            normal: WidgetryTreeExpanderStateColors {
                background: dark::TRANSPARENT,
                border: dark::TRANSPARENT,
                foreground: dark::TEXT,
            },
            hovered: WidgetryTreeExpanderStateColors {
                background: dark::HOVER_SURFACE,
                border: dark::TRANSPARENT,
                foreground: dark::TEXT,
            },
            pressed: WidgetryTreeExpanderStateColors {
                background: dark::PRESSED_SURFACE,
                border: dark::TRANSPARENT,
                foreground: dark::TEXT,
            },
            disabled: WidgetryTreeExpanderStateColors {
                background: dark::TRANSPARENT,
                border: dark::TRANSPARENT,
                foreground: dark::DISABLED_TEXT,
            },
        },
        expanded: WidgetryTreeExpanderInteractionColors {
            normal: WidgetryTreeExpanderStateColors {
                background: dark::TRANSPARENT,
                border: dark::TRANSPARENT,
                foreground: dark::TEXT,
            },
            hovered: WidgetryTreeExpanderStateColors {
                background: dark::HOVER_SURFACE,
                border: dark::TRANSPARENT,
                foreground: dark::TEXT,
            },
            pressed: WidgetryTreeExpanderStateColors {
                background: dark::PRESSED_SURFACE,
                border: dark::TRANSPARENT,
                foreground: dark::TEXT,
            },
            disabled: WidgetryTreeExpanderStateColors {
                background: dark::TRANSPARENT,
                border: dark::TRANSPARENT,
                foreground: dark::DISABLED_TEXT,
            },
        },
    },
};

pub const PINK_DREAM: WidgetryTreeColors = WidgetryTreeColors {
    container: crate::list_view::PINK_DREAM.container,
    item: crate::list_view::PINK_DREAM.item,
    expander: WidgetryTreeExpanderColors {
        collapsed: WidgetryTreeExpanderInteractionColors {
            normal: WidgetryTreeExpanderStateColors {
                background: pink_dream::TRANSPARENT,
                border: pink_dream::TRANSPARENT,
                foreground: pink_dream::TEXT,
            },
            hovered: WidgetryTreeExpanderStateColors {
                background: pink_dream::HOVER_SURFACE,
                border: pink_dream::TRANSPARENT,
                foreground: pink_dream::TEXT,
            },
            pressed: WidgetryTreeExpanderStateColors {
                background: pink_dream::PRESSED_SURFACE,
                border: pink_dream::TRANSPARENT,
                foreground: pink_dream::TEXT,
            },
            disabled: WidgetryTreeExpanderStateColors {
                background: pink_dream::TRANSPARENT,
                border: pink_dream::TRANSPARENT,
                foreground: pink_dream::DISABLED_TEXT,
            },
        },
        expanded: WidgetryTreeExpanderInteractionColors {
            normal: WidgetryTreeExpanderStateColors {
                background: pink_dream::TRANSPARENT,
                border: pink_dream::TRANSPARENT,
                foreground: pink_dream::TEXT,
            },
            hovered: WidgetryTreeExpanderStateColors {
                background: pink_dream::HOVER_SURFACE,
                border: pink_dream::TRANSPARENT,
                foreground: pink_dream::TEXT,
            },
            pressed: WidgetryTreeExpanderStateColors {
                background: pink_dream::PRESSED_SURFACE,
                border: pink_dream::TRANSPARENT,
                foreground: pink_dream::TEXT,
            },
            disabled: WidgetryTreeExpanderStateColors {
                background: pink_dream::TRANSPARENT,
                border: pink_dream::TRANSPARENT,
                foreground: pink_dream::DISABLED_TEXT,
            },
        },
    },
};

pub const KAMURI_VIOLET: WidgetryTreeColors = WidgetryTreeColors {
    container: crate::list_view::KAMURI_VIOLET.container,
    item: crate::list_view::KAMURI_VIOLET.item,
    expander: WidgetryTreeExpanderColors {
        collapsed: WidgetryTreeExpanderInteractionColors {
            normal: WidgetryTreeExpanderStateColors {
                background: kamuri_violet::TRANSPARENT,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            hovered: WidgetryTreeExpanderStateColors {
                background: kamuri_violet::HOVER_SURFACE,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            pressed: WidgetryTreeExpanderStateColors {
                background: kamuri_violet::PRESSED_SURFACE,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryTreeExpanderStateColors {
                background: kamuri_violet::TRANSPARENT,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        expanded: WidgetryTreeExpanderInteractionColors {
            normal: WidgetryTreeExpanderStateColors {
                background: kamuri_violet::TRANSPARENT,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            hovered: WidgetryTreeExpanderStateColors {
                background: kamuri_violet::HOVER_SURFACE,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            pressed: WidgetryTreeExpanderStateColors {
                background: kamuri_violet::PRESSED_SURFACE,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryTreeExpanderStateColors {
                background: kamuri_violet::TRANSPARENT,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
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
            (
                PINK_DREAM.expander.collapsed.normal.background,
                0x00000000u32,
            ),
            (PINK_DREAM.expander.collapsed.normal.border, 0x00000000u32),
            (
                PINK_DREAM.expander.collapsed.normal.foreground,
                0x422B3CFFu32,
            ),
            (
                PINK_DREAM.expander.collapsed.hovered.background,
                0xF9DFEDFFu32,
            ),
            (PINK_DREAM.expander.collapsed.hovered.border, 0x00000000u32),
            (
                PINK_DREAM.expander.collapsed.hovered.foreground,
                0x422B3CFFu32,
            ),
            (
                PINK_DREAM.expander.collapsed.pressed.background,
                0xF2CFE2FFu32,
            ),
            (PINK_DREAM.expander.collapsed.pressed.border, 0x00000000u32),
            (
                PINK_DREAM.expander.collapsed.pressed.foreground,
                0x422B3CFFu32,
            ),
            (
                PINK_DREAM.expander.collapsed.disabled.background,
                0x00000000u32,
            ),
            (PINK_DREAM.expander.collapsed.disabled.border, 0x00000000u32),
            (
                PINK_DREAM.expander.collapsed.disabled.foreground,
                0xAC98A3FFu32,
            ),
            (
                PINK_DREAM.expander.expanded.normal.background,
                0x00000000u32,
            ),
            (PINK_DREAM.expander.expanded.normal.border, 0x00000000u32),
            (
                PINK_DREAM.expander.expanded.normal.foreground,
                0x422B3CFFu32,
            ),
            (
                PINK_DREAM.expander.expanded.hovered.background,
                0xF9DFEDFFu32,
            ),
            (PINK_DREAM.expander.expanded.hovered.border, 0x00000000u32),
            (
                PINK_DREAM.expander.expanded.hovered.foreground,
                0x422B3CFFu32,
            ),
            (
                PINK_DREAM.expander.expanded.pressed.background,
                0xF2CFE2FFu32,
            ),
            (PINK_DREAM.expander.expanded.pressed.border, 0x00000000u32),
            (
                PINK_DREAM.expander.expanded.pressed.foreground,
                0x422B3CFFu32,
            ),
            (
                PINK_DREAM.expander.expanded.disabled.background,
                0x00000000u32,
            ),
            (PINK_DREAM.expander.expanded.disabled.border, 0x00000000u32),
            (
                PINK_DREAM.expander.expanded.disabled.foreground,
                0xAC98A3FFu32,
            ),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(PINK_DREAM.container, crate::list_view::PINK_DREAM.container);
        assert_eq!(PINK_DREAM.item, crate::list_view::PINK_DREAM.item);
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (
                KAMURI_VIOLET.expander.collapsed.normal.background,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.normal.border,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.normal.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.hovered.background,
                0xE8DDF6FFu32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.hovered.border,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.hovered.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.pressed.background,
                0xDBCBECFFu32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.pressed.border,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.pressed.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.disabled.background,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.disabled.border,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.collapsed.disabled.foreground,
                0xB5A9C1FFu32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.normal.background,
                0x00000000u32,
            ),
            (KAMURI_VIOLET.expander.expanded.normal.border, 0x00000000u32),
            (
                KAMURI_VIOLET.expander.expanded.normal.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.hovered.background,
                0xE8DDF6FFu32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.hovered.border,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.hovered.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.pressed.background,
                0xDBCBECFFu32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.pressed.border,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.pressed.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.disabled.background,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.disabled.border,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.expander.expanded.disabled.foreground,
                0xB5A9C1FFu32,
            ),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(
            KAMURI_VIOLET.container,
            crate::list_view::KAMURI_VIOLET.container
        );
        assert_eq!(KAMURI_VIOLET.item, crate::list_view::KAMURI_VIOLET.item);
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.expander.collapsed.normal.background, [0, 0, 0, 0]),
            (LIGHT.expander.collapsed.normal.border, [0, 0, 0, 0]),
            (
                LIGHT.expander.collapsed.normal.foreground,
                [31, 31, 31, 255],
            ),
            (
                LIGHT.expander.collapsed.hovered.background,
                [245, 245, 245, 255],
            ),
            (LIGHT.expander.collapsed.hovered.border, [0, 0, 0, 0]),
            (
                LIGHT.expander.collapsed.hovered.foreground,
                [31, 31, 31, 255],
            ),
            (
                LIGHT.expander.collapsed.pressed.background,
                [235, 235, 235, 255],
            ),
            (LIGHT.expander.collapsed.pressed.border, [0, 0, 0, 0]),
            (
                LIGHT.expander.collapsed.pressed.foreground,
                [31, 31, 31, 255],
            ),
            (LIGHT.expander.collapsed.disabled.background, [0, 0, 0, 0]),
            (LIGHT.expander.collapsed.disabled.border, [0, 0, 0, 0]),
            (
                LIGHT.expander.collapsed.disabled.foreground,
                [191, 191, 191, 255],
            ),
            (LIGHT.expander.expanded.normal.background, [0, 0, 0, 0]),
            (LIGHT.expander.expanded.normal.border, [0, 0, 0, 0]),
            (LIGHT.expander.expanded.normal.foreground, [31, 31, 31, 255]),
            (
                LIGHT.expander.expanded.hovered.background,
                [245, 245, 245, 255],
            ),
            (LIGHT.expander.expanded.hovered.border, [0, 0, 0, 0]),
            (
                LIGHT.expander.expanded.hovered.foreground,
                [31, 31, 31, 255],
            ),
            (
                LIGHT.expander.expanded.pressed.background,
                [235, 235, 235, 255],
            ),
            (LIGHT.expander.expanded.pressed.border, [0, 0, 0, 0]),
            (
                LIGHT.expander.expanded.pressed.foreground,
                [31, 31, 31, 255],
            ),
            (LIGHT.expander.expanded.disabled.background, [0, 0, 0, 0]),
            (LIGHT.expander.expanded.disabled.border, [0, 0, 0, 0]),
            (
                LIGHT.expander.expanded.disabled.foreground,
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
        assert_eq!(LIGHT.container, crate::list_view::LIGHT.container);
        assert_eq!(LIGHT.item, crate::list_view::LIGHT.item);
    }
    #[test]
    fn dark_matches_reference() {
        let slots = [
            (DARK.expander.collapsed.normal.background, [0, 0, 0, 0]),
            (DARK.expander.collapsed.normal.border, [0, 0, 0, 0]),
            (
                DARK.expander.collapsed.normal.foreground,
                [220, 220, 220, 255],
            ),
            (
                DARK.expander.collapsed.hovered.background,
                [38, 38, 38, 255],
            ),
            (DARK.expander.collapsed.hovered.border, [0, 0, 0, 0]),
            (
                DARK.expander.collapsed.hovered.foreground,
                [220, 220, 220, 255],
            ),
            (
                DARK.expander.collapsed.pressed.background,
                [48, 48, 48, 255],
            ),
            (DARK.expander.collapsed.pressed.border, [0, 0, 0, 0]),
            (
                DARK.expander.collapsed.pressed.foreground,
                [220, 220, 220, 255],
            ),
            (DARK.expander.collapsed.disabled.background, [0, 0, 0, 0]),
            (DARK.expander.collapsed.disabled.border, [0, 0, 0, 0]),
            (
                DARK.expander.collapsed.disabled.foreground,
                [89, 89, 89, 255],
            ),
            (DARK.expander.expanded.normal.background, [0, 0, 0, 0]),
            (DARK.expander.expanded.normal.border, [0, 0, 0, 0]),
            (
                DARK.expander.expanded.normal.foreground,
                [220, 220, 220, 255],
            ),
            (DARK.expander.expanded.hovered.background, [38, 38, 38, 255]),
            (DARK.expander.expanded.hovered.border, [0, 0, 0, 0]),
            (
                DARK.expander.expanded.hovered.foreground,
                [220, 220, 220, 255],
            ),
            (DARK.expander.expanded.pressed.background, [48, 48, 48, 255]),
            (DARK.expander.expanded.pressed.border, [0, 0, 0, 0]),
            (
                DARK.expander.expanded.pressed.foreground,
                [220, 220, 220, 255],
            ),
            (DARK.expander.expanded.disabled.background, [0, 0, 0, 0]),
            (DARK.expander.expanded.disabled.border, [0, 0, 0, 0]),
            (
                DARK.expander.expanded.disabled.foreground,
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
        assert_eq!(DARK.container, crate::list_view::DARK.container);
        assert_eq!(DARK.item, crate::list_view::DARK.item);
    }
}
