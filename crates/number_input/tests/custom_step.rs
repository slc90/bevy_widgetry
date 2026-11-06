#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::InputFocus,
    picking::{
        hover::Hovered,
        pointer::{Location, PointerAction, PointerButton},
    },
    prelude::*,
    text::{EditableText, TextEdit},
    ui::InteractionDisabled,
    ui_widgets::{Activate, ActivateOnPress, Button, ValueChange},
};
use bevy_widgetry_core::{pointer::WidgetryPointerPressed, scene::spawn_scene};
use bevy_widgetry_number_input::{
    WidgetryNumberInputHeadless as Number, WidgetryNumberInputHeadlessPlugin,
    WidgetryNumberInputStep,
};
use bevy_widgetry_text_field::{WidgetryTextFieldEditor, WidgetryTextFieldHeadless};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, f64)>);

fn fixture(read_only: bool) -> (App, Entity, Entity, Entity, Entity, Entity) {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryNumberInputHeadlessPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<f64>>, mut changes: ResMut<Changes>| {
            changes.0.push((event.source, event.value))
        },
    );
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera =
        bevy_widgetry_test_utils::spawn_picking_camera(&mut app, window, UVec2::splat(600), 1.0);
    let root=spawn_scene(app.world_mut(),bsn! {
        @Number<f64> {@value:{Some(10.0)},@min:{Some(0.0)},@max:{Some(100.0)},@step:5.0,@read_only:read_only}
        bevy_widgetry_number_input::WidgetryNumberInputEditor {field:#Field}
        Node {position_type:PositionType::Absolute,left:px(40),top:px(40),width:px(200),height:px(40)}
        template(move |_|Ok(UiTargetCamera(camera)))
        Children [#Field @WidgetryTextFieldHeadless {editor:#Editor,@read_only:read_only}
            Node {width:percent(100)} Children [Node Children [#Editor @WidgetryTextFieldEditor {root:#Field} Node {width:px(180),height:px(36)}]]]
    }).unwrap();
    let trigger=spawn_scene(app.world_mut(),bsn! {
        WidgetryNumberInputStep {root:root,increase:true} Button ActivateOnPress WidgetryPointerPressed Hovered(false)
        Node {position_type:PositionType::Absolute,left:px(300),top:px(40),width:px(40),height:px(40)}
        template(move |_|Ok(UiTargetCamera(camera)))
    }).unwrap();
    for _ in 0..3 {
        app.update();
    }
    let field = Number::<f64>::field_entity(app.world(), root).unwrap();
    let editor = Number::<f64>::editor_entity(app.world(), root).unwrap();
    WidgetryTextFieldHeadless::focus_in_world(app.world_mut(), field).unwrap();
    app.update();
    (app, root, field, editor, trigger, window)
}

#[test]
fn headless_plugin_runs_without_default_number_or_text_field_style() {
    let (app, _, _, _, _, _) = fixture(false);
    assert!(!app.is_plugin_added::<bevy_widgetry_number_input::WidgetryNumberInputPlugin>());
    assert!(!app.is_plugin_added::<bevy_widgetry_text_field::WidgetryTextFieldPlugin>());
}

#[test]
fn direct_and_queued_steps_merge_drafts_once_and_reject_invalid_roots() {
    let (mut app, root, field, _, _, _) = fixture(false);
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "25").unwrap();
    assert!(Number::<f64>::step_in_world(app.world_mut(), root, true).unwrap());
    assert_eq!(Number::<f64>::value(app.world(), root).unwrap(), 30.0);
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 30.0)]);
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "invalid").unwrap();
    Number::<f64>::step(&mut app.world_mut().commands(), root, false);
    app.world_mut().flush();
    assert_eq!(Number::<f64>::value(app.world(), root).unwrap(), 25.0);
    assert!(Number::<f64>::step_in_world(app.world_mut(), Entity::PLACEHOLDER, true).is_err());
    Number::<f64>::set_value_in_world(app.world_mut(), root, 100.0).unwrap();
    let count = app.world().resource::<Changes>().0.len();
    assert!(!Number::<f64>::step_in_world(app.world_mut(), root, true).unwrap());
    assert_eq!(app.world().resource::<Changes>().0.len(), count);
}

