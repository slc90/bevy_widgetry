//! 提供 Widgetry 的公共入口，供应用构造基础 Widget、数据视图及窗口组合界面。
//! 调用方可按所需功能使用各公开 module 中的 Widget、Plugin、Model 和 state API。
//!
//! 提供 Button、CheckBox、RadioGroup、TextField、ComboBox 和 Tooltip 的构造与交互入口。
//! 提供 ScrollArea、ListView、Tree 和 Table，用于浏览滚动内容、列表、层级数据和二维数据。
//! 提供 Waveform，用于显示多 channel 的时间序列数据。
//! 提供 Window、MessageBox 和 FileDialog，用于构造自定义窗口、modal 结果与后台文件选择交互。
//! theme module 提供完整 Light/Dark 配色与主题切换入口。
//! style module 提供 foreground color、默认字体设置和 z-index 标识。
//! icon module 提供 SVG Icon 的 Scene 构造、颜色设置与运行时替换入口。
//! scene module 提供 deferred Scene 构造与应用的扩展方法，将失败交给宿主 error handler。
//!
//! 各 Widget 通过自己的 Plugin 或类型注册入口启用对应功能。
//! Props 用于 Scene 的一次性构造，运行时 state 通过对应 Widget API 更新和查询。
//! selection、focus、disabled 与变化通知的具体含义遵循各 Widget 的公开契约。
//! 应用可以组合多个 Widget，并自行组织内容、layout 和业务响应。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

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
    pub use bevy_widgetry_core::{ForegroundColor, WidgetryAppExt};
}

pub mod theme {
    pub use bevy_widgetry_theme::*;
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

pub mod file_dialog {
    pub use bevy_widgetry_file_dialog::*;
}

pub mod text_field {
    pub use bevy_widgetry_text_field::*;
}

pub mod icon {
    pub use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin, WidgetryIconProps};
}
