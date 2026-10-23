#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    picking::pointer::{Location, PointerAction, PointerButton},
    prelude::*,
    text::EditableText,
    time::TimeUpdateStrategy,
    ui_widgets::ValueChange,
};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::{WidgetryAppExt, input_target::WidgetryInputTarget, scene::spawn_scene};
use bevy_widgetry_number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin};
use bevy_widgetry_text_field::WidgetryTextField;
use std::time::Duration;

#[derive(Resource, Default)]
struct Changes(Vec<f64>);

struct Fixture {
    app: App,
    root: Entity,
    parent: Entity,
    field: Entity,
    frame: Entity,
    editor: Entity,
    decrease: Entity,
    increase: Entity,
    window: Entity,
}

fn fixture(value: f64, read_only: bool) -> Fixture {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.add_plugins(WidgetryAssetPlugin);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    bevy_widgetry_test_utils::advance_until(
        &mut app,
        Duration::from_secs(10),
        "NumberInput layout font",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<f64>>, mut changes: ResMut<Changes>| changes.0.push(event.value),
    );
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera =
        bevy_widgetry_test_utils::spawn_picking_camera(&mut app, window, UVec2::splat(600), 1.0);
    let parent=spawn_scene(app.world_mut(),bsn!{Node{width:px(400),height:px(100),align_items:AlignItems::Start,position_type:PositionType::Absolute,left:px(40),top:px(40)} template(move |_|Ok(UiTargetCamera(camera)))}).unwrap();
    let root = spawn_scene(
        app.world_mut(),
        bsn! {@WidgetryNumberInput<f64>{@value:{Some(value)},@step:3.0,@read_only:{read_only}}},
    )
    .unwrap();
    app.world_mut().entity_mut(parent).add_child(root);
    let children = app.world().get::<Children>(root).unwrap();
    let (decrease, field, increase) = (children[0], children[1], children[2]);
    let frame = bevy_widgetry_text_field::internal::frame(app.world(), field).unwrap();
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    for _ in 0..6 {
        app.update();
    }
    Fixture {
        app,
        root,
        parent,
        field,
        frame,
        editor,
        decrease,
        increase,
        window,
    }
}

fn size(f: &Fixture, entity: Entity) -> Vec2 {
    f.app.world().get::<ComputedNode>(entity).unwrap().size()
}

fn settle(f: &mut Fixture) {
    for _ in 0..3 {
        f.app.update();
    }
}

fn key(f: &mut Fixture, code: KeyCode, logical: Key) {
    bevy_widgetry_test_utils::queue_key(
        &mut f.app,
        KeyboardInput {
            key_code: code,
            logical_key: logical,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: f.window,
        },
    );
    f.app.update();
}

#[test]
fn resizing_keeps_equal_buttons_and_a_usable_minimum_editor_width() {
    for read_only in [false, true] {
        let mut f = fixture(10.0, read_only);
        let buttons = [size(&f, f.decrease).x, size(&f, f.increase).x];
        assert_eq!(buttons[0], buttons[1]);
        assert!(size(&f, f.root).y <= buttons[0] + 4.0);
        let middle = size(&f, f.field).x;
        f.app.world_mut().get_mut::<Node>(f.root).unwrap().width = px(300);
        settle(&mut f);
        assert_eq!([size(&f, f.decrease).x, size(&f, f.increase).x], buttons);
        assert!(size(&f, f.field).x > middle + 90.0);
        f.app.world_mut().get_mut::<Node>(f.parent).unwrap().width = px(50);
        f.app.world_mut().get_mut::<Node>(f.root).unwrap().width = percent(100);
        settle(&mut f);
        assert!(size(&f, f.root).x > size(&f, f.parent).x);
        assert_eq!([size(&f, f.decrease).x, size(&f, f.increase).x], buttons);
        assert!(size(&f, f.editor).x >= 36.0);
        assert_eq!(
            f.app.world().get::<TextLayout>(f.editor).unwrap().justify,
            Justify::Center
        );
        assert_eq!(
            WidgetryNumberInput::<f64>::value(f.app.world(), f.root).unwrap(),
            10.0
        );
        assert!(f.app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn one_outer_shell_and_two_separators_keep_draft_step_focus_at_minimum_width() {
    let mut f = fixture(10.0, false);
    assert!(f.app.world().get::<Node>(f.root).unwrap().border.left != px(0));
    assert_eq!(
        f.app.world().get::<Node>(f.frame).unwrap().border,
        UiRect::ZERO
    );
    assert!(f.app.world().get::<Node>(f.decrease).unwrap().border.right != px(0));
    assert!(f.app.world().get::<Node>(f.increase).unwrap().border.left != px(0));
    f.app.world_mut().get_mut::<Node>(f.root).unwrap().width = px(1);
    settle(&mut f);
    WidgetryTextField::set_text_in_world(f.app.world_mut(), f.field, "25").unwrap();
    f.app
        .world_mut()
        .resource_mut::<InputFocus>()
        .set(f.editor, FocusCause::Navigated);
    f.app.update();
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    f.app.world_mut().spawn(pointer);
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(f.window))
            .normalize(None)
            .unwrap(),
        position: f
            .app
            .world()
            .get::<UiGlobalTransform>(f.increase)
            .unwrap()
            .translation,
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
    ] {
        bevy_widgetry_test_utils::queue_pointer(&mut f.app, pointer, at.clone(), action);
        f.app.update();
    }
    assert_eq!(
        WidgetryNumberInput::<f64>::value(f.app.world(), f.root).unwrap(),
        28.0
    );
    f.app
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            400,
        )));
    f.app.update();
    assert_eq!(
        WidgetryNumberInput::<f64>::value(f.app.world(), f.root).unwrap(),
        31.0
    );
    assert_eq!(f.app.world().resource::<InputFocus>().get(), Some(f.editor));
    assert_eq!(f.app.world().resource::<Changes>().0, vec![28.0, 31.0]);
}

#[test]
fn long_numbers_keep_the_official_text_viewport_and_stable_button_geometry() {
    let mut f = fixture(f64::MAX, false);
    f.app.world_mut().get_mut::<Node>(f.root).unwrap().width = px(1);
    settle(&mut f);
    let buttons = [size(&f, f.decrease).x, size(&f, f.increase).x];
    let length = WidgetryTextField::text(f.app.world(), f.field)
        .unwrap()
        .len();
    f.app
        .world_mut()
        .resource_mut::<InputFocus>()
        .set(f.editor, FocusCause::Navigated);
    key(&mut f, KeyCode::Home, Key::Home);
    assert_eq!(
        f.app
            .world()
            .get::<EditableText>(f.editor)
            .unwrap()
            .editor()
            .raw_selection()
            .focus()
            .index(),
        0
    );
    key(&mut f, KeyCode::End, Key::End);
    let edit = f.app.world().get::<EditableText>(f.editor).unwrap();
    assert_eq!(edit.editor().raw_selection().focus().index(), length);
    assert!(edit.viewport.size.x > 0.0);
    assert!(edit.viewport.offset.x > 0.0);
    assert!(size(&f, f.editor).x < size(&f, f.root).x - buttons[0] - buttons[1]);
    assert_eq!([size(&f, f.decrease).x, size(&f, f.increase).x], buttons);
    assert_eq!(
        WidgetryNumberInput::<f64>::value(f.app.world(), f.root).unwrap(),
        f64::MAX
    );
    assert!(f.app.world().resource::<Changes>().0.is_empty());
}
