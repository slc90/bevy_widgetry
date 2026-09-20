//! 按 Widget 组织的 Widgetry 公共入口。

pub mod button {
    pub use bevy_widgetry_button::*;
}

pub mod combo_box {
    pub use bevy_widgetry_combo_box::*;
}

/// 使用固定 direct child index 的标准 RadioGroup。
pub mod radio_group {
    pub use bevy_widgetry_radio_group::*;
}

pub mod style {
    pub use bevy_widgetry_core::{
        ColorTheme, DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin,
        WidgetryAppExt, WidgetryFocusPlugin,
    };
}

pub mod window {
    pub use bevy_widgetry_window::*;
}

/// 固定 button 组合的 non-blocking parent-window modal dialog。
pub mod message_box {
    pub use bevy_widgetry_message_box::*;
}

pub mod text_field {
    pub use bevy_widgetry_text_field::*;
}

/// 可组合到 title bar 等自定义内容中的 SVG icon。
pub mod icon {
    pub use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin, WidgetryIconProps};
}
