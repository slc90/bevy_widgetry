#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod support;

use bevy::{
    prelude::*,
    text::{EditableText, TextEdit},
};
use bevy_widgetry_text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin, WidgetryTextFieldPrefix,
};
use support::editing_app;

fn count(world: &World, root: Entity) -> Option<(Entity, &str)> {
    world
        .get::<Children>(root)?
        .iter()
        .find_map(|e| world.get::<Text>(e).map(|text| (e, text.0.as_str())))
}

fn edit(app: &mut App, root: Entity, edits: impl IntoIterator<Item = TextEdit>) {
    let editor = support::editor_entity(app.world(), root);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_edits
        .extend(edits);
    app.update();
}

#[test]
fn optional_count_formats_formal_graphemes_independently_of_the_limit() {
    for show in [false, true] {
        for max in [None, Some(0), Some(3), Some(100)] {
            let mut app = editing_app();
            app.add_plugins(WidgetryTextFieldPlugin);
            let root = app.world_mut().spawn_scene(bsn! {
                @WidgetryTextField { @show_count: show, @max_length: max, @value: {"中e\u{301}👍🏽🇨🇳👨‍👩‍👧‍👦".to_owned()} }
            }).unwrap().id();
            app.update();
            if !show {
                assert!(count(app.world(), root).is_none());
                continue;
            }
            let expected = match max {
                None => "5",
                Some(0) => "0 / 0",
                Some(3) => "3 / 3",
                _ => "5 / 100",
            };
            assert_eq!(count(app.world(), root).unwrap().1, expected);
            WidgetryTextField::set_text_in_world(app.world_mut(), root, "旗🇺🇸").unwrap();
            assert_eq!(
                count(app.world(), root).unwrap().1,
                if max == Some(0) {
                    "0 / 0"
                } else if max == Some(3) {
                    "2 / 3"
                } else if max.is_some() {
                    "2 / 100"
                } else {
                    "2"
                }
            );
            WidgetryTextField::set_text(&mut app.world_mut().commands(), root, "");
            app.world_mut().flush();
            assert_eq!(
                count(app.world(), root).unwrap().1,
                match max {
                    None => "0",
                    Some(0) => "0 / 0",
                    Some(3) => "0 / 3",
                    _ => "0 / 100",
                }
            );
        }
    }
}

#[test]
fn committed_edits_selection_paste_and_ime_update_the_same_frame() {
    use bevy::clipboard::ClipboardRead;
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryTextField { @show_count: true, @max_length: {Some(3)}, @value: {"中".to_owned()} }
    }).unwrap().id();
    app.update();
    edit(&mut app, root, [TextEdit::Insert("e\u{301}👍🏽🇨🇳".into())]);
    assert_eq!(
        count(app.world(), root).unwrap().1,
        "3 / 3",
        "actual editor: {:?}",
        WidgetryTextField::text(app.world(), root)
    );
    edit(
        &mut app,
        root,
        [TextEdit::SelectAll, TextEdit::Insert("旗".into())],
    );
    assert_eq!(count(app.world(), root).unwrap().1, "1 / 3");
    edit(
        &mut app,
        root,
        [TextEdit::ImeSetCompose {
            value: "预编辑文字不计数".into(),
            cursor: None,
        }],
    );
    assert_eq!(count(app.world(), root).unwrap().1, "1 / 3");
    edit(
        &mut app,
        root,
        [TextEdit::ImeCommit {
            value: "甲乙丙".into(),
        }],
    );
    assert_eq!(count(app.world(), root).unwrap().1, "3 / 3");
    edit(&mut app, root, [TextEdit::SelectAll]);
    let editor = support::editor_entity(app.world(), root);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_paste = Some(ClipboardRead::Ready(Ok("👨‍👩‍👧‍👦".into())));
    app.update();
    assert_eq!(count(app.world(), root).unwrap().1, "1 / 3");
    edit(&mut app, root, [TextEdit::SelectAll, TextEdit::Delete]);
    assert_eq!(count(app.world(), root).unwrap().1, "0 / 3");
}

