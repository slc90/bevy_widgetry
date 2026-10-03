//! 提供各 Widget、theme、字体、Icon 和 Scene 构造的 Widgetry 公共入口。

pub mod scene {
    pub use bevy_widgetry_core::scene::{WidgetrySceneCommandsExt, WidgetrySceneEntityCommandsExt};
}

pub mod button {
    pub use bevy_widgetry_button::*;
}

pub mod combo_box {
    pub use bevy_widgetry_combo_box::*;
}

pub mod radio_group {
    pub use bevy_widgetry_radio_group::*;
}

pub mod scroll_area {
    pub use bevy_widgetry_scroll_area::*;
}

pub mod list_view {
    pub use bevy_widgetry_list_view::*;
}

pub mod tree {
    pub use bevy_widgetry_tree::*;
}

pub mod table {
    pub use bevy_widgetry_table::*;
}

pub mod waveform {
    pub use bevy_widgetry_waveform::*;
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

pub mod message_box {
    pub use bevy_widgetry_message_box::*;
}

pub mod text_field {
    pub use bevy_widgetry_text_field::*;
}

pub mod icon {
    pub use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin, WidgetryIconProps};
}
