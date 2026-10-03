//! 提供 Widgetry 共享的 headless 测试环境、输入构造、theme 切换、日志与错误捕获和 benchmark 测量。

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
