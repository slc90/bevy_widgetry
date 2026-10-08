use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
