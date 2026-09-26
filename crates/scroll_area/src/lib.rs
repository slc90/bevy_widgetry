//! ScrollArea 的 headless 滚动模型与输入行为。

mod headless;

pub use headless::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollAreaPlugin,
    WidgetryScrollAreaViewport, WidgetryScrollIntoView,
};
