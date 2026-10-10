//! State：ReadOnly/普通 TextField、enabled/disabled、focus 与文本/selection。
//! Stimuli：EditCommand、keyboard/pointer input、异步 paste 与程序化内容更新。
//! Guards：ReadOnly 过滤 mutation，disabled 过滤全部 command/paste。
//! Transitions：navigation/selection/copy 保留，mutation 被拒绝。
//! 程序修改仍可更新内容。
//! Invariants：Bevy 消费后 ReadOnly 内容保持，允许的 selection 真正改变选区。
//! Couplings：disabled 覆盖 ReadOnly。
//! copy/IME 过滤覆盖不包含 OS clipboard 或 native IME 验收。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

mod support;

use bevy::{
    clipboard::ClipboardRead,
    input::keyboard::{Key, KeyCode, KeyboardInput},
    input_focus::tab_navigation::TabIndex,
    input_focus::{
        AcquireFocus, FocusCause, InputFocus, InputFocusSystems, dispatch_focused_input,
        tab_navigation::TabNavigationPlugin,
    },
    prelude::*,
    text::{EditableText, EditableTextSystems, TextEdit},
    ui::InteractionDisabled,
    ui_widgets::SelectAllOnFocus,
    window::PrimaryWindow,
};
use bevy_widgetry_test_utils::{primary_press, scene_app, text_input_app};
use bevy_widgetry_text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};
use support::editing_app;

#[test]
fn normal_text_field_keeps_keyboard_input_beside_read_only() {
    let mut app = editing_app();
    app.add_message::<KeyboardInput>()
        .add_systems(
            PreUpdate,
            dispatch_focused_input::<KeyboardInput>.in_set(InputFocusSystems::Dispatch),
        )
        .add_plugins(WidgetryTextFieldPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let normal = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    app.world_mut()
        .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
        .unwrap();
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(normal, FocusCause::Pressed);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyX,
        logical_key: Key::Character("x".into()),
        state: bevy::input::ButtonState::Pressed,
        text: Some("x".into()),
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(normal)
            .unwrap()
            .value()
            .to_string(),
        "x"
    );
    assert!(
        app.world()
            .get::<EditableText>(normal)
            .unwrap()
            .pending_edits
            .is_empty()
    );
}

#[test]
fn pointer_focus_survives_tab_navigation_for_both_text_fields() {
    for read_only in [false, true] {
        let mut app = text_input_app();
        app.add_plugins((TabNavigationPlugin, WidgetryTextFieldPlugin));
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        let entity = if read_only {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
                .unwrap()
                .id()
        } else {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryTextField })
                .unwrap()
                .id()
        };
        app.update();
        app.world_mut().trigger(primary_press(entity));
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(entity));
    }
}

#[test]
fn programmatic_acquire_focus_reaches_focusable_parent() {
    for read_only in [false, true] {
        let mut app = text_input_app();
        app.add_plugins((TabNavigationPlugin, WidgetryTextFieldPlugin));
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let parent = app.world_mut().spawn((Node::default(), TabIndex(0))).id();
        let entity = if read_only {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
                .unwrap()
                .id()
        } else {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryTextField })
                .unwrap()
                .id()
        };
        app.world_mut().entity_mut(parent).add_child(entity);
        app.update();
        app.world_mut().trigger(AcquireFocus {
            focused_entity: entity,
            window,
        });
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(parent));
    }
}

#[test]
fn programmatic_acquire_focus_keeps_official_navigation_cause() {
    for read_only in [false, true] {
        let mut app = text_input_app();
        app.add_plugins((TabNavigationPlugin, WidgetryTextFieldPlugin));
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let entity = if read_only {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
                .unwrap()
                .id()
        } else {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryTextField })
                .unwrap()
                .id()
        };
        app.world_mut()
            .entity_mut(entity)
            .insert((TabIndex(0), SelectAllOnFocus));
        app.update();
        app.world_mut().trigger(AcquireFocus {
            focused_entity: entity,
            window,
        });
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(entity));
        assert!(
            app.world()
                .get::<EditableText>(entity)
                .unwrap()
                .pending_edits
                .contains(&TextEdit::SelectAll)
        );
    }
}

