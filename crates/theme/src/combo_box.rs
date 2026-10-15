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

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn pink_dream_reuses_widget_colors() {
        assert_eq!(PINK_DREAM.list.item, crate::list_view::PINK_DREAM.item);
    }

    #[test]
    fn kamuri_violet_reuses_widget_colors() {
        assert_eq!(
            KAMURI_VIOLET.list.item,
            crate::list_view::KAMURI_VIOLET.item
        );
    }

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.field.normal.background,
            LIGHT.field.normal.border,
            LIGHT.field.normal.foreground,
            LIGHT.field.normal.indicator_foreground,
            LIGHT.field.hovered.background,
            LIGHT.field.hovered.border,
            LIGHT.field.hovered.foreground,
            LIGHT.field.hovered.indicator_foreground,
            LIGHT.field.pressed.background,
            LIGHT.field.pressed.border,
            LIGHT.field.pressed.foreground,
            LIGHT.field.pressed.indicator_foreground,
            LIGHT.field.open.background,
            LIGHT.field.open.border,
            LIGHT.field.open.foreground,
            LIGHT.field.open.indicator_foreground,
            LIGHT.field.disabled.background,
            LIGHT.field.disabled.border,
            LIGHT.field.disabled.foreground,
            LIGHT.field.disabled.indicator_foreground,
            LIGHT.popup.normal.background,
            LIGHT.popup.normal.border,
            LIGHT.popup.normal.foreground,
            LIGHT.popup.disabled.background,
            LIGHT.popup.disabled.border,
            LIGHT.popup.disabled.foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
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
    fn dark_colors_are_finite() {
        let colors = [
            DARK.field.normal.background,
            DARK.field.normal.border,
            DARK.field.normal.foreground,
            DARK.field.normal.indicator_foreground,
            DARK.field.hovered.background,
            DARK.field.hovered.border,
            DARK.field.hovered.foreground,
            DARK.field.hovered.indicator_foreground,
            DARK.field.pressed.background,
            DARK.field.pressed.border,
            DARK.field.pressed.foreground,
            DARK.field.pressed.indicator_foreground,
            DARK.field.open.background,
            DARK.field.open.border,
            DARK.field.open.foreground,
            DARK.field.open.indicator_foreground,
            DARK.field.disabled.background,
            DARK.field.disabled.border,
            DARK.field.disabled.foreground,
            DARK.field.disabled.indicator_foreground,
            DARK.popup.normal.background,
            DARK.popup.normal.border,
            DARK.popup.normal.foreground,
            DARK.popup.disabled.background,
            DARK.popup.disabled.border,
            DARK.popup.disabled.foreground,
        ];
        for color in colors {
            let actual = color.to_srgba();
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
