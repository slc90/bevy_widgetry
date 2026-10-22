use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTabStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTabInteractionColors {
    pub normal: WidgetryTabStateColors,
    pub hovered: WidgetryTabStateColors,
    pub pressed: WidgetryTabStateColors,
    pub disabled: WidgetryTabStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTabColors {
    pub unselected: WidgetryTabInteractionColors,
    pub selected: WidgetryTabInteractionColors,
    pub focus_border: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTabsColors {
    pub header: WidgetryTabColors,
    pub bar_background: Color,
    pub panel_background: Color,
    pub separator: Color,
}

macro_rules! palette {
    ($name:ident, $base:ident) => {
        pub const $name: WidgetryTabsColors = WidgetryTabsColors {
            header: WidgetryTabColors {
                unselected: WidgetryTabInteractionColors {
                    normal: state($base::HOVER_SURFACE, $base::BORDER, $base::TEXT),
                    hovered: state($base::PRESSED_SURFACE, $base::BORDER, $base::TEXT),
                    pressed: state($base::SELECTED_SURFACE, $base::BORDER, $base::TEXT),
                    disabled: state(
                        $base::DISABLED_SURFACE,
                        $base::DISABLED_BORDER,
                        $base::DISABLED_TEXT,
                    ),
                },
                selected: WidgetryTabInteractionColors {
                    normal: state($base::ELEVATED_SURFACE, $base::PRIMARY, $base::PRIMARY),
                    hovered: state(
                        $base::HOVER_SURFACE,
                        $base::PRIMARY_HOVER,
                        $base::PRIMARY_HOVER,
                    ),
                    pressed: state(
                        $base::PRESSED_SURFACE,
                        $base::PRIMARY_PRESSED,
                        $base::PRIMARY_PRESSED,
                    ),
                    disabled: state(
                        $base::SELECTED_SURFACE,
                        $base::DISABLED_BORDER,
                        $base::DISABLED_TEXT,
                    ),
                },
                focus_border: $base::FOCUS_BORDER,
            },
            bar_background: $base::SURFACE,
            panel_background: $base::ELEVATED_SURFACE,
            separator: $base::BORDER,
        };
    };
}

const fn state(background: Color, border: Color, foreground: Color) -> WidgetryTabStateColors {
    WidgetryTabStateColors {
        background,
        border,
        foreground,
    }
}

palette!(LIGHT, light);
palette!(DARK, dark);
palette!(PINK_DREAM, pink_dream);
palette!(KAMURI_VIOLET, kamuri_violet);

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn selected_headers_remain_distinct_when_enabled_and_disabled() {
        for palette in [LIGHT, DARK, PINK_DREAM, KAMURI_VIOLET] {
            assert_ne!(
                palette.header.selected.normal.background,
                palette.header.unselected.normal.background
            );
            assert_ne!(
                palette.header.selected.disabled.background,
                palette.header.unselected.disabled.background
            );
            assert_ne!(
                palette.header.selected.normal.foreground,
                palette.header.selected.disabled.foreground
            );
            assert_eq!(
                palette.header.selected.normal.background,
                palette.panel_background
            );
        }
    }
}
