//! 固定配色与 event 驱动的 theme 切换。

use bevy::{
    app::{App, Plugin},
    color::Color,
    ecs::{event::Event, resource::Resource},
};
use bevy_widgetry_log::widgetry_info;

/// 深色 UI 的完整 state 配色，供 ThemeMode::Dark 共享。
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

/// 浅色 UI 的完整 state 配色，供 ThemeMode::Light 共享。
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

/// Widget 各 interaction state 使用的固定配色，style 解析器负责确定 state 优先级。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTheme {
    /// window 和 title bar 共享的表面背景。
    pub window_background: Color,
    /// 自定义 window 的外 border。
    pub window_border: Color,
    /// title bar 与内容区之间的分隔线。
    pub title_bar_border: Color,
    /// 普通 state 下文本与 icon 使用的 foreground color。
    pub foreground: Color,
    /// disabled state 下文本与 icon 使用的 foreground color。
    pub foreground_disabled: Color,

    /// 没有 interaction state 时的 Widget background color。
    pub control_background: Color,
    /// pointer hover 时的 Widget background color。
    pub control_background_hovered: Color,
    /// pressed 期间的 Widget background color。
    pub control_background_pressed: Color,
    /// 获得 focus 或 Popup 打开时的 Widget background color。
    pub control_background_active: Color,
    /// disabled state 的 Widget background color，覆盖 interaction state 配色。
    pub control_background_disabled: Color,

    /// 普通 state 下的 Widget border color。
    pub control_border: Color,
    /// hover state 下的 Widget border color。
    pub control_border_hovered: Color,
    /// pressed state 下的 Widget border color。
    pub control_border_pressed: Color,
    /// 获得 focus 或打开 Popup 时的 border color。
    pub control_border_active: Color,
    /// disabled state 下的 border color。
    pub control_border_disabled: Color,

    /// Popup list 的基础背景，未选中 option 也使用此颜色。
    pub popup_background: Color,
    /// Popup list 容器的 border color。
    pub popup_border: Color,

    /// option hover 时的背景，覆盖其 selected 颜色。
    pub item_background_hovered: Color,
    /// option 被选中且未 hover 时的背景。
    pub item_background_selected: Color,

    /// TextField 有 focus 时的 selection 背景。
    pub text_selection: Color,
    /// TextField 失去 focus 后保留 selection 的背景。
    pub text_selection_unfocused: Color,
}

/// 更新 ThemeMode resource 后触发此 event，通知带 style 的 Widget 应用新配色。
///
/// 调用方必须先修改 resource，再触发与 resource 一致的模式；event 本身不修改 resource。
/// observer 立即应用颜色，descendant 文本的颜色传播在 PostUpdate 中完成。
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeChanged {
    /// 调用方已经写入 ThemeMode resource 的新模式。
    pub mode: ThemeMode,
}

/// 初始化 theme resource，保留应用预先设置的 theme 模式。
pub struct ThemePlugin;

/// 当前配色选择，默认深色；修改后需触发 ThemeChanged 才会刷新现有 style。
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    Light,

    #[default]
    Dark,
}

impl ThemeMode {
    /// 返回当前模式共享的静态配色，不分配也不修改 theme resource。
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

#[cfg(test)]
mod tests {
    use super::*;

    // 检查默认模式和两种显式模式，验证模式与静态 palette 的映射固定。
    #[test]
    fn modes_resolve_fixed_palettes() {
        assert_eq!(ThemeMode::default(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.colors(), &DARK_THEME);
        assert_eq!(ThemeMode::Light.colors(), &LIGHT_THEME);
    }

    // 应用先提供浅色 resource 再注册 plugin，验证 plugin 初始化不会覆盖消费者选择。
    #[test]
    fn plugin_preserves_initial_mode() {
        let mut app = App::new();
        app.insert_resource(ThemeMode::Light)
            .add_plugins(ThemePlugin);
        assert_eq!(*app.world().resource::<ThemeMode>(), ThemeMode::Light);
    }
}
