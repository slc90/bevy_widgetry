use bevy::{
    app::{App, Plugin, PostUpdate},
    ecs::{component::Component, query::With, schedule::IntoScheduleConfigs, system::Query},
    text::{EditableText, EditableTextSystems, TextCursorStyle, TextLayout},
    ui::InteractionDisabled,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 由控件自身保存必需组件异常的边沿，销毁时不报告恢复。
#[derive(Component, Default, Debug)]
struct TextFieldDiagnostics {
    /// 上一帧是否已报告必需组件缺失。
    failed: bool,
}

/// Widgetry 的基础文本输入控件。
///
/// 编辑能力完全复用 Bevy 官方 `EditableText`。
#[derive(Component, Debug, Default)]
#[require(TextFieldDiagnostics)]
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

/// 在帧末检查必需组件，避免业务查询过滤掉损坏的控件。
fn diagnose_required_components(
    mut query: Query<
        (
            bevy::prelude::Entity,
            &mut TextFieldDiagnostics,
            bevy::ecs::query::Has<EditableText>,
            bevy::ecs::query::Has<TextLayout>,
            bevy::ecs::query::Has<TextCursorStyle>,
        ),
        With<TextField>,
    >,
) {
    for (entity, mut state, has_editable_text, has_text_layout, has_text_cursor_style) in &mut query
    {
        let failed = !(has_editable_text && has_text_layout && has_text_cursor_style);
        if failed != state.failed {
            if failed {
                widgetry_error!(
                    ?entity,
                    has_editable_text,
                    has_text_layout,
                    has_text_cursor_style,
                    "TextField 必需组件缺失"
                );
            } else {
                widgetry_info!(?entity, "TextField 必需组件恢复正常");
            }
            state.failed = failed;
        }
    }
}

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            block_disabled_text_field_edits.before(EditableTextSystems),
        );
        app.add_systems(bevy::app::PostUpdate, diagnose_required_components);
        widgetry_info!("TextFieldPlugin 注册完成");
    }
}
