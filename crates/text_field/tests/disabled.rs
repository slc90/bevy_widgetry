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
        let mut editable = {
            let editor = support::editor_entity(app.world(), entity);
            app.world_mut().get_mut::<EditableText>(editor)
        }
        .unwrap();
        editable.queue_edit(TextEdit::Insert("paste".into()));
        editable.pending_paste = Some(ClipboardRead::Ready(Ok("pending".into())));
    }
    let disabled = support::editor_entity(app.world(), disabled);
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

    {
        let editor = support::editor_entity(app.world(), entity);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .queue_edit(TextEdit::Insert("X".into()));

    assert_eq!(
        support::editor(app.world(), entity)
            .unwrap()
            .pending_edits
            .len(),
        1
    );

    app.update();

    assert!(
        support::editor(app.world(), entity)
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

    {
        let editor = support::editor_entity(app.world(), entity);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .queue_edit(TextEdit::Insert("X".into()));
    {
        let editor = support::editor_entity(app.world(), entity);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .pending_paste = Some(ClipboardRead::Ready(Ok("pending".into())));

    app.update();

    assert_eq!(
        support::editor(app.world(), entity)
            .unwrap()
            .pending_edits
            .len(),
        1
    );
    assert!(
        support::editor(app.world(), entity)
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
        .spawn_scene(
            bsn! { @WidgetryTextField { @value: {("before").to_owned()} } InteractionDisabled },
        )
        .unwrap()
        .id();

    app.update();

    WidgetryTextField::set_text_in_world(app.world_mut(), entity, "after").unwrap();

    app.update();

    let editable = support::editor(app.world(), entity).unwrap();

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
        .spawn_scene(bsn! { @WidgetryTextField { @value: {("base").to_owned()} } })
        .unwrap()
        .id();
    let ancestor = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(root).insert(ChildOf(ancestor));
    let bare = app
        .world_mut()
        .spawn_scene(bsn! { ~{EditableText::new("bare")} })
        .unwrap()
        .id();
    app.update();
    {
        let editor = support::editor_entity(app.world(), root);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .queue_edit(TextEdit::Insert("A".into()));
    app.update();
    assert_eq!(
        support::editor(app.world(), root)
            .unwrap()
            .value()
            .to_string(),
        "baseA"
    );
    app.world_mut()
        .entity_mut(ancestor)
        .insert(InteractionDisabled);
    for entity in [root, bare] {
        let mut edit = {
            let editor = support::editor_entity(app.world(), entity);
            app.world_mut().get_mut::<EditableText>(editor)
        }
        .unwrap();
        edit.queue_edit(TextEdit::Insert("OLD".into()));
        edit.pending_paste = Some(ClipboardRead::Ready(Ok("PASTE".into())));
    }
    app.update();
    let edit = support::editor(app.world(), root).unwrap();
    assert_eq!(edit.value().to_string(), "baseA");
    assert!(edit.pending_edits.is_empty());
    assert!(edit.pending_paste.is_none());
    assert_eq!(
        support::editor(app.world(), bare)
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
        support::editor(app.world(), root)
            .unwrap()
            .value()
            .to_string(),
        "baseA"
    );
    {
        let editor = support::editor_entity(app.world(), root);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .queue_edit(TextEdit::Insert("NEW".into()));
    app.update();
    assert_eq!(
        support::editor(app.world(), root)
            .unwrap()
            .value()
            .to_string(),
        "baseANEW"
    );
    assert!(
        support::editor(app.world(), root)
            .unwrap()
            .pending_edits
            .is_empty()
    );
}
