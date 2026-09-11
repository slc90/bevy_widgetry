#![cfg(test)]

use bevy::{
    app::App,
    text::{EditableText, LineBreak, TextCursorStyle, TextEdit, TextLayout},
    ui::InteractionDisabled,
};
use bevy_widgetry_text_field::{TextField, TextFieldPlugin};

// 只挂载基础输入框标记，验证必需编辑组件及其默认约束被自动补齐。
#[test]
fn text_field_has_expected_default_editing_components() {
    let mut app = App::new();

    let entity = app.world_mut().spawn(TextField).id();

    let world = app.world();

    let editable = world.get::<EditableText>(entity).unwrap();
    let layout = world.get::<TextLayout>(entity).unwrap();

    assert!(!editable.allow_newlines);
    assert_eq!(layout.linebreak, LineBreak::NoWrap);

    assert!(world.get::<TextCursorStyle>(entity).is_some());
}

// 禁用输入框存在待处理用户编辑，验证队列清空且文本保持原值。
#[test]
fn disabled_text_field_discards_queued_edits() {
    let mut app = App::new();
    app.add_plugins(TextFieldPlugin);

    let entity = app.world_mut().spawn((TextField, InteractionDisabled)).id();

    app.world_mut()
        .get_mut::<EditableText>(entity)
        .unwrap()
        .queue_edit(TextEdit::Insert("X".into()));

    assert_eq!(
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .pending_edits
            .len(),
        1
    );

    app.update();

    assert!(
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .pending_edits
            .is_empty()
    );
}

// 未禁用的输入框存在编辑队列，验证拦截系统不清除正常用户输入。
#[test]
fn enabled_text_field_does_not_discard_queued_edits() {
    let mut app = App::new();
    app.add_plugins(TextFieldPlugin);

    let entity = app.world_mut().spawn(TextField).id();

    app.world_mut()
        .get_mut::<EditableText>(entity)
        .unwrap()
        .queue_edit(TextEdit::Insert("X".into()));

    app.update();

    assert_eq!(
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .pending_edits
            .len(),
        1
    );
}

// 直接设置禁用输入框内容，验证仅拦截编辑队列而不回滚程序化赋值。
#[test]
fn disabled_text_field_still_allows_programmatic_value_changes() {
    let mut app = App::new();
    app.add_plugins(TextFieldPlugin);

    let entity = app
        .world_mut()
        .spawn((TextField, EditableText::new("before"), InteractionDisabled))
        .id();

    app.update();

    app.world_mut()
        .get_mut::<EditableText>(entity)
        .unwrap()
        .editor_mut()
        .set_text("after");

    app.update();

    let editable = app.world().get::<EditableText>(entity).unwrap();

    let mut value = String::new();
    for part in editable.value() {
        value.push_str(part);
    }

    assert_eq!(value, "after");
}
