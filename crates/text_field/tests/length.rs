#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod support;

use bevy::{
    prelude::*,
    text::{EditableText, EditableTextSystems, TextEdit, TextEditChange},
};
use bevy_widgetry_text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};
use support::editing_app;

#[test]
fn programmatic_replacement_during_hidden_preedit_restores_the_official_caret() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = field(&mut app, "甲", 2);
    app.update();
    edit(
        &mut app,
        entity,
        [TextEdit::ImeSetCompose {
            value: "输入法预编辑".into(),
            cursor: None,
        }],
    );
    assert!(
        support::editor(app.world(), entity)
            .unwrap()
            .editor()
            .cursor_geometry(1.0)
            .is_none()
    );
    WidgetryTextField::set_text_in_world(app.world_mut(), entity, "新正文").unwrap();
    let editor = support::editor(app.world(), entity).unwrap();
    assert!(!editor.is_composing());
    assert!(editor.editor().cursor_geometry(1.0).is_some());
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "新正"
    );
    edit(&mut app, entity, [TextEdit::TextEnd(false)]);
    assert!(
        support::editor(app.world(), entity)
            .unwrap()
            .editor()
            .cursor_geometry(1.0)
            .is_some()
    );
}

#[test]
fn keyboard_and_ime_messages_reach_the_guard_and_plain_editors_remain_unconstrained() {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        input_focus::{FocusCause, InputFocus},
    };
    let mut app = editing_app();
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryTextFieldPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let entity = field(&mut app, "", 1);
    let bare = app
        .world_mut()
        .spawn_scene(bsn! { EditableText })
        .unwrap()
        .id();
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character("甲乙".into()),
        text: Some("甲乙".into()),
        state: ButtonState::Pressed,
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), entity).unwrap(), "甲");
    app.world_mut().write_message(bevy::window::Ime::Preedit {
        window,
        value: "组合".into(),
        cursor: Some((0, 6)),
    });
    app.update();
    assert!(support::editor(app.world(), entity).unwrap().is_composing());
    app.world_mut().write_message(bevy::window::Ime::Commit {
        window,
        value: "乙".into(),
    });
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), entity).unwrap(), "甲");
    edit(&mut app, bare, [TextEdit::Insert("自由输入".into())]);
    assert_eq!(
        support::editor(app.world(), bare)
            .unwrap()
            .value()
            .to_string(),
        "自由输入"
    );
}

#[test]
fn conflicting_filter_errors_without_letting_other_limited_fields_bypass_the_guard() {
    use bevy::{
        ecs::{error::Severity, schedule::SingleThreadedExecutor},
        text::EditableTextFilter,
    };
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.edit_schedule(PostUpdate, |s| {
        s.set_executor(SingleThreadedExecutor::default());
    });
    let conflicting = field(&mut app, "", 1);
    let other = field(&mut app, "", 1);
    app.update();
    app.world_mut()
        .entity_mut(conflicting)
        .insert(EditableTextFilter::new(|_| false));
    for entity in [conflicting, other] {
        {
            let editor = support::editor_entity(app.world(), entity);
            app.world_mut().get_mut::<EditableText>(editor)
        }
        .unwrap()
        .queue_edit(TextEdit::Insert("甲乙".into()));
    }
    let capture = bevy_widgetry_test_utils::ErrorCapture::default();
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    capture.run(|| app.update());
    let errors = capture.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), Severity::Error);
    assert_eq!(
        WidgetryTextField::text(app.world(), conflicting).unwrap(),
        ""
    );
    assert_eq!(WidgetryTextField::text(app.world(), other).unwrap(), "甲");
}

#[test]
fn ime_preedit_remains_unlimited_and_only_the_commit_is_constrained() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = field(&mut app, "甲", 2);
    app.update();
    edit(
        &mut app,
        entity,
        [TextEdit::ImeSetCompose {
            value: "中文输入预编辑".into(),
            cursor: None,
        }],
    );
    assert!(support::editor(app.world(), entity).unwrap().is_composing());
    assert_eq!(WidgetryTextField::text(app.world(), entity).unwrap(), "甲");
    edit(&mut app, entity, [TextEdit::Insert("旧键盘消息".into())]);
    assert!(support::editor(app.world(), entity).unwrap().is_composing());
    assert_eq!(WidgetryTextField::text(app.world(), entity).unwrap(), "甲");
    edit(
        &mut app,
        entity,
        [TextEdit::ImeCommit {
            value: "乙丙".into(),
        }],
    );
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "甲乙"
    );
    assert!(!support::editor(app.world(), entity).unwrap().is_composing());
    edit(
        &mut app,
        entity,
        [
            TextEdit::SelectAll,
            TextEdit::ImeCommit {
                value: "一二三".into(),
            },
        ],
    );
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "一二"
    );
}

