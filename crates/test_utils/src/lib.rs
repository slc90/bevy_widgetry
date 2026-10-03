//! 提供用于验证 Widgetry 行为的 headless App、输入构造、诊断捕获和 benchmark 工具。
//! 调用方可用这些入口建立 Scene、文本编辑或 UI layout 场景，并控制输入与 update 的推进。
//!
//! scene_app 提供基础 Scene 环境，text_input_app 和 text_edit_app 提供文本输入与编辑场景。
//! add_ui_plugins 与 spawn_ui_camera 可为场景补充 UI 准备能力，并指定 camera 尺寸和 scale factor。
//! 提供 pointer press、click、release、cancel 和 drag 结束等 event 构造入口。
//! queue_key 用于排队 KeyboardInput，press_key 排队一次按键并推进一次 update。
//! add_keyboard_dispatch 使 keyboard message 经 focus dispatch 到当前目标。
//! switch_theme 同时更新 ThemeMode 并通知界面刷新，advance_until 可等待指定 state 直到超时。
//! LogCapture 保存 Widgetry 日志的级别、调用位置和 structured field，ErrorCapture 收集捕获 scope 中的 BevyError。
//! benchmark module 提供 UI fixture、可见文字准备检查、Criterion harness 和测量 artifact 输出。
//!
//! headless 场景通过调用方显式推进 update，便于检查输入前后和生成当帧的 state。
//! pointer helper 构造目标 event，实际 picking 命中与 native window 交互需要对应的运行环境验证。
//! 日志与错误捕获 scope 限于当前 thread，相关 schedule 应使用 single-threaded 执行。
//! benchmark fixture 与准备检查由调用方按被测场景配置，等待未达到目标时返回超时信息。

pub mod benchmark;
mod error;
mod logging;
mod pointer;
mod scene;
mod theme;

pub use error::ErrorCapture;
pub use logging::{LogCapture, LogRecord};
pub use pointer::{
    cancel, drag_end, press, primary_cancel, primary_click, primary_drag_end, primary_press,
    primary_release, release,
};
pub use scene::{
    add_keyboard_dispatch, add_ui_plugins, advance_until, press_key, queue_key, scene_app,
    spawn_ui_camera, text_edit_app, text_input_app,
};
pub use theme::switch_theme;
