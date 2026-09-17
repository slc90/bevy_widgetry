//! Widgetry 共享测试基础设施，提供无窗口测试环境、交互事件构造、主题切换和日志捕获。

mod logging;
mod pointer;
mod scene;
mod theme;

pub use logging::{LogCapture, LogRecord};
pub use pointer::{
    cancel, drag_end, press, primary_cancel, primary_click, primary_drag_end, primary_press,
    primary_release, release,
};
pub use scene::{scene_app, text_input_app};
pub use theme::switch_theme;
