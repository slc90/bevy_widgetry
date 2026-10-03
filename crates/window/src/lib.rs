//! 提供自定义 Window 控件、theme 样式、native window 交互与 modal 支持。

mod modal;
mod scene;
mod title_bar;
mod window_root;

pub use modal::WidgetryModalWindow;
pub use scene::{
    WidgetryWindowControlsConfig, owned_widgetry_window, prepare_native_window, widgetry_window,
};
pub use title_bar::WidgetryWindowPlugin;
