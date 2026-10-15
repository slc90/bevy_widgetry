use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioGroupContainerStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioGroupContainerColors {
    pub normal: WidgetryRadioGroupContainerStateColors,
    pub focused: WidgetryRadioGroupContainerStateColors,
    pub disabled: WidgetryRadioGroupContainerStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioOptionStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
    pub dot: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioOptionInteractionColors {
    pub normal: WidgetryRadioOptionStateColors,
    pub hovered: WidgetryRadioOptionStateColors,
    pub disabled: WidgetryRadioOptionStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioOptionColors {
    pub unchecked: WidgetryRadioOptionInteractionColors,
    pub checked: WidgetryRadioOptionInteractionColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryRadioGroupColors {
    pub container: WidgetryRadioGroupContainerColors,
    pub option: WidgetryRadioOptionColors,
}

pub const LIGHT: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
    container: WidgetryRadioGroupContainerColors {
        normal: WidgetryRadioGroupContainerStateColors {
            background: light::SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
        },
        focused: WidgetryRadioGroupContainerStateColors {
            background: light::SURFACE,
            border: light::FOCUS_BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryRadioGroupContainerStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
    },
    option: WidgetryRadioOptionColors {
        unchecked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: light::SURFACE,
                border: light::BORDER,
                foreground: light::TEXT,
                dot: light::TRANSPARENT,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: light::SURFACE,
                border: light::PRIMARY_HOVER,
                foreground: light::TEXT,
                dot: light::TRANSPARENT,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: light::DISABLED_SURFACE,
                border: light::DISABLED_BORDER,
                foreground: light::DISABLED_TEXT,
                dot: light::TRANSPARENT,
            },
        },
        checked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: light::SURFACE,
                border: light::PRIMARY,
                foreground: light::TEXT,
                dot: light::PRIMARY,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: light::SURFACE,
                border: light::PRIMARY_HOVER,
                foreground: light::TEXT,
                dot: light::PRIMARY_HOVER,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: light::DISABLED_SURFACE,
                border: light::DISABLED_BORDER,
                foreground: light::DISABLED_TEXT,
                dot: light::DISABLED_TEXT,
            },
        },
    },
};

pub const DARK: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
    container: WidgetryRadioGroupContainerColors {
        normal: WidgetryRadioGroupContainerStateColors {
            background: dark::SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
        },
        focused: WidgetryRadioGroupContainerStateColors {
            background: dark::SURFACE,
            border: dark::FOCUS_BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryRadioGroupContainerStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
    },
    option: WidgetryRadioOptionColors {
        unchecked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: dark::SURFACE,
                border: dark::BORDER,
                foreground: dark::TEXT,
                dot: dark::TRANSPARENT,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: dark::SURFACE,
                border: dark::PRIMARY_HOVER,
                foreground: dark::TEXT,
                dot: dark::TRANSPARENT,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: dark::DISABLED_SURFACE,
                border: dark::DISABLED_BORDER,
                foreground: dark::DISABLED_TEXT,
                dot: dark::TRANSPARENT,
            },
        },
        checked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: dark::SURFACE,
                border: dark::PRIMARY,
                foreground: dark::TEXT,
                dot: dark::PRIMARY,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: dark::SURFACE,
                border: dark::PRIMARY_HOVER,
                foreground: dark::TEXT,
                dot: dark::PRIMARY_HOVER,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: dark::DISABLED_SURFACE,
                border: dark::DISABLED_BORDER,
                foreground: dark::DISABLED_TEXT,
                dot: dark::DISABLED_TEXT,
            },
        },
    },
};

