#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::{EditableText, TextEdit},
    ui_widgets::ValueChange,
};
use bevy_widgetry_core::{input_target::WidgetryInputTarget, scene::spawn_scene};
use bevy_widgetry_number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin};
use bevy_widgetry_text_field::WidgetryTextField;

#[derive(Resource, Default)]
struct Changes(Vec<f64>);

fn fixture() -> (App, Entity, Entity, Entity) {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<f64>>, mut changes: ResMut<Changes>| changes.0.push(event.value),
    );
    let root = spawn_scene(app.world_mut(),bsn!{@WidgetryNumberInput<f64>{@value:{Some(10.0)},@min:{Some(-100.0)},@max:{Some(100.0)},@step:3.0}}).unwrap();
    let field = app.world().get::<Children>(root).unwrap()[0];
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    for _ in 0..8 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    (app, root, field, editor)
}

fn key(app: &mut App, code: KeyCode, logical: Key) {
    bevy_widgetry_test_utils::queue_key(
        app,
        KeyboardInput {
            key_code: code,
            logical_key: logical,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
    );
    app.update();
}

fn draft(app: &mut App, field: Entity, text: &str) {
    WidgetryTextField::set_text_in_world(app.world_mut(), field, text).unwrap();
}

#[test]
fn drafts_enter_escape_and_blur_follow_committed_value_contract() {
    let (mut app, root, field, editor) = fixture();
    assert!(app.world().get::<ComputedNode>(editor).unwrap().size().x > 20.0);
    for text in ["", "-", "1e3", "0xFF", "1,000", "NaN", "Infinity", "101"] {
        draft(&mut app, field, text);
        key(&mut app, KeyCode::Enter, Key::Enter);
        assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), text);
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
            10.0
        );
    }
    for (text, value, canonical) in [
        (".5", 0.5, "0.5"),
        ("-.5", -0.5, "-0.5"),
        ("1.", 1.0, "1.0"),
        ("12.50", 12.5, "12.5"),
    ] {
        draft(&mut app, field, text);
        key(&mut app, KeyCode::Enter, Key::Enter);
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
            value
        );
        assert_eq!(
            WidgetryTextField::text(app.world(), field).unwrap(),
            canonical
        );
    }
    assert_eq!(app.world().resource::<Changes>().0.len(), 4);
    draft(&mut app, field, "23");
    key(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "12.5");
    draft(&mut app, field, "25");
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        25.0
    );
    draft(&mut app, field, "abc");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "25.0");
    assert_eq!(app.world().resource::<Changes>().0.len(), 5);
}

#[test]
fn programmatic_writes_override_drafts_and_clear_queued_input_even_for_same_value() {
    let (mut app, root, field, editor) = fixture();
    draft(&mut app, field, "25");
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_edits
        .push(TextEdit::Insert("stale".into()));
    assert!(!WidgetryNumberInput::<f64>::set_value_in_world(app.world_mut(), root, 10.0).unwrap());
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "10.0");
    draft(&mut app, field, "draft");
    assert!(WidgetryNumberInput::<f64>::set_value_in_world(app.world_mut(), root, 101.0).is_err());
    assert_eq!(
        WidgetryTextField::text(app.world(), field).unwrap(),
        "draft"
    );
    assert!(WidgetryNumberInput::<f64>::set_value_in_world(app.world_mut(), root, 50.0).unwrap());
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "50.0");
    assert_eq!(app.world().resource::<Changes>().0, vec![50.0]);
}

#[test]
fn enter_commits_prior_pending_edits_and_escape_cancels_them() {
    let (mut app, root, field, editor) = fixture();
    draft(&mut app, field, "");
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_edits
        .push(TextEdit::Insert("4".into()));
    key(&mut app, KeyCode::Enter, Key::Enter);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        4.0
    );
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_edits
        .push(TextEdit::Insert("7".into()));
    key(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "4.0");
    assert_eq!(app.world().resource::<Changes>().0, vec![4.0]);
}

#[test]
fn numpad_enter_commits_the_same_way_as_main_enter() {
    let (mut app, root, field, _) = fixture();
    draft(&mut app, field, "42");
    key(&mut app, KeyCode::NumpadEnter, Key::Enter);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        42.0
    );
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "42.0");
    assert_eq!(app.world().resource::<Changes>().0, vec![42.0]);
}

#[test]
fn programmatic_write_clears_ime_composition_and_stale_preedit() {
    let (mut app, root, field, editor) = fixture();
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .pending_edits
        .push(TextEdit::ImeSetCompose {
            value: "未完成".into(),
            cursor: None,
        });
    app.update();
    assert!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .is_composing()
    );
    WidgetryNumberInput::<f64>::set_value_in_world(app.world_mut(), root, 50.0).unwrap();
    assert!(
        !app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .is_composing()
    );
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "50.0");
    assert_eq!(app.world().resource::<Changes>().0, vec![50.0]);
}

#[test]
fn same_frame_enter_separates_prior_and_later_keyboard_text() {
    let (mut app, root, field, _) = fixture();
    draft(&mut app, field, "");
    for (code, logical, text) in [
        (
            KeyCode::Digit2,
            Key::Character("2".into()),
            Some("2".into()),
        ),
        (KeyCode::Enter, Key::Enter, None),
        (
            KeyCode::Digit5,
            Key::Character("5".into()),
            Some("5".into()),
        ),
    ] {
        bevy_widgetry_test_utils::queue_key(
            &mut app,
            KeyboardInput {
                key_code: code,
                logical_key: logical,
                state: ButtonState::Pressed,
                text,
                repeat: false,
                window: Entity::PLACEHOLDER,
            },
        );
    }
    app.update();
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        2.0
    );
    assert!(
        WidgetryTextField::text(app.world(), field)
            .unwrap()
            .contains('5')
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![2.0]);
}

#[test]
fn integer_editors_reject_fraction_and_unsigned_negative_without_clamping() {
    let (mut app, _, _, _) = fixture();
    macro_rules! check {
        ($t:ty,$bad:expr,$good:expr,$expected:expr) => {
            let root = spawn_scene(
                app.world_mut(),
                bsn! {@WidgetryNumberInput<$t>{@value:{Some(10 as $t)}}},
            )
            .unwrap();
            let field = app.world().get::<Children>(root).unwrap()[0];
            let editor = app
                .world()
                .get::<WidgetryInputTarget>(field)
                .unwrap()
                .target;
            app.world_mut()
                .resource_mut::<InputFocus>()
                .set(editor, FocusCause::Navigated);
            app.update();
            draft(&mut app, field, $bad);
            key(&mut app, KeyCode::Enter, Key::Enter);
            assert_eq!(
                WidgetryNumberInput::<$t>::value(app.world(), root).unwrap(),
                10 as $t
            );
            draft(&mut app, field, $good);
            key(&mut app, KeyCode::Enter, Key::Enter);
            assert_eq!(
                WidgetryNumberInput::<$t>::value(app.world(), root).unwrap(),
                $expected
            );
        };
    }
    check!(i8, "1.5", "-128", i8::MIN);
    check!(u8, "-1", "255", u8::MAX);
}
