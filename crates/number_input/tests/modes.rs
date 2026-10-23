#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    clipboard::ClipboardRead,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    picking::pointer::{Location, PointerAction, PointerButton},
    prelude::*,
    text::{EditableText, TextEdit},
    time::TimeUpdateStrategy,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::ValueChange,
};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::{WidgetryAppExt, input_target::WidgetryInputTarget, scene::spawn_scene};
use bevy_widgetry_number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin};
use bevy_widgetry_text_field::{WidgetryReadOnlyTextField, WidgetryTextField};
use std::time::Duration;

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, f64)>);

struct Fixture {
    app: App,
    parent: Entity,
    root: Entity,
    field: Entity,
    editor: Entity,
    increase: Entity,
    at: Location,
}

fn fixture(read_only: bool) -> Fixture {
    configured(read_only, 10.0, None)
}

fn configured(read_only: bool, value: f64, decimals: Option<u8>) -> Fixture {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.add_plugins(WidgetryAssetPlugin);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    bevy_widgetry_test_utils::advance_until(
        &mut app,
        Duration::from_secs(10),
        "NumberInput font",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<f64>>, mut changes: ResMut<Changes>| {
            changes.0.push((event.source, event.value));
        },
    );
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera =
        bevy_widgetry_test_utils::spawn_picking_camera(&mut app, window, UVec2::splat(600), 1.0);
    let parent = spawn_scene(app.world_mut(), bsn! { Node }).unwrap();
    let root = spawn_scene(app.world_mut(), bsn! {
        @WidgetryNumberInput<f64> { @value: {Some(value)}, @min: {Some(0.0)}, @max: {Some(100.0)}, @step: 3.0, @read_only: {read_only}, @decimals: {decimals} }
        Node {position_type:PositionType::Absolute,left:px(40),top:px(40)}
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap();
    app.world_mut().entity_mut(parent).add_child(root);
    let children = app.world().get::<Children>(root).unwrap();
    let (field, increase) = (children[1], children[2]);
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    for _ in 0..6 {
        app.update();
    }
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: app
            .world()
            .get::<UiGlobalTransform>(increase)
            .unwrap()
            .translation,
    };
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    app.world_mut().spawn(pointer);
    bevy_widgetry_test_utils::queue_pointer(
        &mut app,
        pointer,
        at.clone(),
        PointerAction::Move { delta: Vec2::ZERO },
    );
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    Fixture {
        app,
        parent,
        root,
        field,
        editor,
        increase,
        at,
    }
}

fn value(f: &Fixture) -> f64 {
    WidgetryNumberInput::<f64>::value(f.app.world(), f.root).unwrap()
}

fn text(f: &Fixture) -> String {
    f.app
        .world()
        .get::<EditableText>(f.editor)
        .unwrap()
        .value()
        .to_string()
}

fn key(f: &mut Fixture, code: KeyCode, logical: Key, text: Option<&str>) {
    bevy_widgetry_test_utils::queue_key(
        &mut f.app,
        KeyboardInput {
            key_code: code,
            logical_key: logical,
            state: ButtonState::Pressed,
            text: text.map(Into::into),
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
    );
    f.app.update();
}

fn advance(f: &mut Fixture, ms: u64) {
    f.app
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            ms,
        )));
    f.app.update();
}

fn pointer(f: &mut Fixture, action: PointerAction) {
    f.app
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    bevy_widgetry_test_utils::queue_pointer(&mut f.app, pointer, f.at.clone(), action);
    f.app.update();
}

