use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryWaveformStateColors {
    pub background: Color,
    pub palette: &'static [Color],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryWaveformColors {
    pub normal: WidgetryWaveformStateColors,
    pub disabled: WidgetryWaveformStateColors,
}

const LIGHT_NORMAL_PALETTE: &[Color] = &[
    Color::srgb_u8(9, 88, 217),
    Color::srgb_u8(8, 151, 156),
    Color::srgb_u8(56, 158, 13),
    Color::srgb_u8(173, 78, 0),
];
const LIGHT_DISABLED_PALETTE: &[Color] = &[
    Color::srgb_u8(140, 140, 140),
    Color::srgb_u8(166, 166, 166),
    Color::srgb_u8(191, 191, 191),
    Color::srgb_u8(217, 217, 217),
];
const DARK_NORMAL_PALETTE: &[Color] = &[
    Color::srgb_u8(64, 150, 255),
    Color::srgb_u8(54, 207, 201),
    Color::srgb_u8(149, 222, 100),
    Color::srgb_u8(255, 192, 105),
];
const DARK_DISABLED_PALETTE: &[Color] = &[
    Color::srgb_u8(89, 89, 89),
    Color::srgb_u8(115, 115, 115),
    Color::srgb_u8(140, 140, 140),
    Color::srgb_u8(166, 166, 166),
];
pub const LIGHT: WidgetryWaveformColors = WidgetryWaveformColors {
    normal: WidgetryWaveformStateColors {
        background: light::SURFACE,
        palette: LIGHT_NORMAL_PALETTE,
    },
    disabled: WidgetryWaveformStateColors {
        background: light::DISABLED_SURFACE,
        palette: LIGHT_DISABLED_PALETTE,
    },
};

pub const DARK: WidgetryWaveformColors = WidgetryWaveformColors {
    normal: WidgetryWaveformStateColors {
        background: dark::SURFACE,
        palette: DARK_NORMAL_PALETTE,
    },
    disabled: WidgetryWaveformStateColors {
        background: dark::DISABLED_SURFACE,
        palette: DARK_DISABLED_PALETTE,
    },
};

const PINK_DREAM_NORMAL_PALETTE: &[Color] = &[
    Color::srgb_u8(180, 67, 125),
    Color::srgb_u8(119, 102, 167),
    Color::srgb_u8(35, 122, 128),
    Color::srgb_u8(157, 108, 40),
];

const PINK_DREAM_DISABLED_PALETTE: &[Color] = &[
    Color::srgb_u8(182, 162, 174),
    Color::srgb_u8(198, 183, 192),
    Color::srgb_u8(212, 203, 208),
    Color::srgb_u8(228, 222, 225),
];

const KAMURI_VIOLET_NORMAL_PALETTE: &[Color] = &[
    Color::srgb_u8(110, 78, 163),
    Color::srgb_u8(169, 83, 135),
    Color::srgb_u8(54, 117, 132),
    Color::srgb_u8(153, 113, 47),
];

const KAMURI_VIOLET_DISABLED_PALETTE: &[Color] = &[
    Color::srgb_u8(174, 164, 189),
    Color::srgb_u8(190, 182, 201),
    Color::srgb_u8(208, 202, 217),
    Color::srgb_u8(223, 218, 231),
];

pub const PINK_DREAM: WidgetryWaveformColors = WidgetryWaveformColors {
    normal: WidgetryWaveformStateColors {
        background: pink_dream::SURFACE,
        palette: PINK_DREAM_NORMAL_PALETTE,
    },
    disabled: WidgetryWaveformStateColors {
        background: pink_dream::DISABLED_SURFACE,
        palette: PINK_DREAM_DISABLED_PALETTE,
    },
};

pub const KAMURI_VIOLET: WidgetryWaveformColors = WidgetryWaveformColors {
    normal: WidgetryWaveformStateColors {
        background: kamuri_violet::SURFACE,
        palette: KAMURI_VIOLET_NORMAL_PALETTE,
    },
    disabled: WidgetryWaveformStateColors {
        background: kamuri_violet::DISABLED_SURFACE,
        palette: KAMURI_VIOLET_DISABLED_PALETTE,
    },
};

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn light_colors_are_finite() {
        let colors = [LIGHT.normal.background, LIGHT.disabled.background];
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
        let colors = [DARK.normal.background, DARK.disabled.background];
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
