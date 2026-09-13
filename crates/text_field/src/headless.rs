use bevy::{
    app::{App, Plugin, PostUpdate},
    ecs::{component::Component, query::With, schedule::IntoScheduleConfigs, system::Query},
    text::{EditableText, EditableTextSystems, TextCursorStyle, TextLayout},
    ui::InteractionDisabled,
};
use bevy_widgetry_log::widgetry_info;

/// Widgetry 的基础文本输入控件。
///
/// 编辑能力完全复用 Bevy 官方 `EditableText`。
#[derive(Component, Debug, Default)]
#[require(EditableText, TextLayout = TextLayout::no_wrap(), TextCursorStyle)]
pub struct TextField;

/// 在编辑处理前清除禁用输入框的用户编辑队列，不阻止程序化赋值。
pub struct TextFieldPlugin;

/// 只清除禁用输入框的待处理编辑，不覆盖当前文本内容。
fn block_disabled_text_field_edits(
    mut query: Query<&mut EditableText, (With<TextField>, With<InteractionDisabled>)>,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.clear();
        editable_text.pending_paste = None;
    }
}

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            block_disabled_text_field_edits.before(EditableTextSystems),
        );
        widgetry_info!("TextFieldPlugin 注册完成");
    }
}
