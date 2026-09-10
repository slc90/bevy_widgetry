use bevy::{
    app::{App, Plugin, PostUpdate},
    ecs::{component::Component, query::With, schedule::IntoScheduleConfigs, system::Query},
    text::{EditableText, EditableTextSystems, TextCursorStyle, TextLayout},
    ui::InteractionDisabled,
};

/// Widgetry 的基础文本输入控件。
///
/// 编辑能力完全复用 Bevy 官方 `EditableText`。
#[derive(Component, Debug, Default)]
#[require(EditableText, TextLayout = TextLayout::no_wrap(), TextCursorStyle)]
pub struct TextField;

pub struct TextFieldPlugin;

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            block_disabled_text_field_edits.before(EditableTextSystems),
        );
    }
}

fn block_disabled_text_field_edits(
    mut query: Query<&mut EditableText, (With<TextField>, With<InteractionDisabled>)>,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.clear();
        editable_text.pending_paste = None;
    }
}
