#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod button;
mod check_box;
mod combo_box;
mod common;
mod file_dialog;
mod icon;
mod link_button;
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
pub use link_button::WidgetryLinkButtonColors;
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
    WIDGETRY_DARK_THEME, WIDGETRY_KAMURI_VIOLET_THEME, WIDGETRY_LIGHT_THEME,
    WIDGETRY_PINK_DREAM_THEME, WidgetryTheme, WidgetryThemeChanged, WidgetryThemeMode,
    WidgetryThemePlugin,
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