#[test]
fn read_only_reuses_selection_but_rejects_text_arrows_buttons_and_hold() {
    let mut f = fixture(true);
    assert!(
        f.app
            .world()
            .get::<WidgetryReadOnlyTextField>(f.field)
            .is_some()
    );
    assert!(f.app.world().get::<WidgetryTextField>(f.field).is_none());
    assert!(f.app.world().get::<InteractionDisabled>(f.root).is_none());
    assert!(f.app.world().get::<InteractionDisabled>(f.editor).is_none());
    f.app
        .world_mut()
        .get_mut::<EditableText>(f.editor)
        .unwrap()
        .queue_edit(TextEdit::SelectAll);
    f.app.update();
    assert_eq!(
        f.app
            .world()
            .get::<EditableText>(f.editor)
            .unwrap()
            .editor()
            .raw_selection()
            .text_range(),
        0..4
    );
    key(
        &mut f,
        KeyCode::Digit2,
        Key::Character("2".into()),
        Some("2"),
    );
    assert_eq!(text(&f), "10.0");
    {
        let mut edit = f.app.world_mut().get_mut::<EditableText>(f.editor).unwrap();
        edit.queue_edit(TextEdit::Paste);
        edit.pending_paste = Some(ClipboardRead::Ready(Ok("25".into())));
        edit.queue_edit(TextEdit::ImeCommit { value: "25".into() });
    }
    key(&mut f, KeyCode::ArrowUp, Key::ArrowUp, None);
    key(&mut f, KeyCode::ArrowDown, Key::ArrowDown, None);
    key(&mut f, KeyCode::Enter, Key::Enter, None);
    pointer(&mut f, PointerAction::Press(PointerButton::Primary));
    advance(&mut f, 500);
    pointer(&mut f, PointerAction::Release(PointerButton::Primary));
    assert_eq!(value(&f), 10.0);
    assert_eq!(text(&f), "10.0");
    assert!(f.app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn disabled_discards_drafts_ime_and_pending_input_before_blur_or_reenable() {
    for inherited in [false, true] {
        let mut f = fixture(false);
        WidgetryTextField::set_text_in_world(f.app.world_mut(), f.field, "25").unwrap();
        f.app
            .world_mut()
            .get_mut::<EditableText>(f.editor)
            .unwrap()
            .queue_edit(TextEdit::ImeSetCompose {
                value: "候选".into(),
                cursor: None,
            });
        f.app.update();
        assert!(
            f.app
                .world()
                .get::<EditableText>(f.editor)
                .unwrap()
                .is_composing()
        );
        {
            let mut edit = f.app.world_mut().get_mut::<EditableText>(f.editor).unwrap();
            edit.queue_edit(TextEdit::ImeCommit { value: "99".into() });
            edit.pending_paste = Some(ClipboardRead::Ready(Ok("50".into())));
        }
        let target = if inherited { f.parent } else { f.root };
        f.app
            .world_mut()
            .entity_mut(target)
            .insert(InteractionDisabled);
        f.app.world_mut().flush();
        assert_eq!(text(&f), "10.0");
        assert!(
            !f.app
                .world()
                .get::<EditableText>(f.editor)
                .unwrap()
                .is_composing()
        );
        key(&mut f, KeyCode::Enter, Key::Enter, None);
        key(&mut f, KeyCode::ArrowUp, Key::ArrowUp, None);
        f.app.world_mut().resource_mut::<InputFocus>().clear();
        f.app.update();
        f.app
            .world_mut()
            .entity_mut(target)
            .remove::<InteractionDisabled>();
        f.app.world_mut().flush();
        f.app.update();
        assert_eq!(value(&f), 10.0);
        assert_eq!(text(&f), "10.0");
        assert!(f.app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn mode_guards_do_not_limit_valid_same_or_invalid_programmatic_values() {
    for read_only in [false, true] {
        for disabled in [false, true] {
            let mut f = fixture(read_only);
            if disabled {
                f.app
                    .world_mut()
                    .entity_mut(f.parent)
                    .insert(InteractionDisabled);
                f.app.world_mut().flush();
            }
            assert!(
                !WidgetryNumberInput::<f64>::set_value_in_world(f.app.world_mut(), f.root, 10.0)
                    .unwrap()
            );
            assert!(
                WidgetryNumberInput::<f64>::set_value_in_world(f.app.world_mut(), f.root, 101.0)
                    .is_err()
            );
            assert_eq!(value(&f), 10.0);
            assert_eq!(text(&f), "10.0");
            WidgetryNumberInput::<f64>::set_value(&mut f.app.world_mut().commands(), f.root, 50.0);
            f.app.world_mut().flush();
            assert_eq!(value(&f), 50.0);
            assert_eq!(text(&f), "50.0");
            assert_eq!(f.app.world().resource::<Changes>().0, vec![(f.root, 50.0)]);
        }
    }
}

#[test]
fn disabling_mid_hold_drops_pressed_and_never_resumes_without_a_new_press() {
    let mut f = fixture(false);
    pointer(&mut f, PointerAction::Press(PointerButton::Primary));
    advance(&mut f, 400);
    assert_eq!(value(&f), 16.0);
    f.app
        .world_mut()
        .entity_mut(f.parent)
        .insert(InteractionDisabled);
    f.app.world_mut().flush();
    assert!(f.app.world().get::<Pressed>(f.increase).is_none());
    advance(&mut f, 500);
    assert_eq!(value(&f), 16.0);
    f.app
        .world_mut()
        .entity_mut(f.parent)
        .remove::<InteractionDisabled>();
    f.app.world_mut().flush();
    advance(&mut f, 1000);
    assert_eq!(value(&f), 16.0);
    pointer(&mut f, PointerAction::Release(PointerButton::Primary));
    pointer(&mut f, PointerAction::Press(PointerButton::Primary));
    assert_eq!(value(&f), 19.0);
    advance(&mut f, 399);
    assert_eq!(value(&f), 19.0);
    advance(&mut f, 1);
    assert_eq!(value(&f), 22.0);
}

#[test]
fn read_only_blur_and_disabled_focus_preserve_full_precision_without_notifications() {
    for read_only in [false, true] {
        let mut f = configured(read_only, 1.234567, Some(2));
        assert_eq!(text(&f), "1.234567");
        f.app.world_mut().resource_mut::<InputFocus>().clear();
        f.app.update();
        assert_eq!(text(&f), "1.23");
        f.app
            .world_mut()
            .resource_mut::<InputFocus>()
            .set(f.editor, FocusCause::Navigated);
        f.app.update();
        assert_eq!(text(&f), "1.234567");
        f.app
            .world_mut()
            .entity_mut(f.parent)
            .insert(InteractionDisabled);
        f.app.world_mut().flush();
        assert_eq!(text(&f), "1.23");
        f.app
            .world_mut()
            .entity_mut(f.parent)
            .remove::<InteractionDisabled>();
        f.app.world_mut().flush();
        f.app.world_mut().resource_mut::<InputFocus>().clear();
        f.app.update();
        assert_eq!(value(&f), 1.234567);
        assert_eq!(text(&f), "1.23");
        assert!(f.app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn initially_disabled_scenes_wait_for_internal_construction_without_host_errors() {
    for read_only in [false, true] {
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        let mut f = fixture(false);
        f.app
            .set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        f.app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let (root,inherited) = errors.run(|| {
            let root = spawn_scene(f.app.world_mut(), bsn! {
                @WidgetryNumberInput<f64> {@value:{Some(1.234567)},@read_only:{read_only},@decimals:{Some(2)}}
                InteractionDisabled
            }).unwrap();
            let ancestor = spawn_scene(f.app.world_mut(), bsn! {Node InteractionDisabled}).unwrap();
            let inherited = spawn_scene(f.app.world_mut(), bsn! {
                template(move |_| Ok(ChildOf(ancestor)))
                @WidgetryNumberInput<f64> {@value:{Some(1.234567)},@read_only:{read_only},@decimals:{Some(2)}}
            }).unwrap();
            f.app.update();
            (root,inherited)
        });
        assert!(errors.take().is_empty());
        for target in [root, inherited] {
            assert_eq!(
                WidgetryNumberInput::<f64>::value(f.app.world(), target).unwrap(),
                1.234567
            );
            assert!(f.app.world().get::<InteractionDisabled>(target).is_some());
            let field = f.app.world().get::<Children>(target).unwrap()[1];
            let get_text = |world: &World| {
                if read_only {
                    WidgetryReadOnlyTextField::text(world, field).unwrap()
                } else {
                    WidgetryTextField::text(world, field).unwrap()
                }
            };
            assert_eq!(get_text(f.app.world()), "1.23");
            assert!(
                WidgetryNumberInput::<f64>::set_value_in_world(f.app.world_mut(), target, 2.345678)
                    .unwrap()
            );
            assert_eq!(get_text(f.app.world()), "2.35");
        }
        assert_eq!(f.app.world().resource::<Changes>().0.len(), 2);
    }
}

#[test]
fn enabling_before_first_materialization_preserves_new_keyboard_input() {
    let mut f = fixture(false);
    f.root = spawn_scene(
        f.app.world_mut(),
        bsn! {
            @WidgetryNumberInput<f64> {@value:{Some(10.0)}} InteractionDisabled
        },
    )
    .unwrap();
    f.app
        .world_mut()
        .entity_mut(f.root)
        .remove::<InteractionDisabled>();
    f.app.world_mut().flush();
    f.field = f.app.world().get::<Children>(f.root).unwrap()[1];
    f.editor = f
        .app
        .world()
        .get::<WidgetryInputTarget>(f.field)
        .unwrap()
        .target;
    WidgetryTextField::set_text_in_world(f.app.world_mut(), f.field, "").unwrap();
    f.app
        .world_mut()
        .resource_mut::<InputFocus>()
        .set(f.editor, FocusCause::Navigated);
    key(
        &mut f,
        KeyCode::Digit2,
        Key::Character("2".into()),
        Some("25"),
    );
    assert_eq!(value(&f), 10.0);
    assert_eq!(text(&f), "25");
}

#[test]
fn reenable_after_keyboard_dispatch_drops_input_received_while_disabled() {
    for inherited in [false, true] {
        let mut f = fixture(false);
        let target = if inherited { f.parent } else { f.root };
        f.app
            .world_mut()
            .entity_mut(target)
            .insert(InteractionDisabled);
        f.app.world_mut().flush();
        f.app.add_systems(
            Update,
            (move |mut commands: Commands| {
                commands.entity(target).remove::<InteractionDisabled>();
            })
            .run_if(bevy::ecs::schedule::common_conditions::run_once),
        );
        key(
            &mut f,
            KeyCode::Digit2,
            Key::Character("2".into()),
            Some("2"),
        );
        assert_eq!(text(&f), "10.0");
        f.app.world_mut().resource_mut::<InputFocus>().clear();
        f.app.update();
        assert_eq!(value(&f), 10.0);
        assert!(f.app.world().resource::<Changes>().0.is_empty());
        f.app
            .world_mut()
            .resource_mut::<InputFocus>()
            .set(f.editor, FocusCause::Navigated);
        key(
            &mut f,
            KeyCode::Digit2,
            Key::Character("2".into()),
            Some("2"),
        );
        assert!(text(&f).contains('2'));
    }
}