#[test]
fn readonly_disabled_theme_and_focus_leave_display_and_editor_state_intact() {
    use bevy::ui::InteractionDisabled;
    use bevy_widgetry_theme::WidgetryThemeMode;
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryReadOnlyTextField { @show_count: true, @value: {"甲\n乙".to_owned()}, @multiline: true, @visible_lines: {Some(2.0)} }
        InteractionDisabled
    }).unwrap().id();
    app.update();
    assert_eq!(count(app.world(), root).unwrap().1, "3");
    edit(&mut app, root, [TextEdit::Insert("ignored".into())]);
    assert_eq!(count(app.world(), root).unwrap().1, "3");
    WidgetryReadOnlyTextField::set_text_in_world(app.world_mut(), root, "新").unwrap();
    assert_eq!(count(app.world(), root).unwrap().1, "1");
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    WidgetryReadOnlyTextField::focus_in_world(app.world_mut(), root).unwrap();
    edit(&mut app, root, [TextEdit::SelectAll]);
    let before = WidgetryReadOnlyTextField::editor(app.world(), root)
        .unwrap()
        .editor()
        .raw_selection()
        .text_range();
    *app.world_mut().resource_mut::<WidgetryThemeMode>() = WidgetryThemeMode::Dark;
    app.update();
    assert_eq!(count(app.world(), root).unwrap().1, "1");
    assert_eq!(
        WidgetryReadOnlyTextField::editor(app.world(), root)
            .unwrap()
            .editor()
            .raw_selection()
            .text_range(),
        before
    );
}

#[test]
fn keyboard_and_unlimited_ime_count_only_formal_text() {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        input_focus::InputFocus,
    };
    let mut app = editing_app();
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryTextFieldPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField { @show_count: true } })
        .unwrap()
        .id();
    app.update();
    WidgetryTextField::focus_in_world(app.world_mut(), root).unwrap();
    let focused = app.world().resource::<InputFocus>().get();
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character("a".into()),
        text: Some("a".into()),
        state: ButtonState::Pressed,
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(count(app.world(), root).unwrap().1, "1");
    edit(
        &mut app,
        root,
        [TextEdit::ImeSetCompose {
            value: "中文预编辑".into(),
            cursor: None,
        }],
    );
    assert_eq!(count(app.world(), root).unwrap().1, "1");
    edit(
        &mut app,
        root,
        [TextEdit::ImeCommit {
            value: "中e\u{301}".into(),
        }],
    );
    assert_eq!(count(app.world(), root).unwrap().1, "3");
    assert_eq!(app.world().resource::<InputFocus>().get(), focused);
}

#[test]
fn count_is_outside_the_frame_right_aligned_and_does_not_take_editor_width() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let mut roots = Vec::new();
    for show in [false, true] {
        roots.push(app.world_mut().spawn_scene(bsn! {
            @WidgetryTextField { @show_count: show, @value: {"abc".to_owned()}, @children: {Some(Box::new(bsn_list! { WidgetryTextFieldPrefix Node {width:px(40)} }) as Box<dyn SceneList>)} }
            Node {width:px(300)}
        }).unwrap().id());
    }
    for _ in 0..5 {
        app.update();
    }
    let (display, _) = count(app.world(), roots[1]).unwrap();
    let frame = support::frame_entity(app.world(), roots[1]);
    assert_eq!(
        app.world().get::<ChildOf>(display).unwrap().parent(),
        roots[1]
    );
    let f = app
        .world()
        .get::<UiGlobalTransform>(frame)
        .unwrap()
        .translation;
    let c = app
        .world()
        .get::<UiGlobalTransform>(display)
        .unwrap()
        .translation;
    let fs = app.world().get::<ComputedNode>(frame).unwrap().size();
    let cs = app.world().get::<ComputedNode>(display).unwrap().size();
    assert!(c.y - cs.y / 2.0 >= f.y + fs.y / 2.0);
    assert_eq!(
        app.world().get::<TextLayout>(display).unwrap().justify,
        Justify::Right
    );
    let sizes: Vec<_> = roots
        .iter()
        .map(|&r| {
            app.world()
                .get::<ComputedNode>(support::editor_entity(app.world(), r))
                .unwrap()
                .size()
        })
        .collect();
    assert_eq!(sizes[0], sizes[1]);
    assert!(!app.world().get::<Pickable>(display).unwrap().is_hoverable);
}
