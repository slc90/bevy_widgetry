use bevy::color::Color;

pub(crate) mod light {
    use super::Color;
    pub const WINDOW_BACKGROUND: Color = Color::srgb_u8(255, 255, 255);
    pub const SURFACE: Color = Color::srgb_u8(255, 255, 255);
    pub const ELEVATED_SURFACE: Color = Color::srgb_u8(255, 255, 255);
    pub const HOVER_SURFACE: Color = Color::srgb_u8(245, 245, 245);
    pub const PRESSED_SURFACE: Color = Color::srgb_u8(235, 235, 235);
    pub const DISABLED_SURFACE: Color = Color::srgb_u8(245, 245, 245);
    pub const BORDER: Color = Color::srgb_u8(217, 217, 217);
    pub const SUBTLE_BORDER: Color = Color::srgb_u8(240, 240, 240);
    pub const DISABLED_BORDER: Color = Color::srgb_u8(217, 217, 217);
    pub const TEXT: Color = Color::srgb_u8(31, 31, 31);
    pub const SECONDARY_TEXT: Color = Color::srgb_u8(89, 89, 89);
    pub const TERTIARY_TEXT: Color = Color::srgb_u8(140, 140, 140);
    pub const DISABLED_TEXT: Color = Color::srgb_u8(191, 191, 191);
    pub const INVERSE_TEXT: Color = Color::srgb_u8(255, 255, 255);
    pub const PRIMARY: Color = Color::srgb_u8(22, 119, 255);
    pub const PRIMARY_HOVER: Color = Color::srgb_u8(64, 150, 255);
    pub const PRIMARY_PRESSED: Color = Color::srgb_u8(9, 88, 217);
    pub const FOCUS_BORDER: Color = Color::srgb_u8(22, 119, 255);
    pub const SELECTED_SURFACE: Color = Color::srgb_u8(230, 244, 255);
    pub const SELECTION_BACKGROUND: Color = Color::srgb_u8(186, 224, 255);
    pub const UNFOCUSED_SELECTION_BACKGROUND: Color = Color::srgb_u8(217, 217, 217);
    pub const TOOLTIP_BACKGROUND: Color = Color::srgb_u8(38, 38, 38);
    pub const DANGER_HOVER: Color = Color::srgb_u8(217, 54, 62);
    pub const DANGER_PRESSED: Color = Color::srgb_u8(168, 7, 26);
    pub const TRANSPARENT: Color = Color::NONE;
    pub const IMAGE_TINT: Color = Color::srgb_u8(255, 255, 255);
}

pub(crate) mod dark {
    use super::Color;
    pub const WINDOW_BACKGROUND: Color = Color::srgb_u8(20, 20, 20);
    pub const SURFACE: Color = Color::srgb_u8(20, 20, 20);
    pub const ELEVATED_SURFACE: Color = Color::srgb_u8(31, 31, 31);
    pub const HOVER_SURFACE: Color = Color::srgb_u8(38, 38, 38);
    pub const PRESSED_SURFACE: Color = Color::srgb_u8(48, 48, 48);
    pub const DISABLED_SURFACE: Color = Color::srgb_u8(31, 31, 31);
    pub const BORDER: Color = Color::srgb_u8(66, 66, 66);
    pub const SUBTLE_BORDER: Color = Color::srgb_u8(48, 48, 48);
    pub const DISABLED_BORDER: Color = Color::srgb_u8(66, 66, 66);
    pub const TEXT: Color = Color::srgb_u8(220, 220, 220);
    pub const SECONDARY_TEXT: Color = Color::srgb_u8(173, 173, 173);
    pub const TERTIARY_TEXT: Color = Color::srgb_u8(126, 126, 126);
    pub const DISABLED_TEXT: Color = Color::srgb_u8(89, 89, 89);
    pub const INVERSE_TEXT: Color = Color::srgb_u8(255, 255, 255);
    pub const PRIMARY: Color = Color::srgb_u8(22, 104, 220);
    pub const PRIMARY_HOVER: Color = Color::srgb_u8(60, 137, 232);
    pub const PRIMARY_PRESSED: Color = Color::srgb_u8(21, 84, 173);
    pub const FOCUS_BORDER: Color = Color::srgb_u8(60, 137, 232);
    pub const SELECTED_SURFACE: Color = Color::srgb_u8(17, 26, 44);
    pub const SELECTION_BACKGROUND: Color = Color::srgb_u8(21, 57, 91);
    pub const UNFOCUSED_SELECTION_BACKGROUND: Color = Color::srgb_u8(48, 48, 48);
    pub const TOOLTIP_BACKGROUND: Color = Color::srgb_u8(66, 66, 66);
    pub const DANGER_HOVER: Color = Color::srgb_u8(217, 54, 62);
    pub const DANGER_PRESSED: Color = Color::srgb_u8(168, 7, 26);
    pub const TRANSPARENT: Color = Color::NONE;
    pub const IMAGE_TINT: Color = Color::srgb_u8(255, 255, 255);
}