pub const PINK_DREAM: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
    container: WidgetryRadioGroupContainerColors {
        normal: WidgetryRadioGroupContainerStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
        },
        focused: WidgetryRadioGroupContainerStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::FOCUS_BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryRadioGroupContainerStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    option: WidgetryRadioOptionColors {
        unchecked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: pink_dream::SURFACE,
                border: pink_dream::BORDER,
                foreground: pink_dream::TEXT,
                dot: pink_dream::TRANSPARENT,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: pink_dream::SURFACE,
                border: pink_dream::PRIMARY_HOVER,
                foreground: pink_dream::TEXT,
                dot: pink_dream::TRANSPARENT,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: pink_dream::DISABLED_SURFACE,
                border: pink_dream::DISABLED_BORDER,
                foreground: pink_dream::DISABLED_TEXT,
                dot: pink_dream::TRANSPARENT,
            },
        },
        checked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: pink_dream::SURFACE,
                border: pink_dream::PRIMARY,
                foreground: pink_dream::TEXT,
                dot: pink_dream::PRIMARY,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: pink_dream::SURFACE,
                border: pink_dream::PRIMARY_HOVER,
                foreground: pink_dream::TEXT,
                dot: pink_dream::PRIMARY_HOVER,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: pink_dream::DISABLED_SURFACE,
                border: pink_dream::DISABLED_BORDER,
                foreground: pink_dream::DISABLED_TEXT,
                dot: pink_dream::DISABLED_TEXT,
            },
        },
    },
};

pub const KAMURI_VIOLET: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
    container: WidgetryRadioGroupContainerColors {
        normal: WidgetryRadioGroupContainerStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
        },
        focused: WidgetryRadioGroupContainerStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::FOCUS_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryRadioGroupContainerStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    option: WidgetryRadioOptionColors {
        unchecked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
                dot: kamuri_violet::TRANSPARENT,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                dot: kamuri_violet::TRANSPARENT,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                dot: kamuri_violet::TRANSPARENT,
            },
        },
        checked: WidgetryRadioOptionInteractionColors {
            normal: WidgetryRadioOptionStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::PRIMARY,
                foreground: kamuri_violet::TEXT,
                dot: kamuri_violet::PRIMARY,
            },
            hovered: WidgetryRadioOptionStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                dot: kamuri_violet::PRIMARY_HOVER,
            },
            disabled: WidgetryRadioOptionStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                dot: kamuri_violet::DISABLED_TEXT,
            },
        },
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
            LIGHT.option.unchecked.normal.background,
            LIGHT.option.unchecked.normal.border,
            LIGHT.option.unchecked.normal.foreground,
            LIGHT.option.unchecked.normal.dot,
            LIGHT.option.unchecked.hovered.background,
            LIGHT.option.unchecked.hovered.border,
            LIGHT.option.unchecked.hovered.foreground,
            LIGHT.option.unchecked.hovered.dot,
            LIGHT.option.unchecked.disabled.background,
            LIGHT.option.unchecked.disabled.border,
            LIGHT.option.unchecked.disabled.foreground,
            LIGHT.option.unchecked.disabled.dot,
            LIGHT.option.checked.normal.background,
            LIGHT.option.checked.normal.border,
            LIGHT.option.checked.normal.foreground,
            LIGHT.option.checked.normal.dot,
            LIGHT.option.checked.hovered.background,
            LIGHT.option.checked.hovered.border,
            LIGHT.option.checked.hovered.foreground,
            LIGHT.option.checked.hovered.dot,
            LIGHT.option.checked.disabled.background,
            LIGHT.option.checked.disabled.border,
            LIGHT.option.checked.disabled.foreground,
            LIGHT.option.checked.disabled.dot,
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
            DARK.option.unchecked.normal.background,
            DARK.option.unchecked.normal.border,
            DARK.option.unchecked.normal.foreground,
            DARK.option.unchecked.normal.dot,
            DARK.option.unchecked.hovered.background,
            DARK.option.unchecked.hovered.border,
            DARK.option.unchecked.hovered.foreground,
            DARK.option.unchecked.hovered.dot,
            DARK.option.unchecked.disabled.background,
            DARK.option.unchecked.disabled.border,
            DARK.option.unchecked.disabled.foreground,
            DARK.option.unchecked.disabled.dot,
            DARK.option.checked.normal.background,
            DARK.option.checked.normal.border,
            DARK.option.checked.normal.foreground,
            DARK.option.checked.normal.dot,
            DARK.option.checked.hovered.background,
            DARK.option.checked.hovered.border,
            DARK.option.checked.hovered.foreground,
            DARK.option.checked.hovered.dot,
            DARK.option.checked.disabled.background,
            DARK.option.checked.disabled.border,
            DARK.option.checked.disabled.foreground,
            DARK.option.checked.disabled.dot,
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
