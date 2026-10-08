//! 提供构造 Widgetry 界面时使用的 theme、字体、内容颜色、SVG Icon 和 Scene 操作能力。
//! 调用方可按需启用这些功能，为自定义 Widget 和组合界面配置一致的外观与内容。
//!
//! 提供 Dark 与 Light 两种 ColorTheme，覆盖窗口、controls、Popup、列表项与文字 selection 的配色。
//! ThemeMode 用于选择当前配色，ThemeChanged 用于通知界面刷新。
//! 提供默认字体设置，为未显式选择字体的新 TextFont 应用默认字体，并保留显式字体选择。
//! 提供 foreground color 的 hierarchy 传播，使文字和 Icon 可以跟随容器的内容颜色。
//! WidgetryIcon 支持从 SVG 路径构造图标，配置最大尺寸、显式颜色以及运行时 SVG 替换。
//! Icon 可清除显式颜色以恢复继承颜色，asset 就绪后生成对应的可见内容。
//! WidgetryPointerPlugin 使整个 App 的官方 Hovered / DirectlyHovered 反映所有有效 Pointer 的命中。
//! Mouse、Custom 与 Touch 共用该语义，Hovered 包含 descendant，DirectlyHovered 仅含直接命中。
//! Widgetry UI 与独立 Widget plugin 自动装配该能力，也可单独安装共享 plugin。
//! 该 plugin 拥有这两个 state 的唯一写入权，宿主自定义 writer 需自行协调。
//! 提供动态 UI 内容的构造阶段入口，便于新内容参与当前帧的 UI 准备。
//! 提供 deferred Scene 构造与应用入口，将构造失败交给宿主 error handler。
//! 提供诊断 state 与 z-index 标识，供自定义 Widget 处理失败状态和浮层顺序。
//!
//! ThemeMode 默认使用 Dark，切换配色时由调用方更新 mode 并发出 ThemeChanged。
//! 默认字体通过 WidgetryFontPlugin 或 set_default_font 启用，显式设置的字体保持原选择。
//! Icon 的 Props 用于一次性构造，后续颜色与 SVG 调整通过公开 API 进行。
//! Scene 错误处理使用宿主的 handler，应用可自行决定失败时的响应方式。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

pub mod diagnostics;
mod font;
mod foreground;
pub mod icon;
pub mod pointer;
pub mod scene;
mod theme;
pub mod ui;
pub mod z_index;

pub use font::{WidgetryAppExt, WidgetryFontPlugin};
pub use foreground::{ForegroundColor, ForegroundColorPlugin};
pub use theme::{ColorTheme, DARK_THEME, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin};
