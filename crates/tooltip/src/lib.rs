//! 提供在有效 Pointer hover 后显示辅助内容的 Tooltip，适用于标签说明、操作提示和其他补充信息。
//! 可在目标上构造 WidgetryTooltip，并通过 content factory 提供文字、Icon 或其他 BSN 内容。
//!
//! 根据当前 hover 命中寻找 Tooltip anchor，也可沿命中对象的祖先找到绑定的 Tooltip。
//! 首次 hover 使用较长等待时间，短时间内连续查看其他 Tooltip 时使用较短的 warm 等待时间。
//! 显示时调用 content factory 构造 Popup，离开 anchor 或目标消失时清理显示内容。
//! Popup 根据 anchor 与窗口空间选择位置，支持下、上、右、左四个候选方向。
//! Popup 的背景、border 与 foreground color 随 theme 更新。
//! Tooltip 内容使用独立的浮层顺序，并保持底层目标的 pointer 交互。
//!
//! 构造时必须提供 content factory，内容在每次显示时生成。
//! cold 等待时间为 200ms，warm 等待时间为 50ms，离开后的 warm 保留时间为 300ms。
//! timing 使用真实经过时间，适用于按需更新的桌面 App。
//! 同一 App 同时显示一个 Tooltip，按有效输入选择 Pointer owner；无输入时仅采用唯一有效 hover source。
//! owner 或窗口 target 改变时重新开始 cold 等待；Cancel、位置失效或 Pointer / 窗口注销时清理。
//! 同 owner 内切换 anchor 时先结束当前显示，并保留 warm 等待规则。
//! disabled 目标仍可显示 Tooltip，便于解释当前操作为何不可用。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod headless;
mod style;

pub use style::{
    TooltipContentFactory, WidgetryTooltip, WidgetryTooltipPlugin, WidgetryTooltipProps,
};
