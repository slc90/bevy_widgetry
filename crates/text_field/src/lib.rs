#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod content;
mod input;
mod length;
mod registration;
mod style;
mod text_field;

pub use registration::WidgetryTextFieldPlugin;
pub use text_field::{WidgetryReadOnlyTextField, WidgetryTextField};

mod colors;
pub use colors::*;

#[derive(Default, Clone, Debug)]
pub struct WidgetryTextFieldProps {
    pub value: String,
    pub max_length: Option<usize>,
    pub colors: WidgetryTextFieldColorOverrides,
}

pub mod internal {
    pub use crate::style::apply_owned_text_field_colors;
}