pub(crate) mod pink_dream {
    use super::Color;
    pub const WINDOW_BACKGROUND: Color = Color::srgb_u8(246, 220, 233);
    pub const SURFACE: Color = Color::srgb_u8(252, 234, 243);
    pub const ELEVATED_SURFACE: Color = Color::srgb_u8(255, 243, 249);
    pub const HOVER_SURFACE: Color = Color::srgb_u8(249, 223, 237);
    pub const PRESSED_SURFACE: Color = Color::srgb_u8(242, 207, 226);
    pub const DISABLED_SURFACE: Color = Color::srgb_u8(241, 225, 233);
    pub const BORDER: Color = Color::srgb_u8(231, 204, 217);
    pub const SUBTLE_BORDER: Color = Color::srgb_u8(243, 227, 236);
    pub const DISABLED_BORDER: Color = Color::srgb_u8(229, 217, 224);
    pub const TEXT: Color = Color::srgb_u8(66, 43, 60);
    pub const SECONDARY_TEXT: Color = Color::srgb_u8(112, 85, 102);
    pub const TERTIARY_TEXT: Color = Color::srgb_u8(146, 122, 136);
    pub const DISABLED_TEXT: Color = Color::srgb_u8(172, 152, 163);
    pub const INVERSE_TEXT: Color = Color::srgb_u8(255, 255, 255);
    pub const PRIMARY: Color = Color::srgb_u8(180, 67, 125);
    pub const PRIMARY_HOVER: Color = Color::srgb_u8(205, 97, 150);
    pub const PRIMARY_PRESSED: Color = Color::srgb_u8(145, 48, 99);
    pub const FOCUS_BORDER: Color = Color::srgb_u8(180, 67, 125);
    pub const SELECTED_SURFACE: Color = Color::srgb_u8(245, 211, 230);
    pub const SELECTION_BACKGROUND: Color = Color::srgb_u8(240, 189, 216);
    pub const UNFOCUSED_SELECTION_BACKGROUND: Color = Color::srgb_u8(235, 222, 229);
    pub const TOOLTIP_BACKGROUND: Color = Color::srgb_u8(73, 53, 65);
    pub const DANGER_HOVER: Color = Color::srgb_u8(212, 73, 93);
    pub const DANGER_PRESSED: Color = Color::srgb_u8(170, 38, 62);
    pub const TRANSPARENT: Color = Color::NONE;
    pub const IMAGE_TINT: Color = Color::srgb_u8(255, 255, 255);
}

pub(crate) mod kamuri_violet {
    use super::Color;
    pub const WINDOW_BACKGROUND: Color = Color::srgb_u8(227, 217, 241);
    pub const SURFACE: Color = Color::srgb_u8(240, 232, 250);
    pub const ELEVATED_SURFACE: Color = Color::srgb_u8(247, 242, 253);
    pub const HOVER_SURFACE: Color = Color::srgb_u8(232, 221, 246);
    pub const PRESSED_SURFACE: Color = Color::srgb_u8(219, 203, 236);
    pub const DISABLED_SURFACE: Color = Color::srgb_u8(232, 224, 240);
    pub const BORDER: Color = Color::srgb_u8(216, 205, 228);
    pub const SUBTLE_BORDER: Color = Color::srgb_u8(238, 231, 245);
    pub const DISABLED_BORDER: Color = Color::srgb_u8(231, 223, 240);
    pub const TEXT: Color = Color::srgb_u8(61, 49, 74);
    pub const SECONDARY_TEXT: Color = Color::srgb_u8(114, 97, 127);
    pub const TERTIARY_TEXT: Color = Color::srgb_u8(154, 140, 166);
    pub const DISABLED_TEXT: Color = Color::srgb_u8(181, 169, 193);
    pub const INVERSE_TEXT: Color = Color::srgb_u8(255, 255, 255);
    pub const PRIMARY: Color = Color::srgb_u8(140, 107, 193);
    pub const PRIMARY_HOVER: Color = Color::srgb_u8(161, 129, 210);
    pub const PRIMARY_PRESSED: Color = Color::srgb_u8(110, 78, 163);
    pub const FOCUS_BORDER: Color = Color::srgb_u8(140, 107, 193);
    pub const SELECTED_SURFACE: Color = Color::srgb_u8(223, 210, 242);
    pub const SELECTION_BACKGROUND: Color = Color::srgb_u8(217, 202, 239);
    pub const UNFOCUSED_SELECTION_BACKGROUND: Color = Color::srgb_u8(232, 224, 240);
    pub const TOOLTIP_BACKGROUND: Color = Color::srgb_u8(74, 58, 95);
    pub const DANGER_HOVER: Color = Color::srgb_u8(155, 92, 134);
    pub const DANGER_PRESSED: Color = Color::srgb_u8(122, 69, 107);
    pub const TRANSPARENT: Color = Color::NONE;
    pub const IMAGE_TINT: Color = Color::srgb_u8(255, 255, 255);
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::color::ColorToPacked;

