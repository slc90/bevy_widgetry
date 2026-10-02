//! Widgetry 共享测试基础设施，提供 headless 测试环境、interaction event 构造、theme 切换和日志捕获。

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
