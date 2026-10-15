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
    pub const SECONDARY_TEXT: Color = Color::srgb_u8(106, 89, 119);
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
