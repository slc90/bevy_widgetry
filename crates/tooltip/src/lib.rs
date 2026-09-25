//! 通过 hover ancestor lookup 显示任意 BSN 内容的 Widgetry Tooltip。

mod headless;
mod style;

pub use style::{
    TooltipContentFactory, WidgetryTooltip, WidgetryTooltipPlugin, WidgetryTooltipProps,
};
