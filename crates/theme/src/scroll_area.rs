use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryScrollTrackStateColors {
    pub background: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryScrollTrackColors {
    pub normal: WidgetryScrollTrackStateColors,
    pub disabled: WidgetryScrollTrackStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryScrollThumbStateColors {
    pub background: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryScrollThumbColors {
    pub normal: WidgetryScrollThumbStateColors,
    pub hovered: WidgetryScrollThumbStateColors,
    pub dragged: WidgetryScrollThumbStateColors,
    pub disabled: WidgetryScrollThumbStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryScrollAxisColors {
    pub track: WidgetryScrollTrackColors,
    pub thumb: WidgetryScrollThumbColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryScrollAreaColors {
    pub horizontal: WidgetryScrollAxisColors,
    pub vertical: WidgetryScrollAxisColors,
}

pub const LIGHT: WidgetryScrollAreaColors = WidgetryScrollAreaColors {
    horizontal: WidgetryScrollAxisColors {
        track: WidgetryScrollTrackColors {
            normal: WidgetryScrollTrackStateColors {
                background: light::TRANSPARENT,
            },
            disabled: WidgetryScrollTrackStateColors {
                background: light::TRANSPARENT,
            },
        },
        thumb: WidgetryScrollThumbColors {
            normal: WidgetryScrollThumbStateColors {
                background: light::BORDER,
            },
            hovered: WidgetryScrollThumbStateColors {
                background: light::PRIMARY_HOVER,
            },
            dragged: WidgetryScrollThumbStateColors {
                background: light::PRIMARY_PRESSED,
            },
            disabled: WidgetryScrollThumbStateColors {
                background: light::DISABLED_BORDER,
            },
        },
    },
    vertical: WidgetryScrollAxisColors {
        track: WidgetryScrollTrackColors {
            normal: WidgetryScrollTrackStateColors {
                background: light::TRANSPARENT,
            },
            disabled: WidgetryScrollTrackStateColors {
                background: light::TRANSPARENT,
            },
        },
        thumb: WidgetryScrollThumbColors {
            normal: WidgetryScrollThumbStateColors {
                background: light::BORDER,
            },
            hovered: WidgetryScrollThumbStateColors {
                background: light::PRIMARY_HOVER,
            },
            dragged: WidgetryScrollThumbStateColors {
                background: light::PRIMARY_PRESSED,
            },
            disabled: WidgetryScrollThumbStateColors {
                background: light::DISABLED_BORDER,
            },
        },
    },
};

pub const DARK: WidgetryScrollAreaColors = WidgetryScrollAreaColors {
    horizontal: WidgetryScrollAxisColors {
        track: WidgetryScrollTrackColors {
            normal: WidgetryScrollTrackStateColors {
                background: dark::TRANSPARENT,
            },
            disabled: WidgetryScrollTrackStateColors {
                background: dark::TRANSPARENT,
            },
        },
        thumb: WidgetryScrollThumbColors {
            normal: WidgetryScrollThumbStateColors {
                background: dark::BORDER,
            },
            hovered: WidgetryScrollThumbStateColors {
                background: dark::PRIMARY_HOVER,
            },
            dragged: WidgetryScrollThumbStateColors {
                background: dark::PRIMARY_PRESSED,
            },
            disabled: WidgetryScrollThumbStateColors {
                background: dark::DISABLED_BORDER,
            },
        },
    },
    vertical: WidgetryScrollAxisColors {
        track: WidgetryScrollTrackColors {
            normal: WidgetryScrollTrackStateColors {
                background: dark::TRANSPARENT,
            },
            disabled: WidgetryScrollTrackStateColors {
                background: dark::TRANSPARENT,
            },
        },
        thumb: WidgetryScrollThumbColors {
            normal: WidgetryScrollThumbStateColors {
                background: dark::BORDER,
            },
            hovered: WidgetryScrollThumbStateColors {
                background: dark::PRIMARY_HOVER,
            },
            dragged: WidgetryScrollThumbStateColors {
                background: dark::PRIMARY_PRESSED,
            },
            disabled: WidgetryScrollThumbStateColors {
                background: dark::DISABLED_BORDER,
            },
        },
    },
};

pub const PINK_DREAM: WidgetryScrollAreaColors = WidgetryScrollAreaColors {
    horizontal: WidgetryScrollAxisColors {
        track: WidgetryScrollTrackColors {
            normal: WidgetryScrollTrackStateColors {
                background: pink_dream::TRANSPARENT,
            },
            disabled: WidgetryScrollTrackStateColors {
                background: pink_dream::TRANSPARENT,
            },
        },
        thumb: WidgetryScrollThumbColors {
            normal: WidgetryScrollThumbStateColors {
                background: pink_dream::BORDER,
            },
            hovered: WidgetryScrollThumbStateColors {
                background: pink_dream::PRIMARY_HOVER,
            },
            dragged: WidgetryScrollThumbStateColors {
                background: pink_dream::PRIMARY_PRESSED,
            },
            disabled: WidgetryScrollThumbStateColors {
                background: pink_dream::DISABLED_BORDER,
            },
        },
    },
    vertical: WidgetryScrollAxisColors {
        track: WidgetryScrollTrackColors {
            normal: WidgetryScrollTrackStateColors {
                background: pink_dream::TRANSPARENT,
            },
            disabled: WidgetryScrollTrackStateColors {
                background: pink_dream::TRANSPARENT,
            },
        },
        thumb: WidgetryScrollThumbColors {
            normal: WidgetryScrollThumbStateColors {
                background: pink_dream::BORDER,
            },
            hovered: WidgetryScrollThumbStateColors {
                background: pink_dream::PRIMARY_HOVER,
            },
            dragged: WidgetryScrollThumbStateColors {
                background: pink_dream::PRIMARY_PRESSED,
            },
            disabled: WidgetryScrollThumbStateColors {
                background: pink_dream::DISABLED_BORDER,
            },
        },
    },
};

pub const KAMURI_VIOLET: WidgetryScrollAreaColors = WidgetryScrollAreaColors {
    horizontal: WidgetryScrollAxisColors {
        track: WidgetryScrollTrackColors {
            normal: WidgetryScrollTrackStateColors {
                background: kamuri_violet::TRANSPARENT,
            },
            disabled: WidgetryScrollTrackStateColors {
                background: kamuri_violet::TRANSPARENT,
            },
        },
        thumb: WidgetryScrollThumbColors {
            normal: WidgetryScrollThumbStateColors {
                background: kamuri_violet::BORDER,
            },
            hovered: WidgetryScrollThumbStateColors {
                background: kamuri_violet::PRIMARY_HOVER,
            },
            dragged: WidgetryScrollThumbStateColors {
                background: kamuri_violet::PRIMARY_PRESSED,
            },
            disabled: WidgetryScrollThumbStateColors {
                background: kamuri_violet::DISABLED_BORDER,
            },
        },
    },
    vertical: WidgetryScrollAxisColors {
        track: WidgetryScrollTrackColors {
            normal: WidgetryScrollTrackStateColors {
                background: kamuri_violet::TRANSPARENT,
            },
            disabled: WidgetryScrollTrackStateColors {
                background: kamuri_violet::TRANSPARENT,
            },
        },
        thumb: WidgetryScrollThumbColors {
            normal: WidgetryScrollThumbStateColors {
                background: kamuri_violet::BORDER,
            },
            hovered: WidgetryScrollThumbStateColors {
                background: kamuri_violet::PRIMARY_HOVER,
            },
            dragged: WidgetryScrollThumbStateColors {
                background: kamuri_violet::PRIMARY_PRESSED,
            },
            disabled: WidgetryScrollThumbStateColors {
                background: kamuri_violet::DISABLED_BORDER,
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
            LIGHT.horizontal.track.normal.background,
            LIGHT.horizontal.track.disabled.background,
            LIGHT.horizontal.thumb.normal.background,
            LIGHT.horizontal.thumb.hovered.background,
            LIGHT.horizontal.thumb.dragged.background,
            LIGHT.horizontal.thumb.disabled.background,
            LIGHT.vertical.track.normal.background,
            LIGHT.vertical.track.disabled.background,
            LIGHT.vertical.thumb.normal.background,
            LIGHT.vertical.thumb.hovered.background,
            LIGHT.vertical.thumb.dragged.background,
            LIGHT.vertical.thumb.disabled.background,
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
            DARK.horizontal.track.normal.background,
            DARK.horizontal.track.disabled.background,
            DARK.horizontal.thumb.normal.background,
            DARK.horizontal.thumb.hovered.background,
            DARK.horizontal.thumb.dragged.background,
            DARK.horizontal.thumb.disabled.background,
            DARK.vertical.track.normal.background,
            DARK.vertical.track.disabled.background,
            DARK.vertical.thumb.normal.background,
            DARK.vertical.thumb.hovered.background,
            DARK.vertical.thumb.dragged.background,
            DARK.vertical.thumb.disabled.background,
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
