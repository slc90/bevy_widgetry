//! 提供用于输入或展示可选择文字的 TextField 与 ReadOnly TextField。
//! 两种 Widget 均可通过 BSN Scene 构造，并使用 EditableText 保存文字、selection 和编辑 state。
//!
//! WidgetryTextField 提供文本编辑入口，可结合 Bevy 的 keyboard、clipboard 和 IME 输入能力使用。
//! WidgetryReadOnlyTextField 保留 cursor navigation、selection 与 copy，过滤会修改文本的编辑操作。
//! InteractionDisabled 用于阻止用户编辑，并清理待处理的 edit 与 paste 请求。
//! 背景、border、文字、cursor 和 selection 配色随 theme、hover、focus 与 disabled 更新。
//! pointer 获取 focus 后保留当前输入目标，便于直接 click 进入文本操作。
//! 调用方可通过 EditableText 配置内容和 selection，并通过 Node 配置尺寸与 layout。
//!
//! ReadOnly 约束用户编辑入口，调用方仍可程序化修改 EditableText 内容。
//! disabled state 的输入过滤优先于 ReadOnly，恢复启用后接受后续新输入。
//! theme 和交互配色更新保留已有文本与 selection。
//! keyboard、clipboard 和 IME 的实际输入由宿主配置的 Bevy 文本输入环境提供。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod style;

pub use style::{WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin};

mod colors;
pub use colors::*;

#[derive(Default, Clone, Debug)]
pub struct WidgetryTextFieldProps {
    pub colors: WidgetryTextFieldColorOverrides,
}

pub mod internal {
    pub use crate::style::apply_owned_text_field_colors;
}