    #[test]
    fn new_tokens_match_approved_rgba() {
        let tokens = [
            (
                pink_dream::WINDOW_BACKGROUND,
                kamuri_violet::WINDOW_BACKGROUND,
                0xF6DCE9FFu32,
                0xE3D9F1FFu32,
            ),
            (
                pink_dream::SURFACE,
                kamuri_violet::SURFACE,
                0xFCEAF3FFu32,
                0xF0E8FAFFu32,
            ),
            (
                pink_dream::ELEVATED_SURFACE,
                kamuri_violet::ELEVATED_SURFACE,
                0xFFF3F9FFu32,
                0xF7F2FDFFu32,
            ),
            (
                pink_dream::HOVER_SURFACE,
                kamuri_violet::HOVER_SURFACE,
                0xF9DFEDFFu32,
                0xE8DDF6FFu32,
            ),
            (
                pink_dream::PRESSED_SURFACE,
                kamuri_violet::PRESSED_SURFACE,
                0xF2CFE2FFu32,
                0xDBCBECFFu32,
            ),
            (
                pink_dream::DISABLED_SURFACE,
                kamuri_violet::DISABLED_SURFACE,
                0xF1E1E9FFu32,
                0xE8E0F0FFu32,
            ),
            (
                pink_dream::BORDER,
                kamuri_violet::BORDER,
                0xE7CCD9FFu32,
                0xD8CDE4FFu32,
            ),
            (
                pink_dream::SUBTLE_BORDER,
                kamuri_violet::SUBTLE_BORDER,
                0xF3E3ECFFu32,
                0xEEE7F5FFu32,
            ),
            (
                pink_dream::DISABLED_BORDER,
                kamuri_violet::DISABLED_BORDER,
                0xE5D9E0FFu32,
                0xE7DFF0FFu32,
            ),
            (
                pink_dream::TEXT,
                kamuri_violet::TEXT,
                0x422B3CFFu32,
                0x3D314AFFu32,
            ),
            (
                pink_dream::SECONDARY_TEXT,
                kamuri_violet::SECONDARY_TEXT,
                0x705566FFu32,
                0x72617FFFu32,
            ),
            (
                pink_dream::TERTIARY_TEXT,
                kamuri_violet::TERTIARY_TEXT,
                0x927A88FFu32,
                0x9A8CA6FFu32,
            ),
            (
                pink_dream::DISABLED_TEXT,
                kamuri_violet::DISABLED_TEXT,
                0xAC98A3FFu32,
                0xB5A9C1FFu32,
            ),
            (
                pink_dream::INVERSE_TEXT,
                kamuri_violet::INVERSE_TEXT,
                0xFFFFFFFFu32,
                0xFFFFFFFFu32,
            ),
            (
                pink_dream::PRIMARY,
                kamuri_violet::PRIMARY,
                0xB4437DFFu32,
                0x8C6BC1FFu32,
            ),
            (
                pink_dream::PRIMARY_HOVER,
                kamuri_violet::PRIMARY_HOVER,
                0xCD6196FFu32,
                0xA181D2FFu32,
            ),
            (
                pink_dream::PRIMARY_PRESSED,
                kamuri_violet::PRIMARY_PRESSED,
                0x913063FFu32,
                0x6E4EA3FFu32,
            ),
            (
                pink_dream::FOCUS_BORDER,
                kamuri_violet::FOCUS_BORDER,
                0xB4437DFFu32,
                0x8C6BC1FFu32,
            ),
            (
                pink_dream::SELECTED_SURFACE,
                kamuri_violet::SELECTED_SURFACE,
                0xF5D3E6FFu32,
                0xDFD2F2FFu32,
            ),
            (
                pink_dream::SELECTION_BACKGROUND,
                kamuri_violet::SELECTION_BACKGROUND,
                0xF0BDD8FFu32,
                0xD9CAEFFFu32,
            ),
            (
                pink_dream::UNFOCUSED_SELECTION_BACKGROUND,
                kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                0xEBDEE5FFu32,
                0xE8E0F0FFu32,
            ),
            (
                pink_dream::TOOLTIP_BACKGROUND,
                kamuri_violet::TOOLTIP_BACKGROUND,
                0x493541FFu32,
                0x4A3A5FFFu32,
            ),
            (
                pink_dream::DANGER_HOVER,
                kamuri_violet::DANGER_HOVER,
                0xD4495DFFu32,
                0x9B5C86FFu32,
            ),
            (
                pink_dream::DANGER_PRESSED,
                kamuri_violet::DANGER_PRESSED,
                0xAA263EFFu32,
                0x7A456BFFu32,
            ),
            (
                pink_dream::TRANSPARENT,
                kamuri_violet::TRANSPARENT,
                0x00000000u32,
                0x00000000u32,
            ),
            (
                pink_dream::IMAGE_TINT,
                kamuri_violet::IMAGE_TINT,
                0xFFFFFFFFu32,
                0xFFFFFFFFu32,
            ),
        ];
        for (pink, violet, expected_pink, expected_violet) in tokens {
            assert_eq!(pink.to_srgba().to_u8_array(), expected_pink.to_be_bytes());
            assert_eq!(
                violet.to_srgba().to_u8_array(),
                expected_violet.to_be_bytes()
            );
        }
    }
}
