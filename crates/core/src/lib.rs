//! 提供 Widgetry 共享的 theme、UI 构造阶段、默认字体、foreground color 传播和 Icon。

pub mod diagnostics;
mod font;
mod foreground;
pub mod icon;
pub mod scene;
mod theme;
pub mod ui;
pub mod z_index;

pub use font::{WidgetryAppExt, WidgetryFontPlugin};
pub use foreground::{ForegroundColor, ForegroundColorPlugin};
pub use theme::{ColorTheme, DARK_THEME, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin};
