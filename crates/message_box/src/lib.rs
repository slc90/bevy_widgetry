//! 提供 non-blocking modal MessageBox，用于显示需要用户确认或选择结果的 dialog。
//! 应用创建 dialog 后继续运行，通过后续结果 event 响应用户选择。
//!
//! widgetry_message_box 接收 parent native window、title、按钮组合和正文 SceneList。
//! 正文可组合文字、Icon 和其他 Widget，按钮区域提供 Ok、YesNo 或 YesNoCancel 三种组合。
//! 用户 activation 产生 Ok、Yes、No 或 Cancel 结果，并通过 WidgetryMessageBoxResultEvent 通知。
//! 结果 event 的 entity 指向 dialog root，observer 可以读取 dialog state 或排队业务操作。
//! dialog 作为 modal child 阻挡 parent 界面的 pointer 交互，并使用独立窗口展示内容。
//! 窗口、正文和结果按钮随 theme 更新配色。
//!
//! parent 参数使用已绑定 Widgetry 窗口界面的 native window Entity。
//! 每个 dialog 最多决议一次，重复 activation 保留第一次已接受的结果。
//! 结果通知在 dialog 关闭前发出，observer 排队的操作完成后再回收 dialog。
//! dialog 关闭时回收自己创建的窗口资源，parent lifecycle 结束时也会清理对应 dialog。
//! 外部销毁或 parent 关闭属于 lifecycle 结束。Cancel 按钮、Escape 与 native close request 提交 Cancel 结果。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod lifecycle;
mod scene;

use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_log::widgetry_info;
use bevy_widgetry_window::WidgetryWindowPlugin;
pub use scene::{
    WidgetryMessageBox, WidgetryMessageBoxButtons, WidgetryMessageBoxResult,
    WidgetryMessageBoxResultEvent, widgetry_message_box,
};

pub struct WidgetryMessageBoxPlugin;

impl Plugin for WidgetryMessageBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryWindowPlugin>() {
            app.add_plugins(WidgetryWindowPlugin);
        }
        if !app.is_plugin_added::<WidgetryButtonPlugin>() {
            app.add_plugins(WidgetryButtonPlugin);
        }
        app.add_observer(scene::refresh_theme)
            .add_observer(lifecycle::finish_closing)
            .add_observer(lifecycle::escape)
            .add_systems(
                Last,
                lifecycle::close_requests.before(bevy::window::close_when_requested),
            );
        widgetry_info!("WidgetryMessageBoxPlugin 注册完成");
    }
}
