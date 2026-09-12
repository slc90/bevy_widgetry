//! Widgetry 控件共享的主题、默认字体、前景色传播和图标基础设施。

mod font;
mod foreground;
pub mod icon;
mod theme;

pub use font::{WidgetryAppExt, WidgetryFontPlugin};
pub use foreground::{ForegroundColor, ForegroundColorPlugin};
pub use theme::{ColorTheme, DARK_THEME, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin};
