//! 固定配色与事件驱动的主题切换。

use bevy::{
    app::{App, Plugin},
    color::Color,
    ecs::{event::Event, resource::Resource},
};

/// 深色界面的完整状态配色，供 ThemeMode::Dark 共享。
pub const DARK_THEME: ColorTheme = ColorTheme {
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

/// 浅色界面的完整状态配色，供 ThemeMode::Light 共享。
pub const LIGHT_THEME: ColorTheme = ColorTheme {
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

/// 控件各交互状态使用的固定配色，样式解析器负责确定状态优先级。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTheme {
    /// 普通状态下文本与图标使用的前景色。
    pub foreground: Color,
    /// 禁用状态下文本与图标使用的前景色。
    pub foreground_disabled: Color,

    /// 没有交互状态时的控件背景色。
    pub control_background: Color,
    /// 指针悬停时的控件背景色。
    pub control_background_hovered: Color,
    /// 按压期间的控件背景色。
    pub control_background_pressed: Color,
    /// 获得焦点或弹层打开时的控件背景色。
    pub control_background_active: Color,
    /// 禁用状态的控件背景色，覆盖交互状态配色。
    pub control_background_disabled: Color,

    /// 普通状态下的控件边框色。
    pub control_border: Color,
    /// 悬停状态下的控件边框色。
    pub control_border_hovered: Color,
    /// 按压状态下的控件边框色。
    pub control_border_pressed: Color,
    /// 获得焦点或打开弹层时的边框色。
    pub control_border_active: Color,
    /// 禁用状态下的边框色。
    pub control_border_disabled: Color,

    /// 弹出列表的基础背景，未选中选项也使用此颜色。
    pub popup_background: Color,
    /// 弹出列表容器的边框色。
    pub popup_border: Color,

    /// 选项悬停时的背景，覆盖其选中色。
    pub item_background_hovered: Color,
    /// 选项被选中且未悬停时的背景。
    pub item_background_selected: Color,

    /// 输入框有焦点时的选区背景。
    pub text_selection: Color,
    /// 输入框失焦后保留选区的背景。
    pub text_selection_unfocused: Color,
}

/// 更新 `ThemeMode` 资源后触发此事件，通知样式控件应用新配色。
///
/// 调用方必须先修改资源，再触发与资源一致的模式；事件本身不修改资源。
/// 观察者立即应用颜色，后代文本的颜色传播在 PostUpdate 中完成。
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeChanged {
    /// 调用方已经写入 ThemeMode 资源的新模式。
    pub mode: ThemeMode,
}

/// 初始化主题资源，保留应用预先设置的主题模式。
pub struct ThemePlugin;

/// 当前配色选择，默认深色；修改后需触发 ThemeChanged 才会刷新现有样式。
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    Light,

    #[default]
    Dark,
}

impl ThemeMode {
    /// 返回当前模式共享的静态配色，不分配也不修改主题资源。
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 检查默认模式和两种显式模式，验证模式与静态调色板的映射固定。
    #[test]
    fn modes_resolve_fixed_palettes() {
        assert_eq!(ThemeMode::default(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.colors(), &DARK_THEME);
        assert_eq!(ThemeMode::Light.colors(), &LIGHT_THEME);
    }

    // 应用先提供浅色资源再注册插件，验证插件初始化不会覆盖消费者选择。
    #[test]
    fn plugin_preserves_initial_mode() {
        let mut app = App::new();
        app.insert_resource(ThemeMode::Light)
            .add_plugins(ThemePlugin);
        assert_eq!(*app.world().resource::<ThemeMode>(), ThemeMode::Light);
    }
}
