#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::{EditableText, TextEdit},
    ui::InteractionDisabled,
    ui_widgets::ValueChange,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_number_input::{
    WidgetryNumberInput, WidgetryNumberInputEditor, WidgetryNumberInputHeadless,
    WidgetryNumberInputPlugin,
};
use bevy_widgetry_text_field::{WidgetryTextFieldEditor, WidgetryTextFieldHeadless};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, f64, bool)>);

fn fixture(read_only: bool) -> (App, Entity, Entity, Entity) {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<f64>>, mut changes: ResMut<Changes>| {
            changes.0.push((event.source, event.value, event.is_final))
        },
    );
    let root = spawn_scene(app.world_mut(), bsn! {
        @WidgetryNumberInputHeadless<f64> { @value: {Some(1.234)}, @decimals: {Some(2)}, @min: {Some(-10.0)}, @max: {Some(10.0)}, @read_only: read_only }
        WidgetryNumberInputEditor {field:#Field}
        Node {width:px(260)}
        Children [
            Node {padding:UiRect::all(px(5))}
            Children [
                #Field @WidgetryTextFieldHeadless {editor:#Editor, @read_only:read_only}
                Children [ Node Children [
                    #Editor @WidgetryTextFieldEditor {root:#Field}
                    Node {width:px(200),height:px(36)}
                ] ]
            ]
        ]
    }).unwrap();
    let field = app
        .world()
        .get::<WidgetryNumberInputEditor>(root)
        .unwrap()
        .field;
    let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), field).unwrap();
    for _ in 0..3 {
        app.update();
    }
    (app, root, field, editor)
}

fn key(app: &mut App, key_code: KeyCode, logical_key: Key) {
    bevy_widgetry_test_utils::queue_key(
        app,
        KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
    );
    app.update();
}

#[test]
fn nested_editor_without_buttons_uses_numeric_identity_and_submit_contract() {
    let (mut app, root, field, editor) = fixture(false);
    assert!(app.world().get::<WidgetryNumberInput<f64>>(root).is_none());
    assert_eq!(
        WidgetryNumberInputHeadless::<f64>::value(app.world(), root).unwrap(),
        1.234
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "1.23"
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "1.234"
    );
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "-.5").unwrap();
    app.update();
    assert_eq!(
        WidgetryNumberInputHeadless::<f64>::value(app.world(), root).unwrap(),
        1.234
    );
    key(&mut app, KeyCode::Enter, Key::Enter);
    assert_eq!(
        WidgetryNumberInputHeadless::<f64>::value(app.world(), root).unwrap(),
        -0.5
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, -0.5, true)]
    );
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "-").unwrap();
    key(&mut app, KeyCode::Enter, Key::Enter);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "-"
    );
    key(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "-0.5"
    );
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "3.25").unwrap();
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(
        WidgetryNumberInputHeadless::<f64>::value(app.world(), root).unwrap(),
        3.25
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 2);
}

#[test]
fn custom_readonly_and_inherited_disabled_keep_setters_and_cancel_stale_edits() {
    for read_only in [false, true] {
        let (mut app, root, field, editor) = fixture(read_only);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(editor, FocusCause::Navigated);
        app.update();
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .unwrap()
            .queue_edit(TextEdit::Insert("9".into()));
        assert!(
            !WidgetryNumberInputHeadless::<f64>::set_value_in_world(app.world_mut(), root, 1.234)
                .unwrap()
        );
        app.update();
        assert_eq!(
            WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
            "1.234"
        );
        let parent = spawn_scene(app.world_mut(), bsn! { Node InteractionDisabled }).unwrap();
        WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "7").unwrap();
        app.world_mut().entity_mut(root).insert(ChildOf(parent));
        app.update();
        assert_eq!(
            WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
            "1.23"
        );
        assert!(
            WidgetryNumberInputHeadless::<f64>::set_value_in_world(app.world_mut(), root, 2.5)
                .unwrap()
        );
        app.world_mut()
            .entity_mut(parent)
            .remove::<InteractionDisabled>();
        app.update();
        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();
        assert_eq!(
            WidgetryNumberInputHeadless::<f64>::value(app.world(), root).unwrap(),
            2.5
        );
        assert_eq!(app.world().resource::<Changes>().0, vec![(root, 2.5, true)]);
    }
}

