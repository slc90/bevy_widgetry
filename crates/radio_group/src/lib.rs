//! 基于 Bevy 官方 Radio 行为、使用 direct child index 的标准 RadioGroup。

mod group;
mod group_style;
mod option;
mod option_style;

use bevy::input_focus::InputFocusSystems;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy::ui_widgets::{RadioGroupPlugin, radio_self_update};
use bevy_widgetry_core::{ForegroundColorPlugin, ThemePlugin, WidgetryFocusPlugin};
use bevy_widgetry_log::widgetry_info;
pub use group::WidgetryRadioGroup;
pub use option::WidgetryRadioOption;

/// 装配 RadioGroup 的官方行为、theme、foreground 传播及共享 focus 策略。
/// 自动补齐 RadioGroupPlugin、TabNavigationPlugin、WidgetryFocusPlugin、ThemePlugin 和 ForegroundColorPlugin。
/// BSN 依赖应用的 Scene 设施，真实输入依赖 Bevy 的 picking、InputFocusPlugin 与输入派发设施。
/// 不安装字体 fallback，用户文本的字体由调用方配置。
pub struct WidgetryRadioGroupPlugin;

impl Plugin for WidgetryRadioGroupPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<RadioGroupPlugin>() {
            app.add_plugins(RadioGroupPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<WidgetryFocusPlugin>() {
            app.add_plugins(WidgetryFocusPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
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
            (group::initialize, group::mirror_disabled)
                .chain()
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
