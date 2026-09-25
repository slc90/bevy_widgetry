//! Widgetry overlay 使用的全局层级 token。

/// Widget 内局部浮层，例如 Gallery title bar 中的 selector。
pub const LOCAL_OVERLAY: i32 = 10;

/// 普通 popup，例如 ComboBox option list。
pub const POPUP: i32 = 100;

/// 非交互提示层，始终高于普通 popup。
pub const TOOLTIP: i32 = 200;

/// parent window 的 pointer modal blocker。
pub const MODAL: i32 = 100_000;
