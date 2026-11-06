#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod support;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};
use bevy_widgetry_text_field::{
    WidgetryTextFieldEditor, WidgetryTextFieldHeadless, WidgetryTextFieldHeadlessPlugin,
};
#[test]
fn public_binding_supports_nested_editor_without_default_frame_or_style() {
    let mut app = support::editing_app();
    app.add_plugins(WidgetryTextFieldHeadlessPlugin);
    let root=app.world_mut().spawn_scene(bsn! {
        #Root @WidgetryTextFieldHeadless {editor:#Editor}
        Node {width:px(300)}
        Children [
            Node {padding:UiRect::all(px(7))}
            Children [
                Node {flex_direction:FlexDirection::Column}
                Children [
                    #Editor @WidgetryTextFieldEditor {root:#Root,@value:{"a👨‍👩‍👧‍👦b".to_owned()},@max_length:{Some(2)}}
                    Node {min_width:px(0)}
                ]
            ]
        ]
    }).unwrap().id();
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        "a👨‍👩‍👧‍👦"
    );
    assert!(
        WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "e\u{301}中x").unwrap()
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        "e\u{301}中"
    );
    WidgetryTextFieldHeadless::focus_in_world(app.world_mut(), root).unwrap();
    let focused = app.world().resource::<InputFocus>().get().unwrap();
    assert_ne!(focused, root);
    assert!(app.world().get::<EditableText>(focused).is_some());
    assert_eq!(
        app.world()
            .get::<WidgetryTextFieldEditor>(focused)
            .unwrap()
            .root,
        root
    );
}

