#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use accesskit::{Role, Toggled};
use bevy::{
    a11y::AccessibilityNode,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    picking::pointer::*,
    prelude::*,
    ui::{Checked, InteractionDisabled, Pressed},
    ui_widgets::{Checkbox, ValueChange},
};
use bevy_widgetry_check_box::{WidgetryCheckBoxHeadless, WidgetryCheckBoxHeadlessPlugin};
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, picking_app, pointer_ids, primary_click, queue_key, queue_pointer,
    spawn_picking_camera,
};

#[derive(Resource, Default)]
struct Requests(Vec<bool>);

fn fixture(id: PointerId) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryCheckBoxHeadlessPlugin)
        .init_resource::<Requests>();
    app.add_observer(
        |event: On<ValueChange<bool>>,
         roots: Query<(), With<WidgetryCheckBoxHeadless>>,
         mut requests: ResMut<Requests>| {
            if roots.contains(event.source) {
                requests.0.push(event.value);
            }
        },
    );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 200).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::new(400, 200), 1.0);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryCheckBoxHeadless
        Node { width: px(96), height: px(36), position_type: PositionType::Absolute, left: px(40), top: px(40) }
        BackgroundColor({Color::srgb_u8(12,34,56)})
        template(move |_| Ok(UiTargetCamera(camera)))
    }).unwrap().id();
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    app.update();
    app.update();
    assert!(!app.is_plugin_added::<bevy_widgetry_core::icon::WidgetryIconPlugin>());
    assert!(app.world().get::<Children>(root).is_none());
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(45.0, 45.0),
    };
    (app, root, window, location)
}

fn pointer(app: &mut App, id: PointerId, at: &Location, action: PointerAction) {
    queue_pointer(app, id, at.clone(), action);
    app.update();
}

#[test]
fn independent_binary_behavior_uses_actual_picking_without_default_indicator() {
    for id in pointer_ids() {
        let (mut app, root, _, at) = fixture(id);
        pointer(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        pointer(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().get::<Pressed>(root).is_some());
        pointer(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(app.world().get::<Checked>(root).is_some());
        assert_eq!(app.world().resource::<Requests>().0, [true]);
        assert_eq!(
            app.world().get::<AccessibilityNode>(root).unwrap().role(),
            Role::CheckBox
        );
        assert_eq!(
            app.world()
                .get::<AccessibilityNode>(root)
                .unwrap()
                .toggled(),
            Some(Toggled::True)
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            Color::srgb_u8(12, 34, 56)
        );
        app.world_mut().entity_mut(root).remove::<Checked>();
        app.update();
        assert_eq!(app.world().resource::<Requests>().0, [true]);
        app.add_plugins(bevy_widgetry_check_box::WidgetryCheckBoxPlugin);
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        pointer(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        pointer(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(app.world().get::<Checked>(root).is_some());
        assert_eq!(app.world().resource::<Requests>().0, [true, true]);
        assert!(
            app.world()
                .get::<bevy_widgetry_check_box::WidgetryCheckBox>(root)
                .is_none()
        );
        assert!(app.world().get::<Children>(root).is_none());
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            Color::srgb_u8(12, 34, 56)
        );
        let native = app.world_mut().spawn((Node::default(), Checkbox)).id();
        app.world_mut().trigger(primary_click(native));
        app.world_mut().flush();
        assert!(app.world().get::<Checked>(native).is_none());
    }
}

#[test]
fn binary_keyboard_and_disabled_inheritance_preserve_native_checked_permissions() {
    let (mut app, root, window, _) = fixture(PointerId::Mouse);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    for (code, state, repeat, expected) in [
        (KeyCode::Enter, ButtonState::Pressed, false, true),
        (KeyCode::Enter, ButtonState::Pressed, true, true),
        (KeyCode::Enter, ButtonState::Released, false, true),
        (KeyCode::Space, ButtonState::Pressed, false, false),
    ] {
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: code,
                logical_key: Key::Enter,
                state,
                text: None,
                repeat,
                window,
            },
        );
        app.update();
        assert_eq!(app.world().get::<Checked>(root).is_some(), expected);
    }
    assert_eq!(app.world().resource::<Requests>().0, [true, false]);
    app.world_mut().resource_mut::<InputFocus>().clear();
    queue_key(
        &mut app,
        KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: Key::Space,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    app.update();
    assert!(app.world().get::<Checked>(root).is_none());
    assert_eq!(app.world().resource::<Requests>().0, [true, false]);
    let parent = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    app.world_mut().entity_mut(root).insert(ChildOf(parent));
    app.update();
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    assert!(app.world().get::<Checked>(root).is_none());
    app.world_mut()
        .entity_mut(root)
        .insert((Checked, InteractionDisabled));
    app.world_mut()
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
    app.update();
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    assert!(app.world().get::<Checked>(root).is_some());
    assert_eq!(app.world().resource::<Requests>().0, [true, false]);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    assert!(app.world().get::<Checked>(root).is_none());
    assert_eq!(app.world().resource::<Requests>().0, [true, false, false]);
}