#[test]
fn external_press_keeps_editor_focus_and_has_no_release_step() {
    let (mut app, root, field, editor, _, window) = fixture(false);
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "25").unwrap();
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    app.world_mut().spawn(pointer);
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(320.0, 60.0),
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
    ] {
        bevy_widgetry_test_utils::queue_pointer(&mut app, pointer, at.clone(), action);
        app.update();
    }
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 30.0)]);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    bevy_widgetry_test_utils::queue_pointer(
        &mut app,
        pointer,
        at,
        PointerAction::Release(PointerButton::Primary),
    );
    app.update();
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 30.0)]);
}

#[test]
fn focusable_release_trigger_does_not_submit_a_draft_on_press() {
    use bevy::input_focus::{
        AcquireFocus,
        tab_navigation::{TabIndex, acquire_focus_tab_index},
    };
    let (mut app, root, field, editor, trigger, window) = fixture(false);
    app.add_plugins(bevy_widgetry_button::WidgetryButtonPlugin);
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    app.world_mut()
        .entity_mut(trigger)
        .remove::<(Button, ActivateOnPress, WidgetryPointerPressed)>()
        .insert(TabIndex(0));
    app.world_mut()
        .entity_mut(trigger)
        .apply_scene(bsn! { @bevy_widgetry_button::WidgetryButtonHeadless })
        .unwrap();
    // Exercise native tab focus after the global redirect, independently of global observer order.
    app.world_mut()
        .entity_mut(trigger)
        .observe(acquire_focus_tab_index);
    WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "25").unwrap();
    let mut event = AcquireFocus {
        focused_entity: trigger,
        window,
    };
    app.world_mut().trigger_ref(&mut event);
    assert_eq!(event.focused_entity, editor);
    errors.run(|| app.update());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert!(app.world().resource::<Changes>().0.is_empty());
    let pointer = bevy::picking::pointer::PointerId::Mouse;
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(320.0, 60.0),
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
    ] {
        bevy_widgetry_test_utils::queue_pointer(&mut app, pointer, at.clone(), action);
        errors.run(|| app.update());
    }
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert!(app.world().get::<bevy::ui::Pressed>(trigger).is_some());
    bevy_widgetry_test_utils::queue_pointer(
        &mut app,
        pointer,
        at,
        PointerAction::Release(PointerButton::Primary),
    );
    errors.run(|| app.update());
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 30.0)]);
    assert!(errors.take_records().is_empty());
}

#[test]
fn an_empty_ime_clear_request_does_not_block_a_valid_step() {
    let (mut app, root, _, editor, _, _) = fixture(false);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::clear_ime_compose());
    assert!(Number::<f64>::step_in_world(app.world_mut(), root, true).unwrap());
    assert_eq!(Number::<f64>::value(app.world(), root).unwrap(), 15.0);
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 15.0)]);
}

