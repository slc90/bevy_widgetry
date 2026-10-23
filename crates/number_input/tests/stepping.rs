#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    picking::pointer::{Location, PointerAction, PointerButton},
    prelude::*,
    ui_widgets::ValueChange,
};
use bevy_widgetry_core::{input_target::WidgetryInputTarget, scene::spawn_scene};
use bevy_widgetry_number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin};
use bevy_widgetry_text_field::WidgetryTextField;

#[derive(Resource, Default)]
struct Changes(Vec<f64>);

fn fixture(value: f64, max: f64, step: f64) -> (App, Entity, Entity, Entity, Entity) {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera =
        bevy_widgetry_test_utils::spawn_picking_camera(&mut app, window, UVec2::splat(600), 1.0);
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<f64>>, mut changes: ResMut<Changes>| changes.0.push(event.value),
    );
    let root=spawn_scene(app.world_mut(),bsn!{@WidgetryNumberInput<f64>{@value:{Some(value)},@min:{Some(0.0)},@max:{Some(max)},@step:{step}} Node{position_type:PositionType::Absolute,left:px(40),top:px(40)} template(move |_|Ok(UiTargetCamera(camera)))}).unwrap();
    let field = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|e| app.world().get::<WidgetryTextField>(*e).is_some())
        .unwrap();
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    for _ in 0..6 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    (app, root, window, field, editor)
}

fn key(app: &mut App, increase: bool, repeat: bool) {
    bevy_widgetry_test_utils::queue_key(
        app,
        KeyboardInput {
            key_code: if increase {
                KeyCode::ArrowUp
            } else {
                KeyCode::ArrowDown
            },
            logical_key: if increase {
                Key::ArrowUp
            } else {
                Key::ArrowDown
            },
            state: ButtonState::Pressed,
            text: None,
            repeat,
            window: Entity::PLACEHOLDER,
        },
    );
    app.update();
}

#[test]
fn keyboard_uses_current_value_and_reachable_endpoints_with_one_notification() {
    let (mut app, root, _, _, _) = fixture(0.0, 10.0, 3.0);
    for value in [3.0, 6.0, 9.0, 10.0, 10.0] {
        key(&mut app, true, true);
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
            value
        );
    }
    for value in [7.0, 4.0, 1.0, 0.0, 0.0] {
        key(&mut app, false, false);
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
            value
        );
    }
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![3.0, 6.0, 9.0, 10.0, 7.0, 4.0, 1.0, 0.0]
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    key(&mut app, true, false);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        0.0
    );
}

#[test]
fn valid_and_invalid_drafts_step_directly_to_one_final_value() {
    let (mut app, root, _, field, _) = fixture(10.0, 100.0, 5.0);
    WidgetryTextField::set_text_in_world(app.world_mut(), field, "5").unwrap();
    key(&mut app, true, false);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        10.0
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    WidgetryTextField::set_text_in_world(app.world_mut(), field, "25").unwrap();
    key(&mut app, true, false);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        30.0
    );
    WidgetryTextField::set_text_in_world(app.world_mut(), field, "abc").unwrap();
    key(&mut app, true, false);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        35.0
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![30.0, 35.0]);
}

#[test]
fn arrows_leave_active_and_pending_ime_composition_owned_by_the_editor() {
    use bevy::text::{EditableText, TextEdit};
    for pending in [false, true] {
        let (mut app, root, _, field, editor) = fixture(10.0, 100.0, 5.0);
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .unwrap()
            .pending_edits
            .push(TextEdit::ImeSetCompose {
                value: "候选".into(),
                cursor: None,
            });
        if !pending {
            app.update();
        }
        key(&mut app, true, false);
        assert!(
            WidgetryTextField::editor(app.world(), field)
                .unwrap()
                .is_composing()
        );
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
            10.0
        );
        assert!(app.world().resource::<Changes>().0.is_empty());
        key(&mut app, false, false);
        assert!(
            WidgetryTextField::editor(app.world(), field)
                .unwrap()
                .is_composing()
        );
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
            10.0
        );
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn disabled_guards_input_but_keeps_programmatic_values_available() {
    let (mut app, root, _, _, _) = fixture(0.0, 10.0, 3.0);
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    app.update();
    key(&mut app, true, false);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        0.0
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    WidgetryNumberInput::<f64>::set_value_in_world(app.world_mut(), root, 4.0).unwrap();
    assert_eq!(app.world().resource::<Changes>().0, vec![4.0]);
}

