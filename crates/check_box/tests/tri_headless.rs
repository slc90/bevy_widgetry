#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use accesskit::Toggled;
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
    ui_widgets::ValueChange,
};
use bevy_widgetry_check_box::{
    WidgetryCheckState, WidgetryTriStateCheckboxHeadless, WidgetryTriStateCheckboxHeadlessPlugin,
};
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, picking_app, pointer_ids, queue_key, queue_pointer, spawn_picking_camera,
};

#[derive(Resource, Default)]
struct Changes(Vec<WidgetryCheckState>);

fn fixture(id: PointerId) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryTriStateCheckboxHeadlessPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<WidgetryCheckState>>,
         states: Query<&WidgetryCheckState>,
         mut changes: ResMut<Changes>| {
            assert_eq!(*states.get(event.source).unwrap(), event.value);
            changes.0.push(event.value);
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
    let root=app.world_mut().spawn_scene(bsn! {
        @WidgetryTriStateCheckboxHeadless
        Node { width: px(80), height: px(48), position_type: PositionType::Absolute, left: px(40), top: px(40) }
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
    assert!(app.world().resource::<Changes>().0.is_empty());
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(45.0, 45.0),
    };
    (app, root, window, at)
}
fn pointer(app: &mut App, id: PointerId, at: &Location, action: PointerAction) {
    queue_pointer(app, id, at.clone(), action);
    app.update();
}
fn state(app: &App, root: Entity) -> WidgetryCheckState {
    *app.world().get::<WidgetryCheckState>(root).unwrap()
}

#[test]
fn activate_on_press_is_one_shot_and_cancel_clears_pressed_without_an_indicator() {
    for id in pointer_ids() {
        let (mut app, root, _, at) = fixture(id);
        app.world_mut()
            .entity_mut(root)
            .insert(bevy::ui_widgets::ActivateOnPress);
        pointer(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        pointer(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert_eq!(state(&app, root), WidgetryCheckState::Checked);
        pointer(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert_eq!(
            app.world().resource::<Changes>().0,
            [WidgetryCheckState::Checked]
        );
        pointer(&mut app, id, &at, PointerAction::Cancel);
        assert!(app.world().get::<Pressed>(root).is_none());
        assert_eq!(state(&app, root), WidgetryCheckState::Checked);
    }
}

#[test]
fn mounting_behavior_on_an_existing_state_initializes_accessibility_without_notification() {
    let (mut app, _, _, _) = fixture(PointerId::Mouse);
    let root = app
        .world_mut()
        .spawn(WidgetryCheckState::Indeterminate)
        .id();
    app.update();
    app.world_mut()
        .entity_mut(root)
        .apply_scene(bsn! { @WidgetryTriStateCheckboxHeadless })
        .unwrap();
    app.update();
    assert_eq!(state(&app, root), WidgetryCheckState::Indeterminate);
    assert_eq!(
        app.world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .toggled(),
        Some(Toggled::Mixed)
    );
    assert!(app.world().get::<Checked>(root).is_none());
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn three_picking_cycles_commit_and_accessibility_works_without_indicator_or_checked() {
    for id in pointer_ids() {
        let (mut app, root, _, at) = fixture(id);
        pointer(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
        for (expected, toggled) in [
            (WidgetryCheckState::Checked, Toggled::True),
            (WidgetryCheckState::Indeterminate, Toggled::Mixed),
            (WidgetryCheckState::Unchecked, Toggled::False),
        ] {
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
            assert_eq!(state(&app, root), expected);
            assert_eq!(
                app.world()
                    .get::<AccessibilityNode>(root)
                    .unwrap()
                    .toggled(),
                Some(toggled)
            );
            assert!(app.world().get::<Checked>(root).is_none());
            assert!(app.world().get::<Pressed>(root).is_none());
        }
        assert_eq!(
            app.world().resource::<Changes>().0,
            [
                WidgetryCheckState::Checked,
                WidgetryCheckState::Indeterminate,
                WidgetryCheckState::Unchecked
            ]
        );
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
        assert_eq!(app.world().resource::<Changes>().0.len(), 4);
        assert!(app.world().get::<Children>(root).is_none());
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            Color::srgb_u8(12, 34, 56)
        );
    }
}

#[test]
fn queued_set_cycle_same_value_and_disabled_preserve_single_state_authority() {
    let (mut app, root, _, _) = fixture(PointerId::Mouse);
    for _ in 0..3 {
        WidgetryTriStateCheckboxHeadless::cycle_state(&mut app.world_mut().commands(), root);
    }
    assert_eq!(state(&app, root), WidgetryCheckState::Unchecked);
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Changes>().0,
        [
            WidgetryCheckState::Checked,
            WidgetryCheckState::Indeterminate,
            WidgetryCheckState::Unchecked
        ]
    );
    WidgetryTriStateCheckboxHeadless::set_state(
        &mut app.world_mut().commands(),
        root,
        WidgetryCheckState::Unchecked,
    );
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Changes>().0.len(), 3);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    WidgetryTriStateCheckboxHeadless::set_state(
        &mut app.world_mut().commands(),
        root,
        WidgetryCheckState::Indeterminate,
    );
    app.world_mut().flush();
    app.update();
    assert_eq!(state(&app, root), WidgetryCheckState::Indeterminate);
    assert_eq!(
        app.world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .toggled(),
        Some(Toggled::Mixed)
    );
    assert!(app.world().get::<Checked>(root).is_none());
    assert_eq!(app.world().resource::<Changes>().0.len(), 4);
}

#[test]
fn headless_keyboard_focus_repeat_and_inherited_disabled_guards() {
    let (mut app, root, window, _) = fixture(PointerId::Mouse);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    for (key, repeat, expected) in [
        (KeyCode::Enter, false, WidgetryCheckState::Checked),
        (KeyCode::Enter, true, WidgetryCheckState::Checked),
        (KeyCode::Space, false, WidgetryCheckState::Indeterminate),
    ] {
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: key,
                logical_key: Key::Enter,
                state: ButtonState::Pressed,
                text: None,
                repeat,
                window,
            },
        );
        app.update();
        assert_eq!(state(&app, root), expected);
    }
    let parent = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    app.world_mut().entity_mut(root).insert(ChildOf(parent));
    app.update();
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
    assert_eq!(state(&app, root), WidgetryCheckState::Indeterminate);
    app.world_mut()
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
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
    assert_eq!(state(&app, root), WidgetryCheckState::Indeterminate);
    assert_eq!(
        app.world().resource::<Changes>().0,
        [
            WidgetryCheckState::Checked,
            WidgetryCheckState::Indeterminate
        ]
    );
}
