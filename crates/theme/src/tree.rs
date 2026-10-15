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

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn pink_dream_reuses_widget_colors() {
        assert_eq!(PINK_DREAM.container, crate::list_view::PINK_DREAM.container);
        assert_eq!(PINK_DREAM.item, crate::list_view::PINK_DREAM.item);
    }

    #[test]
    fn kamuri_violet_reuses_widget_colors() {
        assert_eq!(
            KAMURI_VIOLET.container,
            crate::list_view::KAMURI_VIOLET.container
        );
        assert_eq!(KAMURI_VIOLET.item, crate::list_view::KAMURI_VIOLET.item);
    }

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.expander.collapsed.normal.background,
            LIGHT.expander.collapsed.normal.border,
            LIGHT.expander.collapsed.normal.foreground,
            LIGHT.expander.collapsed.hovered.background,
            LIGHT.expander.collapsed.hovered.border,
            LIGHT.expander.collapsed.hovered.foreground,
            LIGHT.expander.collapsed.pressed.background,
            LIGHT.expander.collapsed.pressed.border,
            LIGHT.expander.collapsed.pressed.foreground,
            LIGHT.expander.collapsed.disabled.background,
            LIGHT.expander.collapsed.disabled.border,
            LIGHT.expander.collapsed.disabled.foreground,
            LIGHT.expander.expanded.normal.background,
            LIGHT.expander.expanded.normal.border,
            LIGHT.expander.expanded.normal.foreground,
            LIGHT.expander.expanded.hovered.background,
            LIGHT.expander.expanded.hovered.border,
            LIGHT.expander.expanded.hovered.foreground,
            LIGHT.expander.expanded.pressed.background,
            LIGHT.expander.expanded.pressed.border,
            LIGHT.expander.expanded.pressed.foreground,
            LIGHT.expander.expanded.disabled.background,
            LIGHT.expander.expanded.disabled.border,
            LIGHT.expander.expanded.disabled.foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
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
    fn dark_colors_are_finite() {
        let colors = [
            DARK.expander.collapsed.normal.background,
            DARK.expander.collapsed.normal.border,
            DARK.expander.collapsed.normal.foreground,
            DARK.expander.collapsed.hovered.background,
            DARK.expander.collapsed.hovered.border,
            DARK.expander.collapsed.hovered.foreground,
            DARK.expander.collapsed.pressed.background,
            DARK.expander.collapsed.pressed.border,
            DARK.expander.collapsed.pressed.foreground,
            DARK.expander.collapsed.disabled.background,
            DARK.expander.collapsed.disabled.border,
            DARK.expander.collapsed.disabled.foreground,
            DARK.expander.expanded.normal.background,
            DARK.expander.expanded.normal.border,
            DARK.expander.expanded.normal.foreground,
            DARK.expander.expanded.hovered.background,
            DARK.expander.expanded.hovered.border,
            DARK.expander.expanded.hovered.foreground,
            DARK.expander.expanded.pressed.background,
            DARK.expander.expanded.pressed.border,
            DARK.expander.expanded.pressed.foreground,
            DARK.expander.expanded.disabled.background,
            DARK.expander.expanded.disabled.border,
            DARK.expander.expanded.disabled.foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
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
