//! 提供 Widgetry 界面的固定 Light/Dark 配色与主题选择能力。
//! 调用方可读取各 Widget 与组合部件的完整 state 配色，构造一致的界面。
//!
//! 提供 Text、Icon、基础 Widget、数据视图、窗口与对话内容的独立 Colors 数据。
//! Waveform 提供四 channel 起始 palette，并保留禁用 palette。
//! WidgetryThemeMode 提供 World 与 Commands 切换入口，以全局 event 通知消费者。
//!
//! 默认使用 Dark，Light/Dark 均返回固定的完整配色。
//! 切换先提交 mode 再通知，合法同值不通知，Commands 在实际执行时提交。
//! Plugin 仅管理主题选择，UI 刷新由各消费者负责。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod button;
mod check_box;
mod combo_box;
mod common;
mod file_dialog;
mod icon;
mod list_view;
mod message_box;
mod radio_group;
mod scroll_area;
mod table;
mod text;
mod text_field;
mod theme;
mod tooltip;
mod tree;
mod waveform;
mod window;

pub use button::{WidgetryButtonColors, WidgetryButtonStateColors};
pub use check_box::{
    WidgetryCheckBoxColors, WidgetryCheckBoxInteractionColors, WidgetryCheckBoxStateColors,
};
pub use combo_box::{
    WidgetryComboBoxColors, WidgetryComboBoxFieldColors, WidgetryComboBoxFieldStateColors,
    WidgetryComboBoxPopupColors, WidgetryComboBoxPopupStateColors,
};
pub use file_dialog::{
    WidgetryFileDialogBodyColors, WidgetryFileDialogBodyStateColors, WidgetryFileDialogColors,
    WidgetryFileDialogEntryColors, WidgetryFileDialogEntryStateColors,
    WidgetryFileDialogStatusColors, WidgetryFileDialogStatusStateColors,
};
pub use icon::{WidgetryIconColors, WidgetryIconStateColors};
pub use list_view::{
    WidgetryListViewColors, WidgetryListViewContainerColors, WidgetryListViewContainerStateColors,
    WidgetryListViewItemColors, WidgetryListViewItemStateColors,
};
pub use message_box::{
    WidgetryMessageBoxBodyColors, WidgetryMessageBoxBodyStateColors, WidgetryMessageBoxColors,
};
pub use radio_group::{
    WidgetryRadioGroupColors, WidgetryRadioGroupContainerColors,
    WidgetryRadioGroupContainerStateColors, WidgetryRadioOptionColors,
    WidgetryRadioOptionInteractionColors, WidgetryRadioOptionStateColors,
};
pub use scroll_area::{
    WidgetryScrollAreaColors, WidgetryScrollAxisColors, WidgetryScrollThumbColors,
    WidgetryScrollThumbStateColors, WidgetryScrollTrackColors, WidgetryScrollTrackStateColors,
};
pub use table::{WidgetryTableColors, WidgetryTableRegionColors, WidgetryTableStateColors};
pub use text::{WidgetryTextColors, WidgetryTextStateColors};
pub use text_field::{
    WidgetryTextFieldColors, WidgetryTextFieldInteractionColors, WidgetryTextFieldStateColors,
};
pub use theme::{
    WIDGETRY_DARK_THEME, WIDGETRY_LIGHT_THEME, WidgetryTheme, WidgetryThemeChanged,
    WidgetryThemeMode, WidgetryThemePlugin,
};
pub use tooltip::{WidgetryTooltipColors, WidgetryTooltipPopupColors, WidgetryTooltipStateColors};
pub use tree::{
    WidgetryTreeColors, WidgetryTreeExpanderColors, WidgetryTreeExpanderInteractionColors,
    WidgetryTreeExpanderStateColors,
};
pub use waveform::{WidgetryWaveformColors, WidgetryWaveformStateColors};
pub use window::{
    WidgetryWindowButtonColors, WidgetryWindowButtonStateColors, WidgetryWindowColors,
    WidgetryWindowStateColors, WidgetryWindowSurfaceColors,
};
