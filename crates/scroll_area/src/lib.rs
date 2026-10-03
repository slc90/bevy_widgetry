//! 提供 ScrollArea，支持两轴滚动、scrollbar policy、keyboard navigation 和目标 reveal。

mod headless;
mod layout;
mod style;

pub use headless::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollAreaContent,
    WidgetryScrollAreaPlugin, WidgetryScrollAreaViewport, WidgetryScrollIntoView,
};
pub use style::{WidgetryScrollArea, WidgetryScrollAreaProps};
