//! 提供用于从列表中选择一个选项的 ComboBox，适用于设置项、模式选择和其他下拉选择场景。
//! 选项来自 WidgetryListModel，关闭时的 Field 展示当前 selection，展开时通过 Popup 浏览选项。
//!
//! 支持 generic 选项数据，并通过 renderer 为 Field 和列表项构造文字、Icon 或其他 BSN 内容。
//! 可配置 item_height 和 max_visible_items，控制行高以及 Popup 可见行数上限。
//! Popup 支持 pointer 选择、keyboard navigation 和滚动浏览。
//! 选择选项后关闭 Popup，Escape 与外部 click 也可关闭 Popup。
//! 支持通过 stable item ID 设置 selection，或通过 clear_selection 清空当前选择。
//! selection 变化通过以 ComboBox 为 source 的 ValueChange 通知，payload 可表达未选中。
//! Model 的插入、移除、移动和内容更新会反映到选项与 Field 展示中。
//! 提供随 theme 更新的 Field、Popup 和展开状态箭头配色。
//!
//! 使用前通过 register_widgetry_combo_box 注册对应数据类型，并提供匹配的 Model source 与 renderer。
//! 非空 Model 在首次初始化时默认选择首项，空 Model 保持未选中。
//! selection 通过 stable item ID 表达，选项移动后仍可保持同一业务选项。
//! 程序化设置与用户选择共享真实 selection，程序化设置保留已有 Popup 状态。
//! InteractionDisabled 阻止用户操作，调用方仍可通过 API 调整 selection。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod combo_box;
mod field;
mod popup;
mod registration;

use bevy::prelude::*;
use bevy::ui_widgets::popover::PopoverPlugin;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_list_view::WidgetryListViewPlugin;
use bevy_widgetry_log::widgetry_info;
pub use combo_box::{WidgetryComboBox, WidgetryComboBoxProps};
pub use registration::WidgetryComboBoxAppExt;

pub struct WidgetryComboBoxPlugin;

impl Plugin for WidgetryComboBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryButtonPlugin>() {
            app.add_plugins(WidgetryButtonPlugin);
        }
        if !app.is_plugin_added::<WidgetryAssetPlugin>() {
            app.add_plugins(WidgetryAssetPlugin);
        }
        if !app.is_plugin_added::<WidgetryIconPlugin>() {
            app.add_plugins(WidgetryIconPlugin);
        }
        if !app.is_plugin_added::<WidgetryListViewPlugin>() {
            app.add_plugins(WidgetryListViewPlugin);
        }
        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }
        app.add_observer(popup::handle_outside_click)
            .add_observer(popup::refresh_theme);
        widgetry_info!("WidgetryComboBoxPlugin 注册完成");
    }
}
