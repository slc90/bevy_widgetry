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
            (PINK_DREAM.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.disabled.background, 0xF1E1E9FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(PINK_DREAM.normal.palette.len(), 4);
        assert_eq!(
            PINK_DREAM.normal.palette[0].to_srgba().to_u8_array(),
            0xB4437DFFu32.to_be_bytes()
        );
        assert_eq!(
            PINK_DREAM.normal.palette[1].to_srgba().to_u8_array(),
            0x7766A7FFu32.to_be_bytes()
        );
        assert_eq!(
            PINK_DREAM.normal.palette[2].to_srgba().to_u8_array(),
            0x237A80FFu32.to_be_bytes()
        );
        assert_eq!(
            PINK_DREAM.normal.palette[3].to_srgba().to_u8_array(),
            0x9D6C28FFu32.to_be_bytes()
        );
        assert_eq!(PINK_DREAM.disabled.palette.len(), 4);
        assert_eq!(
            PINK_DREAM.disabled.palette[0].to_srgba().to_u8_array(),
            0xB6A2AEFFu32.to_be_bytes()
        );
        assert_eq!(
            PINK_DREAM.disabled.palette[1].to_srgba().to_u8_array(),
            0xC6B7C0FFu32.to_be_bytes()
        );
        assert_eq!(
            PINK_DREAM.disabled.palette[2].to_srgba().to_u8_array(),
            0xD4CBD0FFu32.to_be_bytes()
        );
        assert_eq!(
            PINK_DREAM.disabled.palette[3].to_srgba().to_u8_array(),
            0xE4DEE1FFu32.to_be_bytes()
        );
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.normal.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.disabled.background, 0xE8E0F0FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(KAMURI_VIOLET.normal.palette.len(), 4);
        assert_eq!(
            KAMURI_VIOLET.normal.palette[0].to_srgba().to_u8_array(),
            0x6E4EA3FFu32.to_be_bytes()
        );
        assert_eq!(
            KAMURI_VIOLET.normal.palette[1].to_srgba().to_u8_array(),
            0xA95387FFu32.to_be_bytes()
        );
        assert_eq!(
            KAMURI_VIOLET.normal.palette[2].to_srgba().to_u8_array(),
            0x367584FFu32.to_be_bytes()
        );
        assert_eq!(
            KAMURI_VIOLET.normal.palette[3].to_srgba().to_u8_array(),
            0x99712FFFu32.to_be_bytes()
        );
        assert_eq!(KAMURI_VIOLET.disabled.palette.len(), 4);
        assert_eq!(
            KAMURI_VIOLET.disabled.palette[0].to_srgba().to_u8_array(),
            0xAEA4BDFFu32.to_be_bytes()
        );
        assert_eq!(
            KAMURI_VIOLET.disabled.palette[1].to_srgba().to_u8_array(),
            0xBEB6C9FFu32.to_be_bytes()
        );
        assert_eq!(
            KAMURI_VIOLET.disabled.palette[2].to_srgba().to_u8_array(),
            0xD0CAD9FFu32.to_be_bytes()
        );
        assert_eq!(
            KAMURI_VIOLET.disabled.palette[3].to_srgba().to_u8_array(),
            0xDFDAE7FFu32.to_be_bytes()
        );
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.normal.background, [255, 255, 255, 255]),
            (LIGHT.disabled.background, [245, 245, 245, 255]),
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
        assert_eq!(
            LIGHT.normal.palette,
            &[
                Color::srgb_u8(9, 88, 217),
                Color::srgb_u8(8, 151, 156),
                Color::srgb_u8(56, 158, 13),
                Color::srgb_u8(173, 78, 0)
            ]
        );
        assert_eq!(
            LIGHT.disabled.palette,
            &[
                Color::srgb_u8(140, 140, 140),
                Color::srgb_u8(166, 166, 166),
                Color::srgb_u8(191, 191, 191),
                Color::srgb_u8(217, 217, 217)
            ]
        );
    }
    #[test]
    fn dark_matches_reference() {
        let slots = [
            (DARK.normal.background, [20, 20, 20, 255]),
            (DARK.disabled.background, [31, 31, 31, 255]),
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
        assert_eq!(
            DARK.normal.palette,
            &[
                Color::srgb_u8(64, 150, 255),
                Color::srgb_u8(54, 207, 201),
                Color::srgb_u8(149, 222, 100),
                Color::srgb_u8(255, 192, 105)
            ]
        );
        assert_eq!(
            DARK.disabled.palette,
            &[
                Color::srgb_u8(89, 89, 89),
                Color::srgb_u8(115, 115, 115),
                Color::srgb_u8(140, 140, 140),
                Color::srgb_u8(166, 166, 166)
            ]
        );
    }
}