#[test]
fn native_clipboard_is_read_once_in_queue_order_and_pending_paste_precedes_later_edits() {
    use bevy::clipboard::{Clipboard, ClipboardRead};
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = field(&mut app, "甲乙", 2);
    app.update();
    let old = app
        .world_mut()
        .resource_mut::<Clipboard>()
        .fetch_text()
        .poll_result();
    app.world_mut()
        .resource_mut::<Clipboard>()
        .set_text("丙丁戊")
        .unwrap();
    edit(
        &mut app,
        entity,
        [
            TextEdit::SelectAll,
            TextEdit::Paste,
            TextEdit::SelectAll,
            TextEdit::Copy,
        ],
    );
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "丙丁"
    );
    assert_eq!(
        app.world_mut()
            .resource_mut::<Clipboard>()
            .fetch_text()
            .poll_result()
            .unwrap()
            .unwrap(),
        "丙丁"
    );
    {
        let editor = support::editor_entity(app.world(), entity);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .pending_paste = Some(ClipboardRead::Ready(Ok("一二三".into())));
    edit(
        &mut app,
        entity,
        [
            TextEdit::TextStart(false),
            TextEdit::Delete,
            TextEdit::Insert("新".into()),
        ],
    );
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "新二"
    );
    {
        let editor = support::editor_entity(app.world(), entity);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .pending_paste = Some(ClipboardRead::Ready(Err(
        bevy::clipboard::ClipboardError::ContentNotAvailable,
    )));
    edit(
        &mut app,
        entity,
        [TextEdit::SelectAll, TextEdit::Insert("末".into())],
    );
    assert_eq!(WidgetryTextField::text(app.world(), entity).unwrap(), "末");
    if let Some(Ok(text)) = old {
        app.world_mut()
            .resource_mut::<Clipboard>()
            .set_text(text)
            .unwrap();
    }
}

#[test]
fn setter_preserves_legal_selection_clears_old_work_and_validates_at_command_execution() {
    use bevy::clipboard::ClipboardRead;
    use bevy::ecs::error::Severity;
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = field(&mut app, "abcd", 4);
    app.update();
    edit(
        &mut app,
        entity,
        [TextEdit::TextStart(false), TextEdit::Right(true)],
    );
    {
        let editor = support::editor_entity(app.world(), entity);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .pending_paste = Some(ClipboardRead::Ready(Ok("stale".into())));
    {
        let editor = support::editor_entity(app.world(), entity);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .queue_edit(TextEdit::Insert("stale".into()));
    assert!(WidgetryTextField::set_text_in_world(app.world_mut(), entity, "wxyz").unwrap());
    let editor = support::editor(app.world(), entity).unwrap();
    assert_eq!(editor.editor().raw_selection().text_range(), 0..1);
    assert!(editor.pending_paste.is_none());
    assert!(editor.pending_edits.is_empty());
    app.world_mut().commands().queue(move |world: &mut World| {
        let mut commands = world.commands();
        WidgetryTextField::set_text(&mut commands, entity, "好长的内容");
    });
    app.world_mut().flush();
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "好长的内"
    );
    let invalid = app.world_mut().spawn_empty().id();
    assert_eq!(
        WidgetryTextField::set_text_in_world(app.world_mut(), invalid, "bad")
            .unwrap_err()
            .severity(),
        Severity::Error
    );
    assert_eq!(
        WidgetryReadOnlyTextField::text(app.world(), entity)
            .unwrap_err()
            .severity(),
        Severity::Error
    );
    let capture = bevy_widgetry_test_utils::ErrorCapture::default();
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    WidgetryTextField::set_text(&mut app.world_mut().commands(), invalid, "bad");
    capture.run(|| app.world_mut().flush());
    let errors = capture.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), Severity::Error);
}

