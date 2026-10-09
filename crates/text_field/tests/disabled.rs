//! State：enabled/disabled、文本/selection、pending edit/paste 与普通 EditableText 对照。
//! Stimuli：keyboard input、EditCommand、paste 与程序化 set_text。
//! Guards：disabled 时在 Bevy 编辑阶段前丢弃全部用户 mutation。
//! Transitions：enabled → disabled → enabled。
//! 禁用期间程序修改保留，恢复后只消费新输入。
//! Invariants：旧 edit/paste 不重放。
//! 过滤 queue 与消费后文本分别验证。
//! Couplings：disabled 限制用户编辑，保留程序化内容和恢复后的编辑能力。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

mod support;

use bevy::clipboard::ClipboardRead;
use bevy::text::EditableTextSystems;
use bevy::{
    prelude::*,
    text::{EditableText, TextEdit},
    ui::InteractionDisabled,
};
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};
use support::editing_app;

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

#[test]
fn disabled_recovery_consumes_only_new_edits() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField template_value(EditableText::new("base")) })
        .unwrap()
        .id();
    let ancestor = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(root).insert(ChildOf(ancestor));
    let bare = app
        .world_mut()
        .spawn_scene(bsn! { template_value(EditableText::new("bare")) })
        .unwrap()
        .id();
    app.update();
    app.world_mut()
        .get_mut::<EditableText>(root)
        .unwrap()
        .queue_edit(TextEdit::Insert("A".into()));
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(root)
            .unwrap()
            .value()
            .to_string(),
        "baseA"
    );
    app.world_mut()
        .entity_mut(ancestor)
        .insert(InteractionDisabled);
    for entity in [root, bare] {
        let mut edit = app.world_mut().get_mut::<EditableText>(entity).unwrap();
        edit.queue_edit(TextEdit::Insert("OLD".into()));
        edit.pending_paste = Some(ClipboardRead::Ready(Ok("PASTE".into())));
    }
    app.update();
    let edit = app.world().get::<EditableText>(root).unwrap();
    assert_eq!(edit.value().to_string(), "baseA");
    assert!(edit.pending_edits.is_empty());
    assert!(edit.pending_paste.is_none());
    assert_eq!(
        app.world()
            .get::<EditableText>(bare)
            .unwrap()
            .value()
            .to_string(),
        "barePASTEOLD"
    );
    app.world_mut()
        .entity_mut(ancestor)
        .remove::<InteractionDisabled>();
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(root)
            .unwrap()
            .value()
            .to_string(),
        "baseA"
    );
    app.world_mut()
        .get_mut::<EditableText>(root)
        .unwrap()
        .queue_edit(TextEdit::Insert("NEW".into()));
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(root)
            .unwrap()
            .value()
            .to_string(),
        "baseANEW"
    );
    assert!(
        app.world()
            .get::<EditableText>(root)
            .unwrap()
            .pending_edits
            .is_empty()
    );
}
