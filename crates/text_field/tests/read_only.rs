#![cfg(test)]

use bevy::{
    clipboard::ClipboardRead,
    input::keyboard::{Key, KeyCode, KeyboardInput},
    input_focus::{
        FocusCause, InputFocus, InputFocusSystems, dispatch_focused_input,
        tab_navigation::TabNavigationPlugin,
    },
    prelude::*,
    text::{EditableText, EditableTextSystems, TextEdit},
    ui::InteractionDisabled,
    window::PrimaryWindow,
};
use bevy_widgetry_test_utils::{primary_press, scene_app, text_input_app};
use bevy_widgetry_text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};

// 同页存在 ReadOnly 时，官方 keyboard input 仍须送达获得 focus 的普通 TextField。
#[test]
fn normal_text_field_keeps_keyboard_input_beside_read_only() {
    let mut app = text_input_app();
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
    assert!(
        app.world()
            .get::<EditableText>(normal)
            .unwrap()
            .pending_edits
            .contains(&TextEdit::Insert("x".into()))
    );
}

// Gallery 装配 TabNavigationPlugin 时，鼠标 press 仍须让两种 TextField 保持 focus。
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

// Disabled 仍由 Bevy 的 AcquireFocus 路径清除 focus，不能被 TextField 拦截重新获得 focus。
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

// ReadOnly 在官方编辑阶段前过滤所有 mutation，保留复制、导航和鼠标 selection command。
#[test]
fn read_only_filters_mutations_and_keeps_navigation() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryReadOnlyTextField template_value(EditableText::new("before"))
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

// Disabled 覆盖 ReadOnly，全部 command 和异步 paste 都在官方编辑前清空。
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

// 只读身份不阻止程序直接修改 EditableText 的内容。
#[test]
fn read_only_allows_programmatic_changes() {
    let mut app = scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryReadOnlyTextField template_value(EditableText::new("before"))
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
