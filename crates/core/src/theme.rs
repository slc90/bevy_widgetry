use bevy::{
    app::{App, Plugin},
    color::Color,
    ecs::{event::Event, resource::Resource},
};
use bevy_widgetry_log::widgetry_info;

pub const DARK_THEME: ColorTheme = ColorTheme {
    window_background: Color::srgb_u8(37, 39, 43),
    window_border: Color::srgb_u8(73, 77, 85),
    title_bar_border: Color::srgb_u8(73, 77, 85),
    foreground: Color::srgb_u8(235, 237, 240),
    foreground_disabled: Color::srgb_u8(126, 132, 142),

    control_background: Color::srgb_u8(48, 50, 55),
    control_background_hovered: Color::srgb_u8(58, 61, 67),
    control_background_pressed: Color::srgb_u8(40, 43, 48),
    control_background_active: Color::srgb_u8(43, 56, 72),
    control_background_disabled: Color::srgb_u8(37, 39, 43),

    control_border: Color::srgb_u8(83, 87, 96),
    control_border_hovered: Color::srgb_u8(94, 153, 255),
    control_border_pressed: Color::srgb_u8(68, 134, 245),
    control_border_active: Color::srgb_u8(68, 134, 245),
    control_border_disabled: Color::srgb_u8(59, 62, 69),

    popup_background: Color::srgb_u8(37, 39, 43),
    popup_border: Color::srgb_u8(73, 77, 85),

    item_background_hovered: Color::srgb_u8(52, 61, 73),
    item_background_selected: Color::srgb_u8(42, 74, 115),

    text_selection: Color::srgb_u8(52, 92, 140),
    text_selection_unfocused: Color::srgb_u8(65, 70, 78),
};

pub const LIGHT_THEME: ColorTheme = ColorTheme {
    window_background: Color::srgb_u8(255, 255, 255),
    window_border: Color::srgb_u8(198, 203, 211),
    title_bar_border: Color::srgb_u8(198, 203, 211),
    foreground: Color::srgb_u8(35, 38, 43),
    foreground_disabled: Color::srgb_u8(142, 148, 158),

    control_background: Color::srgb_u8(247, 248, 250),
    control_background_hovered: Color::srgb_u8(236, 239, 244),
    control_background_pressed: Color::srgb_u8(222, 228, 236),
    control_background_active: Color::srgb_u8(230, 238, 248),
    control_background_disabled: Color::srgb_u8(238, 240, 243),

    control_border: Color::srgb_u8(183, 188, 197),
    control_border_hovered: Color::srgb_u8(85, 142, 235),
    control_border_pressed: Color::srgb_u8(58, 121, 218),
    control_border_active: Color::srgb_u8(58, 121, 218),
    control_border_disabled: Color::srgb_u8(207, 211, 218),

    popup_background: Color::srgb_u8(255, 255, 255),
    popup_border: Color::srgb_u8(198, 203, 211),

    item_background_hovered: Color::srgb_u8(238, 243, 249),
    item_background_selected: Color::srgb_u8(220, 234, 255),

    text_selection: Color::srgb_u8(190, 216, 255),
    text_selection_unfocused: Color::srgb_u8(220, 224, 230),
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTheme {
    pub window_background: Color,
    pub window_border: Color,
    pub title_bar_border: Color,
    pub foreground: Color,
    pub foreground_disabled: Color,

    pub control_background: Color,
    pub control_background_hovered: Color,
    pub control_background_pressed: Color,
    pub control_background_active: Color,
    pub control_background_disabled: Color,

    pub control_border: Color,
    pub control_border_hovered: Color,
    pub control_border_pressed: Color,
    pub control_border_active: Color,
    pub control_border_disabled: Color,

    pub popup_background: Color,
    pub popup_border: Color,

    pub item_background_hovered: Color,
    pub item_background_selected: Color,

    pub text_selection: Color,
    pub text_selection_unfocused: Color,
}

#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeChanged {
    pub mode: ThemeMode,
}

pub struct ThemePlugin;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    Light,

    #[default]
    Dark,
}

impl ThemeMode {
    pub const fn colors(self) -> &'static ColorTheme {
        match self {
            Self::Light => &LIGHT_THEME,
            Self::Dark => &DARK_THEME,
        }
    }
}

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ThemeMode>();
        widgetry_info!("ThemePlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn modes_resolve_fixed_palettes() {
        assert_eq!(ThemeMode::default(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.colors(), &DARK_THEME);
        assert_eq!(ThemeMode::Light.colors(), &LIGHT_THEME);
    }

    #[test]
    fn plugin_preserves_initial_mode() {
        let mut app = App::new();
        app.insert_resource(ThemeMode::Light)
            .add_plugins(ThemePlugin);
        assert_eq!(*app.world().resource::<ThemeMode>(), ThemeMode::Light);
    }
}
