//! 提供用于从固定选项中选择一项的 RadioGroup。
//! WidgetryRadioGroup 与 WidgetryRadioOption 可通过 BSN Scene 组合，适用于模式或互斥设置的选择。
//!
//! group 在初始化时选择首项，并使组内 selection 保持互斥。
//! option 支持组合调用方提供的标签和内容，通过 pointer click 选择对应项。
//! group 获得 focus 后，可通过方向键在 options 之间切换 selection。
//! 用户选择通过以 group 为 source 的 ValueChange 通知，并以 option index 表达选择结果。
//! set_selected 可通过 index 程序化设置组内 selection。
//! group 的 InteractionDisabled 会同步到 options，控制整个组的用户操作。
//! group 与 option 的背景、border 和 foreground 配色随 theme、focus 与交互 state 更新。
//!
//! group 要求至少一个 option，WidgetryRadioOption 必须是 group 的直接 child。
//! options 按构造时的固定集合使用，selection index 对应其 child 顺序。
//! 程序化 set_selected 更新 Checked，保持静默，用户输入的 ValueChange 用于响应实际选择操作。
//! 每个 group 独立维护互斥关系，多个 group 可在同一界面中并存。
//! 使用 Tab navigation 时，由调用方在祖先容器上提供 TabGroup。

use bevy_widgetry_theme::WidgetryThemePlugin;
#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod group;
mod group_style;
mod option;
mod option_style;

use bevy::input_focus::InputFocusSystems;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy::ui_widgets::{RadioGroupPlugin, radio_self_update};
use bevy_widgetry_core::ForegroundColorPlugin;
use bevy_widgetry_log::widgetry_info;
pub use group::WidgetryRadioGroup;
pub use option::WidgetryRadioOption;

pub struct WidgetryRadioGroupPlugin;

impl Plugin for WidgetryRadioGroupPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<RadioGroupPlugin>() {
            app.add_plugins(RadioGroupPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }
        app.add_observer(radio_self_update)
            .add_observer(group::handle_value_change)
            .add_observer(group_style::refresh_theme)
            .add_observer(option_style::refresh_theme);
        app.add_systems(
            PreUpdate,
            group::initialize
                .before(InputFocusSystems::Dispatch)
                .before(PickingSystems::Hover),
        );
        app.add_systems(
            Update,
            (
                group_style::update_changed,
                group_style::update_focus_and_removed,
                option_style::update_changed,
                option_style::update_removed,
            ),
        );
        widgetry_info!("WidgetryRadioGroupPlugin 注册完成");
    }
}
