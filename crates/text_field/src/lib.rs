#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod auto_size;
mod composition;
mod content;
mod count;
mod input;
mod length;
mod registration;
mod style;
mod text_field;

pub use auto_size::AutoSize;
pub use registration::WidgetryTextFieldPlugin;
pub use text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPrefix, WidgetryTextFieldSuffix,
};

mod colors;
pub use colors::*;

pub struct WidgetryTextFieldProps {
    pub value: String,
    pub max_length: Option<usize>,
    pub show_count: bool,
    pub children: Option<Box<dyn bevy::prelude::SceneList>>,
    pub multiline: bool,
    pub auto_size: Option<AutoSize>,
    pub visible_lines: Option<f32>,
    pub visible_width: Option<f32>,
    pub colors: WidgetryTextFieldColorOverrides,
}

impl Default for WidgetryTextFieldProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            max_length: None,
            show_count: false,
            children: None,
            multiline: false,
            auto_size: None,
            visible_lines: Some(1.0),
            visible_width: None,
            colors: Default::default(),
        }
    }
}

pub mod internal {
    pub use crate::style::apply_owned_text_field_colors;
}
