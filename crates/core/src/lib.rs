//! Widgetry 的 Widget 共享的 theme、UI 构造阶段、默认字体、foreground color 传播和 icon 基础设施。

mod font;
mod foreground;
pub mod icon;
mod theme;
pub mod ui;
pub mod z_index;

pub use font::{WidgetryAppExt, WidgetryFontPlugin};
pub use foreground::{ForegroundColor, ForegroundColorPlugin};
pub use theme::{ColorTheme, DARK_THEME, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin};