#[test]
fn singleton_ranges_and_unsigned_type_limits_remain_stable() {
    let (mut app, _, _, _, _) = fixture(0.0, 10.0, 3.0);
    let root = spawn_scene(
        app.world_mut(),
        bsn! {@WidgetryNumberInput<u8>{@value:{Some(253)},@step:5}},
    )
    .unwrap();
    let field = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|e| app.world().get::<WidgetryTextField>(*e).is_some())
        .unwrap();
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    key(&mut app, true, false);
    key(&mut app, true, false);
    assert_eq!(
        WidgetryNumberInput::<u8>::value(app.world(), root).unwrap(),
        255
    );
    key(&mut app, false, false);
    assert_eq!(
        WidgetryNumberInput::<u8>::value(app.world(), root).unwrap(),
        250
    );
    let root = spawn_scene(
        app.world_mut(),
        bsn! {@WidgetryNumberInput<f64>{@value:{Some(3.0)},@min:{Some(3.0)},@max:{Some(3.0)}}},
    )
    .unwrap();
    let field = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|e| app.world().get::<WidgetryTextField>(*e).is_some())
        .unwrap();
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    key(&mut app, true, false);
    key(&mut app, false, false);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        3.0
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn internal_button_keeps_editor_focus_and_does_not_commit_an_intermediate_draft() {
    let (mut app, root, window, field, editor) = fixture(10.0, 100.0, 5.0);
    WidgetryTextField::set_text_in_world(app.world_mut(), field, "25").unwrap();
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    app.world_mut().spawn(pointer);
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(226.0, 56.0),
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        bevy_widgetry_test_utils::queue_pointer(&mut app, pointer, at.clone(), action);
        app.update();
    }
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        30.0
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert_eq!(app.world().resource::<Changes>().0, vec![30.0]);
}

#[test]
fn wheel_over_number_input_scrolls_the_parent_without_changing_value() {
    use bevy_widgetry_scroll_area::{
        WidgetryScrollArea, WidgetryScrollAreaPlugin, WidgetryScrollAreaViewport,
    };
    let (mut app, _, _, _, _) = fixture(0.0, 10.0, 3.0);
    app.add_plugins(WidgetryScrollAreaPlugin);
    spawn_scene(
        app.world_mut(),
        bsn! {@WidgetryScrollArea{@children:bsn_list!{
            #WheelNumber @WidgetryNumberInput<f64>{@value:{Some(5.0)}}--
            Node{height:px(400),flex_shrink:0.0}
        }} Node{width:px(220),height:px(100)}},
    )
    .unwrap();
    for _ in 0..6 {
        app.update();
    }
    let root = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "WheelNumber").then_some(entity))
        .unwrap();
    let field = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|e| app.world().get::<WidgetryTextField>(*e).is_some())
        .unwrap();
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    let click = bevy_widgetry_test_utils::primary_click(editor);
    app.world_mut()
        .trigger(bevy::picking::events::PointerScroll {
            entity: editor,
            pointer: click.pointer,
            x: 0.0,
            y: -20.0,
            unit: bevy::input::mouse::MouseScrollUnit::Pixel,
            hit: click.hit,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<bevy::ui::ScrollPosition>(viewport)
            .unwrap()
            .0
            .y,
        20.0
    );
    assert_eq!(
        WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
        5.0
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}
