use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
