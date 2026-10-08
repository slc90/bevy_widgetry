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
