//! 提供二态与三态 CheckBox，用于表达开关选择以及部分选中的状态。
//! 两种 Widget 均可通过 BSN Scene 构造，并组合调用方提供的标签或其他内容。
//!
//! WidgetryCheckBox 使用 Checked 表达二态选择，提供随 theme 和交互 state 更新的配色与指示图标。
//! WidgetryTriStateCheckbox 使用 WidgetryCheckState 表达 Unchecked、Checked 和 Indeterminate。
//! 三态 Widget 支持 pointer activation，以及获得 focus 后通过 Space 或 Enter 切换 state。
//! set_state 可指定三态值，cycle_state 可按 Unchecked → Checked → Indeterminate → Unchecked 循环切换。
//! 三态值发生变化时，通过 ValueChange 通知调用方，并同步对应的 accessibility state。
//! theme 与 disabled state 变化会更新指示、背景、border 和内容配色。
//!
//! 三态 Widget 默认处于 Unchecked，程序化更新在 Commands 实际执行时提交。
//! 三态通知在真实 state 提交后发出，同值设置保持 state 且不发变化通知。
//! InteractionDisabled 阻止用户切换，三态程序化更新入口仍可用于设置 state。
//! 二态 Checked 保持 Bevy 官方 Component 的使用方式。

mod checkbox;
mod indicator;
mod style;
mod tri_state;

use bevy::prelude::*;
use bevy::ui_widgets::CheckboxPlugin;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_core::{ForegroundColorPlugin, ThemePlugin};
use bevy_widgetry_log::widgetry_info;
pub use checkbox::WidgetryCheckBox;
pub use tri_state::{WidgetryCheckState, WidgetryTriStateCheckbox};

pub struct WidgetryCheckBoxPlugin;

impl Plugin for WidgetryCheckBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<CheckboxPlugin>() {
            app.add_plugins(CheckboxPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }
        if !app.is_plugin_added::<WidgetryAssetPlugin>() {
            app.add_plugins(WidgetryAssetPlugin);
        }
        if !app.is_plugin_added::<WidgetryIconPlugin>() {
            app.add_plugins(WidgetryIconPlugin);
        }
        app.add_observer(tri_state::on_press)
            .add_observer(style::refresh_theme)
            .add_observer(tri_state::on_click)
            .add_observer(tri_state::on_release)
            .add_observer(tri_state::on_drag_end)
            .add_observer(tri_state::on_cancel)
            .add_observer(tri_state::on_key)
            .add_systems(
                Update,
                (
                    tri_state::sync_accessibility,
                    style::update_changed,
                    style::update_removed,
                ),
            );
        widgetry_info!("WidgetryCheckBoxPlugin 注册完成");
    }
}