fn field(app: &mut App, value: &str, max: usize) -> Entity {
    let value = value.to_owned();
    app.world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField { @value: value, @max_length: {Some(max)} }
        })
        .unwrap()
        .id()
}

fn edit(app: &mut App, entity: Entity, edits: impl IntoIterator<Item = TextEdit>) {
    {
        let editor = support::editor_entity(app.world(), entity);
        app.world_mut().get_mut::<EditableText>(editor)
    }
    .unwrap()
    .pending_edits
    .extend(edits);
    app.update();
}

#[test]
fn initial_and_programmatic_text_use_whole_graphemes() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    for grapheme in ["中", "e\u{301}", "👍🏽", "🇨🇳", "👨‍👩‍👧‍👦"] {
        let entity = field(&mut app, &format!("{grapheme}尾"), 1);
        assert_eq!(
            WidgetryTextField::text(app.world(), entity).unwrap(),
            grapheme
        );
        app.update();
        assert!(
            WidgetryTextField::set_text_in_world(
                app.world_mut(),
                entity,
                format!("{grapheme}extra")
            )
            .is_ok()
        );
        assert_eq!(
            WidgetryTextField::text(app.world(), entity).unwrap(),
            grapheme
        );
        assert!(!WidgetryTextField::set_text_in_world(app.world_mut(), entity, grapheme).unwrap());
    }
    let zero = field(&mut app, "非空", 0);
    assert_eq!(WidgetryTextField::text(app.world(), zero).unwrap(), "");
}

#[test]
fn edits_observe_prior_selection_and_deletion_before_constraining_inserts() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = field(&mut app, "甲乙", 2);
    app.update();
    edit(
        &mut app,
        entity,
        [TextEdit::SelectAll, TextEdit::Insert("丙丁戊".into())],
    );
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "丙丁"
    );
    edit(
        &mut app,
        entity,
        [
            TextEdit::TextStart(false),
            TextEdit::Delete,
            TextEdit::Insert("一二".into()),
        ],
    );
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "一丁"
    );
    let edit = support::editor(app.world(), entity).unwrap();
    assert!(edit.pending_edits.is_empty());
    assert!(edit.editor().raw_selection().text_range().end <= edit.value().to_string().len());
}

#[test]
fn insertion_counts_the_trial_replacement_including_boundary_merges() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = field(&mut app, "e", 1);
    app.update();
    edit(&mut app, entity, [TextEdit::Insert("\u{301}x".into())]);
    assert_eq!(
        WidgetryTextField::text(app.world(), entity).unwrap(),
        "e\u{301}"
    );
}

#[derive(Resource, Default)]
struct Observed(Vec<String>);

#[test]
fn official_notifications_and_render_boundary_only_observe_constrained_text() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin)
        .init_resource::<Observed>();
    app.add_observer(
        |change: On<TextEditChange>, query: Query<&EditableText>, mut values: ResMut<Observed>| {
            values.0.push(
                query
                    .get(change.event_target())
                    .unwrap()
                    .value()
                    .to_string(),
            );
        },
    );
    let entity = field(&mut app, "", 2);
    app.update();
    app.world_mut().resource_mut::<Observed>().0.clear();
    let editor = support::editor_entity(app.world(), entity);
    app.add_systems(
        PostUpdate,
        (move |query: Query<&EditableText>| {
            assert!(
                query
                    .get(editor)
                    .unwrap()
                    .value()
                    .to_string()
                    .chars()
                    .count()
                    <= 2
            );
        })
        .after(EditableTextSystems),
    );
    edit(&mut app, entity, [TextEdit::Insert("甲乙丙".into())]);
    assert_eq!(app.world().resource::<Observed>().0, ["甲乙"]);
}

#[test]
fn read_only_and_disabled_still_allow_constrained_programmatic_values() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryReadOnlyTextField { @max_length: {Some(1)} }
            bevy::ui::InteractionDisabled
        })
        .unwrap()
        .id();
    assert!(WidgetryReadOnlyTextField::set_text_in_world(app.world_mut(), entity, "甲乙").unwrap());
    edit(&mut app, entity, [TextEdit::Insert("丙".into())]);
    assert_eq!(
        WidgetryReadOnlyTextField::text(app.world(), entity).unwrap(),
        "甲"
    );
}
