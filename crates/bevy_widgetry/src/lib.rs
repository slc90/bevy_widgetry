//! 按控件组织的 Widgetry 公共入口。

pub mod button {
    pub use bevy_widgetry_button::*;
}

pub mod combo_box {
    pub use bevy_widgetry_combo_box::*;
}

pub mod style {
    pub use bevy_widgetry_core::{
        ColorTheme, DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin,
        WidgetryAppExt,
    };
}

pub mod window {
    pub use bevy_widgetry_window::*;
}

pub mod text_field {
    pub use bevy_widgetry_text_field::*;
}

/// 可组合到标题栏等自定义内容中的 SVG 图标。
pub mod icon {
    pub use bevy_widgetry_core::icon::{Icon, IconPlugin};
}