#[test]
fn headless_preserves_all_numeric_types_and_exact_large_integer_values() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    macro_rules! check {
        ($($t:ty),+) => {$({
            let root = spawn_scene(app.world_mut(),bsn! {
                @WidgetryNumberInputHeadless<$t> {@value:{Some(2 as $t)},@step:{3 as $t}}
                WidgetryNumberInputEditor {field:#Field}
                Children [#Field @bevy_widgetry_text_field::WidgetryTextField]
            }).unwrap();
            assert!(WidgetryNumberInputHeadless::<$t>::set_value_in_world(app.world_mut(),root,4 as $t).unwrap());
            assert_eq!(WidgetryNumberInputHeadless::<$t>::value(app.world(),root).unwrap(),4 as $t);
        })+};
    }
    check!(
        i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
    );
    let root = spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetryNumberInputHeadless<u128> {@value:{Some(u128::MAX)}}
            WidgetryNumberInputEditor {field:#Field}
            Children [#Field @bevy_widgetry_text_field::WidgetryTextField]
        },
    )
    .unwrap();
    let field = WidgetryNumberInputHeadless::<u128>::field_entity(app.world(), root).unwrap();
    assert!(
        !WidgetryNumberInputHeadless::<u128>::set_value_in_world(app.world_mut(), root, u128::MAX)
            .unwrap()
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        u128::MAX.to_string()
    );
}

#[test]
fn foreign_or_inconsistent_editor_bindings_fail_before_mutating_value_or_draft() {
    use bevy_widgetry_number_input::NumberInputError;
    let (mut app, root, field, _) = fixture(false);
    let other = spawn_scene(
        app.world_mut(),
        bsn! { @bevy_widgetry_text_field::WidgetryTextField {@value:{"other".to_owned()}} },
    )
    .unwrap();
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryNumberInputEditor { field: other });
    let error = WidgetryNumberInputHeadless::<f64>::set_value_in_world(app.world_mut(), root, 2.0)
        .unwrap_err();
    assert!(
        matches!(error.kind(), NumberInputError::InvalidEditorBinding {root:owner,field:bound,..} if *owner==root && *bound==other)
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), other).unwrap(),
        "other"
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "1.23"
    );
    assert_eq!(
        WidgetryNumberInputHeadless::<f64>::value(app.world(), root).unwrap(),
        1.234
    );
    let readonly = spawn_scene(
        app.world_mut(),
        bsn! { @bevy_widgetry_text_field::WidgetryReadOnlyTextField },
    )
    .unwrap();
    app.world_mut().entity_mut(root).add_child(readonly);
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryNumberInputEditor { field: readonly });
    assert!(matches!(
        WidgetryNumberInputHeadless::<f64>::set_value_in_world(app.world_mut(), root, 2.0)
            .unwrap_err()
            .kind(),
        NumberInputError::InvalidEditorBinding { .. }
    ));
}

fn reject_editor_options(max_length: Option<usize>, multiline: bool) {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    let root = spawn_scene(app.world_mut(),bsn! {
            @WidgetryNumberInputHeadless<i32> {@value:{Some(123)}}
            WidgetryNumberInputEditor {field:#Field}
            Children [
                #Field @WidgetryTextFieldHeadless {editor:#Editor}
                Children [#Editor @WidgetryTextFieldEditor {root:#Field,@value:{"12".to_owned()},@max_length:max_length,@multiline:multiline}]
            ]
        }).unwrap();
    let field = app
        .world()
        .get::<WidgetryNumberInputEditor>(root)
        .unwrap()
        .field;
    let before = WidgetryTextFieldHeadless::text(app.world(), field).unwrap();
    let logs = bevy_widgetry_test_utils::LogCapture::default();
    let error = logs.run(|| {
        WidgetryNumberInputHeadless::<i32>::set_value_in_world(app.world_mut(), root, 456)
            .unwrap_err()
    });
    assert!(
        matches!(error.kind(),bevy_widgetry_number_input::NumberInputError::IncompatibleEditor {root:owner,field:bound,max_length:limit,multiline:lines,..} if *owner==root && *bound==field && *limit==max_length && *lines==multiline)
    );
    assert_eq!(
        logs.diagnostic_records(
            error.origin().id(),
            bevy_widgetry_test_utils::DiagnosticPhase::Source
        )
        .len(),
        1
    );
    assert_eq!(
        WidgetryNumberInputHeadless::<i32>::value(app.world(), root).unwrap(),
        123
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        before
    );
    assert!(WidgetryNumberInputHeadless::<i32>::editor_entity(app.world(), root).is_err());
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| app.update());
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    errors.run(|| app.update());
    assert!(errors.take_records().is_empty());
    assert_eq!(
        WidgetryNumberInputHeadless::<i32>::value(app.world(), root).unwrap(),
        123
    );
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        before
    );
}

#[test]
fn numeric_binding_rejects_text_truncation() {
    reject_editor_options(Some(2), false);
}

#[test]
fn numeric_binding_rejects_multiline_enter_consumption() {
    reject_editor_options(None, true);
}
