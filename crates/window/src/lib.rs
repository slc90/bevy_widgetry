//! 提供自定义窗口界面的 Scene 构造与 native window 操作能力。
//! 适用于需要自定义 title bar、正文内容、多窗口和 modal 交互的桌面应用。
//!
//! widgetry_window 将调用方已有的 native window 与 camera 绑定到 Widgetry 窗口界面。
//! owned_widgetry_window 创建窗口与 camera，并由窗口界面的 lifecycle 管理这些资源。
//! title bar 与正文均接收 SceneList，可组合标签、工具按钮和其他 Widget 内容。
//! 支持拖动 title bar 移动窗口，通过边缘 resize 区域调整尺寸。
//! 提供最小化、最大化或恢复、关闭 controls，可分别配置是否显示，并配置窗口是否 resizable。
//! prepare_native_window 配置透明、无 native decorations 的窗口属性，用于承载自定义窗口外观。
//! WidgetryModalWindow 将 child 关联到 parent native window，并在 parent 界面提供 pointer blocker。
//! 构造时显式选择 Theme 背景或带 opacity 的 Image 背景。
//! Theme 背景、窗口 border 与 title bar border 随 theme 更新，Image 背景保持构造配置。
//!
//! 调用方提供的 native window 和 camera 由调用方持有，窗口界面回收时保留这些资源。
//! owned 窗口在界面 root 回收时清理自己创建的 native window 与 camera。
//! 一个 native window 与 camera 分别绑定到一个窗口界面，构造时要求目标有效且窗口属性已准备。
//! 透明窗口还需要宿主配置支持透明 compositing 的 rendering 环境。
//! title bar 与正文的自定义内容由调用方或所组合的 Widget 管理配色。
//! 内建 controls 使用固定配色，并按 hover 和 pressed 更新外观。
//! modal 的 parent 参数使用 native window Entity，parent 界面结束时清理对应 modal child。
//! 同一 parent 的多个 modal child 共享 blocker，最后一个有效 child 结束后释放阻挡。

mod background;
mod modal;
mod scene;
mod title_bar;
mod window_root;

pub use background::{
    WidgetryWindowBackground, WidgetryWindowImageBackground, WidgetryWindowImageMode,
};
pub use modal::WidgetryModalWindow;
pub use scene::{
    WidgetryWindowControlsConfig, owned_widgetry_window, prepare_native_window, widgetry_window,
};
pub use title_bar::WidgetryWindowPlugin;