#[test]
fn a_focusable_trigger_cannot_commit_a_disabled_field_or_editor_draft() {
    use bevy::input_focus::tab_navigation::{TabIndex, acquire_focus_tab_index};
    for disable_field in [false, true] {
        let (mut app, root, field, editor, trigger, window) = fixture(false);
        app.world_mut()
            .entity_mut(trigger)
            .insert(TabIndex(0))
            .observe(acquire_focus_tab_index);
        WidgetryTextFieldHeadless::set_text_in_world(app.world_mut(), field, "25").unwrap();
        app.world_mut()
            .entity_mut(if disable_field { field } else { editor })
            .insert(InteractionDisabled);
        app.world_mut().flush();
        let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
        app.world_mut().spawn(pointer);
        let at = Location {
            target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                .normalize(None)
                .unwrap(),
            position: Vec2::new(320.0, 60.0),
        };
        for action in [
            PointerAction::Move { delta: Vec2::ZERO },
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            bevy_widgetry_test_utils::queue_pointer(&mut app, pointer, at.clone(), action);
            app.update();
        }
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
        assert_eq!(Number::<f64>::value(app.world(), root).unwrap(), 10.0);
        assert_eq!(
            WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
            "25"
        );
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn readonly_disabled_editor_and_pending_ime_guard_both_step_paths() {
    let (mut app, root, _, editor, trigger, _) = fixture(true);
    assert!(!Number::<f64>::step_in_world(app.world_mut(), root, true).unwrap());
    app.world_mut().trigger(Activate { entity: trigger });
    app.world_mut().flush();
    assert_eq!(Number::<f64>::value(app.world(), root).unwrap(), 10.0);
    assert!(Number::<f64>::set_value_in_world(app.world_mut(), root, 20.0).unwrap());
    assert!(app.world().get::<EditableText>(editor).is_some());
    let (mut app, root, _, editor, trigger, _) = fixture(false);
    for disabled in [root, editor] {
        app.world_mut()
            .entity_mut(disabled)
            .insert(InteractionDisabled);
        app.world_mut().flush();
        assert!(!Number::<f64>::step_in_world(app.world_mut(), root, true).unwrap());
        app.world_mut().trigger(Activate { entity: trigger });
        app.world_mut().flush();
        assert_eq!(Number::<f64>::value(app.world(), root).unwrap(), 10.0);
        app.world_mut()
            .entity_mut(disabled)
            .remove::<InteractionDisabled>();
        app.world_mut().flush();
    }
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .queue_edit(TextEdit::ImeSetCompose {
            value: "未完成".into(),
            cursor: None,
        });
    assert!(!Number::<f64>::step_in_world(app.world_mut(), root, true).unwrap());
    app.world_mut().trigger(Activate { entity: trigger });
    app.world_mut().flush();
    assert_eq!(Number::<f64>::value(app.world(), root).unwrap(), 10.0);
}

#[test]
fn keyboard_steps_headless_and_stale_trigger_reports_through_host() {
    let (mut app, root, _, _, trigger, _) = fixture(false);
    bevy_widgetry_test_utils::queue_key(
        &mut app,
        KeyboardInput {
            key_code: KeyCode::ArrowUp,
            logical_key: Key::ArrowUp,
            state: ButtonState::Pressed,
            text: None,
            repeat: true,
            window: Entity::PLACEHOLDER,
        },
    );
    app.update();
    assert_eq!(Number::<f64>::value(app.world(), root).unwrap(), 15.0);
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    app.world_mut().despawn(root);
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| {
        app.world_mut().trigger(Activate { entity: trigger });
        app.world_mut().flush();
    });
    let errors = errors.take_records();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
}

#[test]
fn triggers_dispatch_to_the_bound_numeric_type_without_crossing_roots() {
    let (mut app, first, _, _, first_trigger, _) = fixture(false);
    let second=spawn_scene(app.world_mut(),bsn! {
        @Number<u128> {@value:{Some(u128::MAX-1)}}
        bevy_widgetry_number_input::WidgetryNumberInputEditor {field:#Field}
        Children [#Field @WidgetryTextFieldHeadless {editor:#Editor} Children [#Editor @WidgetryTextFieldEditor {root:#Field}]]
    }).unwrap();
    let second_trigger = spawn_scene(
        app.world_mut(),
        bsn! {WidgetryNumberInputStep {root:second,increase:true}},
    )
    .unwrap();
    app.world_mut().trigger(Activate {
        entity: second_trigger,
    });
    app.world_mut().flush();
    assert_eq!(
        Number::<u128>::value(app.world(), second).unwrap(),
        u128::MAX
    );
    assert_eq!(Number::<f64>::value(app.world(), first).unwrap(), 10.0);
    assert!(app.world().resource::<Changes>().0.is_empty());
    app.world_mut().trigger(Activate {
        entity: first_trigger,
    });
    app.world_mut().flush();
    assert_eq!(Number::<f64>::value(app.world(), first).unwrap(), 15.0);
    assert_eq!(
        Number::<u128>::value(app.world(), second).unwrap(),
        u128::MAX
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![(first, 15.0)]);
}
