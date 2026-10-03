//! 提供按 hover timing 显示任意 BSN 内容的 Tooltip 和 theme style。

mod headless;
mod style;

pub use style::{
    TooltipContentFactory, WidgetryTooltip, WidgetryTooltipPlugin, WidgetryTooltipProps,
};
