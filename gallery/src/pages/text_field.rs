use bevy::prelude::*;
use bevy::text::EditableText;
use bevy_widgetry::text_field::StyledTextField;

/// 保留两个可独立编辑的输入框及其原始文本。
pub(crate) fn scene() -> impl SceneList {
    bsn_list! [
        (template(|_| Ok(StyledTextField)) template(|_| Ok(EditableText::new("A")))),
        (template(|_| Ok(StyledTextField)) template(|_| Ok(EditableText::new("B")))),
    ]
}
