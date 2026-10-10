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
            (PINK_DREAM.horizontal.track.normal.background, 0x00000000u32),
            (
                PINK_DREAM.horizontal.track.disabled.background,
                0x00000000u32,
            ),
            (PINK_DREAM.horizontal.thumb.normal.background, 0xE7CCD9FFu32),
            (
                PINK_DREAM.horizontal.thumb.hovered.background,
                0xCD6196FFu32,
            ),
            (
                PINK_DREAM.horizontal.thumb.dragged.background,
                0x913063FFu32,
            ),
            (
                PINK_DREAM.horizontal.thumb.disabled.background,
                0xE5D9E0FFu32,
            ),
            (PINK_DREAM.vertical.track.normal.background, 0x00000000u32),
            (PINK_DREAM.vertical.track.disabled.background, 0x00000000u32),
            (PINK_DREAM.vertical.thumb.normal.background, 0xE7CCD9FFu32),
            (PINK_DREAM.vertical.thumb.hovered.background, 0xCD6196FFu32),
            (PINK_DREAM.vertical.thumb.dragged.background, 0x913063FFu32),
            (PINK_DREAM.vertical.thumb.disabled.background, 0xE5D9E0FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (
                KAMURI_VIOLET.horizontal.track.normal.background,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.horizontal.track.disabled.background,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.horizontal.thumb.normal.background,
                0xD8CDE4FFu32,
            ),
            (
                KAMURI_VIOLET.horizontal.thumb.hovered.background,
                0xA181D2FFu32,
            ),
            (
                KAMURI_VIOLET.horizontal.thumb.dragged.background,
                0x6E4EA3FFu32,
            ),
            (
                KAMURI_VIOLET.horizontal.thumb.disabled.background,
                0xE7DFF0FFu32,
            ),
            (
                KAMURI_VIOLET.vertical.track.normal.background,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.vertical.track.disabled.background,
                0x00000000u32,
            ),
            (
                KAMURI_VIOLET.vertical.thumb.normal.background,
                0xD8CDE4FFu32,
            ),
            (
                KAMURI_VIOLET.vertical.thumb.hovered.background,
                0xA181D2FFu32,
            ),
            (
                KAMURI_VIOLET.vertical.thumb.dragged.background,
                0x6E4EA3FFu32,
            ),
            (
                KAMURI_VIOLET.vertical.thumb.disabled.background,
                0xE7DFF0FFu32,
            ),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.horizontal.track.normal.background, [0, 0, 0, 0]),
            (LIGHT.horizontal.track.disabled.background, [0, 0, 0, 0]),
            (
                LIGHT.horizontal.thumb.normal.background,
                [217, 217, 217, 255],
            ),
            (
                LIGHT.horizontal.thumb.hovered.background,
                [64, 150, 255, 255],
            ),
            (LIGHT.horizontal.thumb.dragged.background, [9, 88, 217, 255]),
            (
                LIGHT.horizontal.thumb.disabled.background,
                [217, 217, 217, 255],
            ),
            (LIGHT.vertical.track.normal.background, [0, 0, 0, 0]),
            (LIGHT.vertical.track.disabled.background, [0, 0, 0, 0]),
            (LIGHT.vertical.thumb.normal.background, [217, 217, 217, 255]),
            (LIGHT.vertical.thumb.hovered.background, [64, 150, 255, 255]),
            (LIGHT.vertical.thumb.dragged.background, [9, 88, 217, 255]),
            (
                LIGHT.vertical.thumb.disabled.background,
                [217, 217, 217, 255],
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
    }
    #[test]
    fn dark_matches_reference() {
        let slots = [
            (DARK.horizontal.track.normal.background, [0, 0, 0, 0]),
            (DARK.horizontal.track.disabled.background, [0, 0, 0, 0]),
            (DARK.horizontal.thumb.normal.background, [66, 66, 66, 255]),
            (
                DARK.horizontal.thumb.hovered.background,
                [60, 137, 232, 255],
            ),
            (DARK.horizontal.thumb.dragged.background, [21, 84, 173, 255]),
            (DARK.horizontal.thumb.disabled.background, [66, 66, 66, 255]),
            (DARK.vertical.track.normal.background, [0, 0, 0, 0]),
            (DARK.vertical.track.disabled.background, [0, 0, 0, 0]),
            (DARK.vertical.thumb.normal.background, [66, 66, 66, 255]),
            (DARK.vertical.thumb.hovered.background, [60, 137, 232, 255]),
            (DARK.vertical.thumb.dragged.background, [21, 84, 173, 255]),
            (DARK.vertical.thumb.disabled.background, [66, 66, 66, 255]),
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
