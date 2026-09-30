//! 按 Widget 组织的 Widgetry 公共入口。

pub mod button {
    pub use bevy_widgetry_button::*;
}

/// 消费 stable model 的泛型 ComboBox，由 Button 与 ListView 组合，负责 Field projection 与 Popup lifecycle。
pub mod combo_box {
    pub use bevy_widgetry_combo_box::*;
}

/// 使用固定 direct child index 的标准 RadioGroup。
pub mod radio_group {
    pub use bevy_widgetry_radio_group::*;
}

/// 使用官方 ScrollArea 与 Scrollbar 的可组合滚动容器。
pub mod scroll_area {
    pub use bevy_widgetry_scroll_area::*;
}

/// 使用 model-local stable identity 的泛型 ListView，提供 model、renderer、virtualization 与 selection authority。
pub mod list_view {
    pub use bevy_widgetry_list_view::*;
}

pub mod check_box {
    pub use bevy_widgetry_check_box::*;
}

pub mod style {
    pub use bevy_widgetry_core::z_index;
    pub use bevy_widgetry_core::{
        ColorTheme, DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin,
        WidgetryAppExt,
    };
}

pub mod tooltip {
    pub use bevy_widgetry_tooltip::*;
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
