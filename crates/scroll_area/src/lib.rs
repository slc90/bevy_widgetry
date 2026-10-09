//! 提供用于展示超出可见区域内容的 ScrollArea。
//! 可承载通过 BSN Scene 构造的内容和 children，适用于需要滚动浏览的列表、面板及其他 UI 内容。
//! 内容超出 viewport 时，调用方可以通过用户输入或程序控制调整可见位置。
//!
//! 支持水平、垂直和两轴滚动，可根据内容的布局方向选择允许滚动的 Axis。
//! 水平和垂直 scrollbar 可分别配置 Auto、Always 或 Hidden 显示策略。
//! Auto 根据内容是否溢出决定显示，Always 始终显示，Hidden 隐藏对应 scrollbar。
//! 支持配置 scrollbar 的厚度，并提供随 theme 更新的 thumb 配色。
//! thumb 在 hover 和 drag 时显示对应的交互配色。
//! 支持通过滚轮和拖动 scrollbar thumb 浏览内容。
//! 启用 keyboard scrolling 且 ScrollArea 获得 focus 时，方向键沿允许的 Axis 滚动。
//! 垂直滚动还支持 PageUp、PageDown 按 viewport 高度翻页，以及 Home、End 跳到起点或终点。
//! 调用方可以修改 viewport 上的 ScrollPosition，程序化控制滚动位置。
//! 可向内容 entity 发出 WidgetryScrollIntoView，使其所在的最近 ScrollArea 调整可见位置。
//!
//! 默认启用垂直滚动、Auto scrollbar 和 keyboard scrolling。
//! scrollbar 的显示策略与允许滚动的 Axis 分别配置，Hidden 不关闭对应 Axis 的滚动能力。
//! keyboard scrolling 可独立关闭，关闭后仍可通过其他入口调整滚动位置。
//! keyboard scrolling 和 WidgetryScrollIntoView 将位置限制在有效滚动范围内。
//! WidgetryScrollIntoView 在目标超出可见区域时调整位置，目标已完整可见时保留当前位置。
//! 嵌套 ScrollArea 中，WidgetryScrollIntoView 由目标所在的最近 viewport 处理。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod disabled;
mod headless;
mod layout;
mod pointer;
mod style;

pub use headless::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollAreaContent,
    WidgetryScrollAreaPlugin, WidgetryScrollAreaViewport, WidgetryScrollIntoView,
};
pub use style::{WidgetryScrollArea, WidgetryScrollAreaProps};
