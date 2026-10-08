use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