fn nested(app: &mut App, value: &str, max: Option<usize>, read_only: bool) -> Entity {
    let value = value.to_owned();
    app.world_mut()
        .spawn_scene(bsn! {
            #Root @WidgetryTextFieldHeadless {editor:#Editor,@read_only:read_only}
            bevy::input_focus::tab_navigation::TabIndex(0)
            Node {width:px(300)}
            Children [
                Node {padding:UiRect::all(px(8))}
                Children [
                    Node {flex_direction:FlexDirection::Column}
                    Children [
                        #Editor @WidgetryTextFieldEditor {root:#Root,@value:value,@max_length:max}
                        Node {min_width:px(0),height:px(36)}--
                        Node bevy::input_focus::tab_navigation::TabIndex(0)
                    ]
                ]
            ]
        })
        .unwrap()
        .id()
}
fn update(app: &mut App) {
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
}
fn edit(app: &mut App, root: Entity, edits: impl IntoIterator<Item = bevy::text::TextEdit>) {
    let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), root).unwrap();
    let mut editable = app.world_mut().get_mut::<EditableText>(editor).unwrap();
    for request in edits {
        editable.queue_edit(request);
    }
}
#[test]
fn nested_editing_preserves_grapheme_limits_ime_and_disabled_readonly_semantics() {
    use bevy::{text::TextEdit, ui::InteractionDisabled};
    let mut app = support::editing_app();
    app.add_plugins(WidgetryTextFieldHeadlessPlugin);
    let root = nested(&mut app, "ab", Some(2), false);
    let readonly = nested(&mut app, "copy", None, true);
    let zero = nested(&mut app, "👨‍👩‍👧‍👦", Some(0), false);
    update(&mut app);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), zero).unwrap(),
        ""
    );
    assert!(WidgetryTextFieldHeadless::is_read_only(app.world(), readonly).unwrap());
    edit(
        &mut app,
        root,
        [TextEdit::SelectAll, TextEdit::Insert("e\u{301}🇦🇧x".into())],
    );
    update(&mut app);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        "e\u{301}🇦🇧"
    );
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "").unwrap();
    edit(
        &mut app,
        root,
        [TextEdit::ImeSetCompose {
            value: "长预编辑，不裁剪".into(),
            cursor: None,
        }],
    );
    update(&mut app);
    assert!(
        WidgetryTextFieldHeadless::editor(app.world(), root)
            .unwrap()
            .is_composing()
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        ""
    );
    edit(
        &mut app,
        root,
        [TextEdit::ImeCommit {
            value: "👨‍👩‍👧‍👦e\u{301}x".into(),
        }],
    );
    update(&mut app);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        "👨‍👩‍👧‍👦e\u{301}"
    );
    edit(
        &mut app,
        readonly,
        [
            TextEdit::SelectAll,
            TextEdit::Insert("bad".into()),
            TextEdit::Backspace,
        ],
    );
    update(&mut app);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), readonly).unwrap(),
        "copy"
    );
    assert!(
        !WidgetryTextFieldHeadless::editor(app.world(), readonly)
            .unwrap()
            .editor()
            .raw_selection()
            .text_range()
            .is_empty()
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    edit(
        &mut app,
        root,
        [TextEdit::SelectAll, TextEdit::Insert("stale".into())],
    );
    let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), root).unwrap();
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Ready(Ok("stale".into())));
    update(&mut app);
    assert!(
        WidgetryTextFieldHeadless::editor(app.world(), root)
            .unwrap()
            .pending_edits
            .is_empty()
    );
    assert!(
        WidgetryTextFieldHeadless::editor(app.world(), root)
            .unwrap()
            .pending_paste
            .is_none()
    );
    assert!(WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), root, "新正文").unwrap());
    assert!(
        WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), readonly, "new").unwrap()
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    update(&mut app);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        "新正"
    );
}
#[test]
fn bindings_reject_shared_foreign_stale_and_detached_editors_without_writes() {
    use bevy_widgetry_text_field::{TextFieldError, WidgetryTextFieldHeadless as Root};
    let mut app = support::editing_app();
    app.add_plugins(WidgetryTextFieldHeadlessPlugin);
    let a = nested(&mut app, "A", None, false);
    let b = nested(&mut app, "B", None, false);
    let editor = Root::editor_entity(app.world(), a).unwrap();
    let own = Root::editor_entity(app.world(), b).unwrap();
    assert_eq!(
        WidgetryTextFieldEditor::owner(app.world(), editor).unwrap(),
        a
    );
    app.world_mut().entity_mut(b).insert(Root { editor });
    assert!(
        matches!(Root::set_text_in_world(app.world_mut(),b,"bad").unwrap_err().kind(),TextFieldError::InvalidEditorOwner {root,actual,..} if *root==b&&*actual==a)
    );
    assert_eq!(Root::text(app.world(), a).unwrap(), "A");
    app.world_mut().entity_mut(b).insert(Root { editor: own });
    app.world_mut().entity_mut(own).remove::<ChildOf>();
    assert!(
        matches!(Root::text(app.world(),b).unwrap_err().kind(),TextFieldError::EditorOutsideRoot {root,editor} if *root==b&&*editor==own)
    );
    app.world_mut().despawn(own);
    assert!(
        matches!(Root::text(app.world(),b).unwrap_err().kind(),TextFieldError::MissingPart {entity,component:"WidgetryTextFieldEditor",..} if *entity==own)
    );
    assert_eq!(Root::text(app.world(), a).unwrap(), "A");
}
#[test]
fn root_focus_mapping_preserves_deep_child_focus_and_local_editor_configuration() {
    use bevy::input_focus::{AcquireFocus, tab_navigation::TabIndex};
    let mut app = support::editing_app();
    app.add_plugins(WidgetryTextFieldHeadlessPlugin);
    let root = nested(&mut app, "focus", None, false);
    let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), root).unwrap();
    app.world_mut().entity_mut(editor).insert(TextFont {
        font_size: FontSize::Px(23.0),
        ..default()
    });
    update(&mut app);
    assert_eq!(
        app.world().get::<TextFont>(editor).unwrap().font_size,
        FontSize::Px(23.0)
    );
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let button = app
        .world_mut()
        .query_filtered::<Entity, (With<TabIndex>, Without<WidgetryTextFieldHeadless>)>()
        .iter(app.world())
        .next()
        .unwrap();
    app.world_mut().trigger(AcquireFocus {
        focused_entity: button,
        window,
    });
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    app.world_mut().trigger(AcquireFocus {
        focused_entity: root,
        window,
    });
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    app.world_mut().entity_mut(root).insert(TextFont {
        font_size: FontSize::Px(37.0),
        ..default()
    });
    update(&mut app);
    assert_eq!(
        app.world().get::<TextFont>(editor).unwrap().font_size,
        FontSize::Px(37.0)
    );
}

