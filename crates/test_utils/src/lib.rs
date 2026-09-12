//! 在无窗口测试中构造指针事件并推进观察者命令。

mod logging;
mod pointer;
mod theme;

pub use logging::{LogCapture, LogRecord};
pub use pointer::{
    cancel, drag_end, press, primary_cancel, primary_click, primary_drag_end, primary_press,
    primary_release, release,
};
pub use theme::switch_theme;
