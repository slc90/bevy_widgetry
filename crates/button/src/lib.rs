//! 提供可承载自定义内容的 Button，用于触发应用操作或作为组合 Widget 的交互入口。
//! WidgetryButton 可通过 BSN Scene 构造，并由调用方添加文字、Icon 或其他 children。
//!
//! 支持 Bevy Button 的 activation 行为，调用方可通过 Activate observer 响应操作。
//! 根据 normal、hover、pressed 和 disabled state 更新背景、border 与 foreground 配色。
//! theme 切换后刷新 Button 配色，并将 foreground color 传播到内容。
//! 调用方可以配置 Node 的尺寸、padding 和 layout，以适配不同内容与界面位置。
//!
//! 同时存在多个交互 state 时，配色优先级为 disabled、pressed、hover、normal。
//! InteractionDisabled 用于控制 Button 是否接受用户 activation。
//! 内容与字体由调用方配置，Button 的 theme 更新保留调用方的 Node 配置和 children。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod colors;
mod style;
pub use colors::*;

pub use style::{WidgetryButton, WidgetryButtonPlugin, WidgetryButtonProps};

pub mod internal {
    pub use crate::style::apply_owned_button_colors;
}