#[test]
fn nested_keyboard_routes_to_official_editor_and_default_plugin_does_not_style_it() {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        text::TextCursorStyle,
    };
    let mut app = support::editing_app();
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryTextFieldHeadlessPlugin);
    let root = nested(&mut app, "", Some(2), false);
    let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), root).unwrap();
    let color = Color::srgb(0.2, 0.7, 0.3);
    app.world_mut()
        .entity_mut(editor)
        .insert((TextColor(color), TextCursorStyle { color, ..default() }));
    app.add_plugins(bevy_widgetry_text_field::WidgetryTextFieldPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    update(&mut app);
    WidgetryTextFieldHeadless::focus_in_world(app.world_mut(), root).unwrap();
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character("甲e\u{301}乙".into()),
        state: ButtonState::Pressed,
        text: Some("甲e\u{301}乙".into()),
        repeat: false,
        window,
    });
    update(&mut app);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        "甲e\u{301}"
    );
    assert_eq!(
        WidgetryTextFieldHeadless::max_length(app.world(), root).unwrap(),
        Some(2)
    );
    assert_eq!(app.world().get::<TextColor>(editor).unwrap().0, color);
    assert_eq!(
        app.world().get::<TextCursorStyle>(editor).unwrap().color,
        color
    );
    assert!(
        app.world()
            .get::<bevy_widgetry_text_field::WidgetryTextField>(root)
            .is_none()
    );
}
#[test]
fn queued_api_validates_the_current_owner_and_keeps_other_instances_working() {
    use bevy_widgetry_test_utils::ErrorCapture;
    let mut app = support::editing_app();
    app.add_plugins(WidgetryTextFieldHeadlessPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let a = nested(&mut app, "A", None, false);
    let b = nested(&mut app, "B", None, false);
    let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), a).unwrap();
    {
        let mut commands = app.world_mut().commands();
        commands.queue(move |world: &mut World| {
            world
                .entity_mut(editor)
                .insert(WidgetryTextFieldEditor { root: b });
        });
        WidgetryTextFieldHeadless::set_text(&mut commands, a, "bad");
        WidgetryTextFieldHeadless::set_text(&mut commands, b, "good");
    }
    let errors = ErrorCapture::default();
    errors.run(|| app.world_mut().flush());
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    assert!(records[0].managed().is_some());
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .value()
            .to_string(),
        "A"
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), b).unwrap(),
        "good"
    );
}

#[test]
fn native_clipboard_paste_and_readonly_copy_work_through_nested_bindings() {
    use bevy::{clipboard::Clipboard, text::TextEdit};
    let mut app = support::editing_app();
    app.add_plugins(WidgetryTextFieldHeadlessPlugin);
    let root = nested(&mut app, "old", Some(2), false);
    let readonly = nested(&mut app, "copy", None, true);
    update(&mut app);
    let previous = app
        .world_mut()
        .resource_mut::<Clipboard>()
        .fetch_text()
        .poll_result();
    app.world_mut()
        .resource_mut::<Clipboard>()
        .set_text("甲e\u{301}乙")
        .unwrap();
    edit(&mut app, root, [TextEdit::SelectAll, TextEdit::Paste]);
    update(&mut app);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), root).unwrap(),
        "甲e\u{301}"
    );
    edit(&mut app, readonly, [TextEdit::SelectAll, TextEdit::Copy]);
    update(&mut app);
    assert_eq!(
        app.world_mut()
            .resource_mut::<Clipboard>()
            .fetch_text()
            .poll_result()
            .unwrap()
            .unwrap(),
        "copy"
    );
    if let Some(Ok(value)) = previous {
        app.world_mut()
            .resource_mut::<Clipboard>()
            .set_text(value)
            .unwrap();
    }
}
