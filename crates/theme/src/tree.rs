use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
