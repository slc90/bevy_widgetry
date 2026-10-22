use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySegmentedContainerStateColors {
    pub background: Color,
    pub border: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySegmentedContainerColors {
    pub normal: WidgetrySegmentedContainerStateColors,
    pub focused: WidgetrySegmentedContainerStateColors,
    pub disabled: WidgetrySegmentedContainerStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySegmentedOptionStateColors {
    pub background: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySegmentedOptionInteractionColors {
    pub normal: WidgetrySegmentedOptionStateColors,
    pub hovered: WidgetrySegmentedOptionStateColors,
    pub disabled: WidgetrySegmentedOptionStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySegmentedOptionColors {
    pub unchecked: WidgetrySegmentedOptionInteractionColors,
    pub checked: WidgetrySegmentedOptionInteractionColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySegmentedColors {
    pub container: WidgetrySegmentedContainerColors,
    pub option: WidgetrySegmentedOptionColors,
}

const fn colors(
    container: Color,
    selection: Color,
    hover: Color,
    disabled_surface: Color,
    disabled_selection: Color,
    foreground: Color,
    disabled_foreground: Color,
    focus: Color,
) -> WidgetrySegmentedColors {
    WidgetrySegmentedColors {
        container: WidgetrySegmentedContainerColors {
            normal: WidgetrySegmentedContainerStateColors {
                background: container,
                border: Color::NONE,
            },
            focused: WidgetrySegmentedContainerStateColors {
                background: container,
                border: focus,
            },
            disabled: WidgetrySegmentedContainerStateColors {
                background: disabled_surface,
                border: Color::NONE,
            },
        },
        option: WidgetrySegmentedOptionColors {
            unchecked: WidgetrySegmentedOptionInteractionColors {
                normal: WidgetrySegmentedOptionStateColors {
                    background: Color::NONE,
                    foreground,
                },
                hovered: WidgetrySegmentedOptionStateColors {
                    background: hover,
                    foreground,
                },
                disabled: WidgetrySegmentedOptionStateColors {
                    background: Color::NONE,
                    foreground: disabled_foreground,
                },
            },
            checked: WidgetrySegmentedOptionInteractionColors {
                normal: WidgetrySegmentedOptionStateColors {
                    background: selection,
                    foreground,
                },
                hovered: WidgetrySegmentedOptionStateColors {
                    background: selection,
                    foreground,
                },
                disabled: WidgetrySegmentedOptionStateColors {
                    background: disabled_selection,
                    foreground: disabled_foreground,
                },
            },
        },
    }
}

pub const LIGHT: WidgetrySegmentedColors = colors(
    light::HOVER_SURFACE,
    light::ELEVATED_SURFACE,
    light::PRESSED_SURFACE,
    light::DISABLED_SURFACE,
    light::PRESSED_SURFACE,
    light::TEXT,
    light::DISABLED_TEXT,
    light::FOCUS_BORDER,
);
pub const DARK: WidgetrySegmentedColors = colors(
    dark::ELEVATED_SURFACE,
    dark::BORDER,
    dark::HOVER_SURFACE,
    dark::DISABLED_SURFACE,
    dark::HOVER_SURFACE,
    dark::TEXT,
    dark::DISABLED_TEXT,
    dark::FOCUS_BORDER,
);
pub const PINK_DREAM: WidgetrySegmentedColors = colors(
    pink_dream::HOVER_SURFACE,
    pink_dream::ELEVATED_SURFACE,
    pink_dream::PRESSED_SURFACE,
    pink_dream::DISABLED_SURFACE,
    pink_dream::PRESSED_SURFACE,
    pink_dream::TEXT,
    pink_dream::DISABLED_TEXT,
    pink_dream::FOCUS_BORDER,
);
pub const KAMURI_VIOLET: WidgetrySegmentedColors = colors(
    kamuri_violet::HOVER_SURFACE,
    kamuri_violet::ELEVATED_SURFACE,
    kamuri_violet::PRESSED_SURFACE,
    kamuri_violet::DISABLED_SURFACE,
    kamuri_violet::PRESSED_SURFACE,
    kamuri_violet::TEXT,
    kamuri_violet::DISABLED_TEXT,
    kamuri_violet::FOCUS_BORDER,
);

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn every_palette_keeps_checked_blocks_distinct_when_enabled_or_disabled() {
        for palette in [LIGHT, DARK, PINK_DREAM, KAMURI_VIOLET] {
            assert_ne!(
                palette.option.checked.normal.background,
                palette.container.normal.background
            );
            assert_ne!(
                palette.option.checked.normal.background,
                palette.option.unchecked.normal.background
            );
            assert_ne!(
                palette.option.checked.disabled.background,
                palette.option.unchecked.disabled.background
            );
            assert_ne!(
                palette.container.focused.border,
                palette.container.normal.border
            );
        }
    }
}
