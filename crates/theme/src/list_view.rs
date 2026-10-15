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

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.container.normal.background,
            LIGHT.container.normal.border,
            LIGHT.container.normal.foreground,
            LIGHT.container.focused.background,
            LIGHT.container.focused.border,
            LIGHT.container.focused.foreground,
            LIGHT.container.disabled.background,
            LIGHT.container.disabled.border,
            LIGHT.container.disabled.foreground,
            LIGHT.item.normal.background,
            LIGHT.item.normal.border,
            LIGHT.item.normal.foreground,
            LIGHT.item.hovered.background,
            LIGHT.item.hovered.border,
            LIGHT.item.hovered.foreground,
            LIGHT.item.pressed.background,
            LIGHT.item.pressed.border,
            LIGHT.item.pressed.foreground,
            LIGHT.item.selected.background,
            LIGHT.item.selected.border,
            LIGHT.item.selected.foreground,
            LIGHT.item.disabled.background,
            LIGHT.item.disabled.border,
            LIGHT.item.disabled.foreground,
            LIGHT.item.active_border,
            LIGHT.item.disabled_active_border,
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
            DARK.container.normal.background,
            DARK.container.normal.border,
            DARK.container.normal.foreground,
            DARK.container.focused.background,
            DARK.container.focused.border,
            DARK.container.focused.foreground,
            DARK.container.disabled.background,
            DARK.container.disabled.border,
            DARK.container.disabled.foreground,
            DARK.item.normal.background,
            DARK.item.normal.border,
            DARK.item.normal.foreground,
            DARK.item.hovered.background,
            DARK.item.hovered.border,
            DARK.item.hovered.foreground,
            DARK.item.pressed.background,
            DARK.item.pressed.border,
            DARK.item.pressed.foreground,
            DARK.item.selected.background,
            DARK.item.selected.border,
            DARK.item.selected.foreground,
            DARK.item.disabled.background,
            DARK.item.disabled.border,
            DARK.item.disabled.foreground,
            DARK.item.active_border,
            DARK.item.disabled_active_border,
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
