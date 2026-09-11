use bevy::prelude::*;
use bevy_widgetry::button::StyledButton;

/// 保留原有按钮 demo 的尺寸和内容。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        #ButtonDemo
        template(|_| Ok(StyledButton))
        Node {
            width: px(160),
            height: px(40),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [Text("Button")]
    }
}