#[test]
fn disabled_text_fields_do_not_retain_pointer_focus() {
    for read_only in [false, true] {
        let mut app = text_input_app();
        app.add_plugins((TabNavigationPlugin, WidgetryTextFieldPlugin));
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        let entity = if read_only {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryReadOnlyTextField InteractionDisabled })
                .unwrap()
                .id()
        } else {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryTextField InteractionDisabled })
                .unwrap()
                .id()
        };
        app.update();
        app.world_mut().trigger(primary_press(entity));
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), None);
    }
}

#[test]
fn read_only_filters_mutations_and_keeps_navigation() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryReadOnlyTextField ~{EditableText::new("before")}
        })
        .unwrap()
        .id();
    let kept = vec![
        TextEdit::Copy,
        TextEdit::SelectAll,
        TextEdit::Left(false),
        TextEdit::MoveToPoint(Vec2::ZERO),
        TextEdit::ExtendSelectionToPoint(Vec2::ONE),
        TextEdit::SelectWordAtPoint(Vec2::ZERO),
        TextEdit::ShiftClickExtension(Vec2::ONE),
    ];
    let editable = &mut app.world_mut().get_mut::<EditableText>(entity).unwrap();
    editable.pending_edits.clear();
    for edit in [
        TextEdit::Cut,
        TextEdit::Paste,
        TextEdit::Insert("X".into()),
        TextEdit::Backspace,
        TextEdit::BackspaceWord,
        TextEdit::Delete,
        TextEdit::DeleteWord,
        TextEdit::ImeSetCompose {
            value: "X".into(),
            cursor: None,
        },
        TextEdit::ImeCommit { value: "X".into() },
    ]
    .into_iter()
    .chain(kept.clone())
    {
        editable.queue_edit(edit);
    }
    editable.pending_paste = Some(ClipboardRead::Ready(Ok("pending".into())));
    app.add_systems(
        PostUpdate,
        (move |query: Query<&EditableText>| {
            let editable = query.get(entity).unwrap();
            assert_eq!(editable.pending_edits, kept);
            assert!(editable.pending_paste.is_none());
            assert_eq!(editable.value().to_string(), "before");
        })
        .in_set(EditableTextSystems),
    );
    app.update();
}

#[test]
fn disabled_read_only_discards_every_command() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryReadOnlyTextField InteractionDisabled
        })
        .unwrap()
        .id();
    let editable = &mut app.world_mut().get_mut::<EditableText>(entity).unwrap();
    editable.queue_edit(TextEdit::Copy);
    editable.queue_edit(TextEdit::SelectAll);
    editable.pending_paste = Some(ClipboardRead::Ready(Ok("pending".into())));
    app.add_systems(
        PostUpdate,
        (move |query: Query<&EditableText>| {
            let editable = query.get(entity).unwrap();
            assert!(editable.pending_edits.is_empty());
            assert!(editable.pending_paste.is_none());
        })
        .in_set(EditableTextSystems),
    );
    app.update();
}

#[test]
fn read_only_allows_programmatic_changes() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryReadOnlyTextField ~{EditableText::new("before")}
        })
        .unwrap()
        .id();
    app.world_mut()
        .get_mut::<EditableText>(entity)
        .unwrap()
        .editor_mut()
        .set_text("after");
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .value()
            .to_string(),
        "after"
    );
}

#[test]
fn read_only_preserves_value_but_consumes_selection() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let normal = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField ~{EditableText::new("original")} })
        .unwrap()
        .id();
    let read_only = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryReadOnlyTextField ~{EditableText::new("original")} })
        .unwrap()
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(read_only)
            .unwrap()
            .editor()
            .raw_selection()
            .text_range(),
        8..8
    );
    for root in [normal, read_only] {
        let mut edit = app.world_mut().get_mut::<EditableText>(root).unwrap();
        edit.queue_edit(TextEdit::Insert("X".into()));
        edit.queue_edit(TextEdit::SelectAll);
    }
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(normal)
            .unwrap()
            .value()
            .to_string(),
        "originalX"
    );
    let edit = app.world().get::<EditableText>(read_only).unwrap();
    assert_eq!(edit.value().to_string(), "original");
    assert_eq!(edit.editor().raw_selection().text_range(), 0..8);
    assert!(edit.pending_edits.is_empty());
}
