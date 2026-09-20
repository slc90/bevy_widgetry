#![cfg(test)]

use bevy::clipboard::ClipboardRead;
use bevy::text::EditableTextSystems;
use bevy::{
    prelude::*,
    text::{EditableText, TextEdit},
    ui::InteractionDisabled,
};
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};

// disabled 兼容逻辑必须在官方编辑阶段前清除 paste 和 queue，且不触及裸 EditableText。
#[test]
fn workaround_is_scoped_and_runs_before_official_editing() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let disabled = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField InteractionDisabled })
        .unwrap()
        .id();
    let bare = app
        .world_mut()
        .spawn_scene(bsn! { EditableText InteractionDisabled })
        .unwrap()
        .id();
    for entity in [disabled, bare] {
        let mut editable = app.world_mut().get_mut::<EditableText>(entity).unwrap();
        editable.queue_edit(TextEdit::Insert("paste".into()));
        editable.pending_paste = Some(ClipboardRead::Ready(Ok("pending".into())));
    }
    app.add_systems(
        PostUpdate,
        (move |query: Query<&EditableText>| {
            let editable = query.get(disabled).unwrap();
            assert!(editable.pending_edits.is_empty());
            assert!(editable.pending_paste.is_none());
            let editable = query.get(bare).unwrap();
            assert_eq!(editable.pending_edits.len(), 1);
            assert!(editable.pending_paste.is_some());
        })
        .in_set(EditableTextSystems),
    );
    app.update();
}

// disabled TextField 存在待处理用户编辑，验证 queue 清空且文本保持原值。
#[test]
fn disabled_text_field_discards_queued_edits() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);

    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField InteractionDisabled })
        .unwrap()
        .id();

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

// 未禁用的 TextField 存在编辑 queue，验证拦截 system 不清除正常用户输入。
#[test]
fn enabled_text_field_does_not_discard_queued_edits() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);

    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();

    app.world_mut()
        .get_mut::<EditableText>(entity)
        .unwrap()
        .queue_edit(TextEdit::Insert("X".into()));
    app.world_mut()
        .get_mut::<EditableText>(entity)
        .unwrap()
        .pending_paste = Some(ClipboardRead::Ready(Ok("pending".into())));

    app.update();

    assert_eq!(
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .pending_edits
            .len(),
        1
    );
    assert!(
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .pending_paste
            .is_some()
    );
}

// 直接设置 disabled TextField 内容，验证仅拦截编辑 queue 而不回滚程序化赋值。
#[test]
fn disabled_text_field_still_allows_programmatic_value_changes() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);

    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField template_value(EditableText::new("before")) InteractionDisabled }).unwrap()
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
