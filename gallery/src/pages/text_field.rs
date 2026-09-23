use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::text_field::{WidgetryReadOnlyTextField, WidgetryTextField};

/// 展示官方默认 single-line、可见四行的自然高度及 disabled 编辑外观。
pub(crate) fn scene() -> impl SceneList {
    bsn_list! [
        (@WidgetryTextField
            template_value(EditableText::new("Single line"))
            Node { margin: UiRect::bottom(px(16)) }
        ),
        (@WidgetryTextField
            template_value(EditableText::new("First line\nSecond line\nThird line\nFourth line"))
            EditableText { visible_lines: {Some(4.0)}, allow_newlines: true }
            Node { margin: UiRect::bottom(px(16)) }
        ),
        (@WidgetryReadOnlyTextField
            template_value(EditableText::new("Read-only text: select and copy me"))
            Node { margin: UiRect::bottom(px(16)) }
        ),
        (@WidgetryTextField template_value(EditableText::new("Disabled")) InteractionDisabled),
    ]
}
