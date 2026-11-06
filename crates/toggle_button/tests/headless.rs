#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use accesskit::{Role, Toggled};
use bevy::{
    a11y::AccessibilityNode,
    prelude::*,
    ui::{Checked, InteractionDisabled},
};
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, add_ui_plugins, primary_click, queue_key, scene_app,
};
use bevy_widgetry_toggle_button::{
    WidgetryToggleButtonChanged, WidgetryToggleButtonHeadless, WidgetryToggleButtonHeadlessPlugin,
};

#[derive(Resource, Default)]
struct Changes(Vec<bool>);

fn fixture(selected: bool) -> (App, Entity) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins(WidgetryToggleButtonHeadlessPlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<WidgetryToggleButtonChanged>,
         states: Query<Has<Checked>>,
         mut changes: ResMut<Changes>| {
            assert_eq!(states.get(event.entity).unwrap(), event.selected);
            changes.0.push(event.selected);
        },
    );
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryToggleButtonHeadless { @selected: selected }
            BackgroundColor({Color::srgb_u8(12, 34, 56)})
        })
        .unwrap()
        .id();
    app.update();
    (app, root)
}

#[test]
fn headless_input_and_api_commit_once_without_default_style() {
    let (mut app, root) = fixture(false);
    assert!(
        app.world()
            .get::<bevy_widgetry_toggle_button::WidgetryToggleButton>(root)
            .is_none()
    );
    assert!(app.world().get::<Children>(root).is_none());
    assert!(app.world().resource::<Changes>().0.is_empty());
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    app.update();
    assert!(WidgetryToggleButtonHeadless::selected(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<Changes>().0, [true]);
    assert_eq!(
        app.world().get::<AccessibilityNode>(root).unwrap().role(),
        Role::Button
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
    assert!(
        app.world()
            .get::<bevy_widgetry_core::foreground::ResolvedForeground>(root)
            .is_none()
    );
    assert!(
        !WidgetryToggleButtonHeadless::set_selected_in_world(app.world_mut(), root, true).unwrap()
    );
    app.world_mut().entity_mut(root).remove::<Checked>();
    app.update();
    assert!(!WidgetryToggleButtonHeadless::selected(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<Changes>().0, [true]);
    WidgetryToggleButtonHeadless::set_selected(&mut app.world_mut().commands(), root, true);
    assert!(!WidgetryToggleButtonHeadless::selected(app.world(), root).unwrap());
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Changes>().0, [true, true]);
    app.add_plugins(bevy_widgetry_toggle_button::WidgetryToggleButtonPlugin);
    app.update();
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    app.update();
    assert_eq!(app.world().resource::<Changes>().0, [true, true, false]);
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::srgb_u8(12, 34, 56)
    );
}

#[test]
fn queued_setter_rejects_a_target_that_lost_its_behavior_identity() {
    use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
    use bevy_widgetry_test_utils::ErrorCapture;
    use bevy_widgetry_toggle_button::ToggleButtonError;
    let (mut app, root) = fixture(false);
    app.set_error_handler(ErrorCapture::handler());
    let capture = ErrorCapture::default();
    {
        let mut commands = app.world_mut().commands();
        commands
            .entity(root)
            .remove::<WidgetryToggleButtonHeadless>();
        WidgetryToggleButtonHeadless::set_selected(&mut commands, root, true);
    }
    capture.run(|| app.world_mut().flush());
    let records = capture.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let diagnostic = records[0]
        .error
        .downcast_ref::<WidgetryEcsError>()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap();
    assert!(
        matches!(diagnostic.source().unwrap().downcast_ref::<ToggleButtonError>(), Some(ToggleButtonError::InvalidTarget { root: target }) if *target == root)
    );
    assert!(app.world().get::<Checked>(root).is_none());
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn inherited_and_local_disabled_block_input_but_allow_programmatic_updates() {
    let (mut app, root) = fixture(true);
    let parent = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    app.world_mut().entity_mut(root).insert(ChildOf(parent));
    app.update();
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    assert!(app.world().resource::<Changes>().0.is_empty());
    WidgetryToggleButtonHeadless::set_selected_in_world(app.world_mut(), root, false).unwrap();
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut()
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
    app.update();
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    assert!(!WidgetryToggleButtonHeadless::selected(app.world(), root).unwrap());
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    assert!(WidgetryToggleButtonHeadless::selected(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<Changes>().0, [false, true]);
}

#[test]
fn headless_keyboard_requires_focus_and_ignores_repeat_release_and_other_keys() {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    use bevy::input_focus::{FocusCause, InputFocus};
    let (mut app, root) = fixture(false);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    for (key, state, repeat, expected) in [
        (KeyCode::Enter, ButtonState::Pressed, false, true),
        (KeyCode::Enter, ButtonState::Pressed, true, true),
        (KeyCode::Enter, ButtonState::Released, false, true),
        (KeyCode::KeyA, ButtonState::Pressed, false, true),
        (KeyCode::Space, ButtonState::Pressed, false, false),
    ] {
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: key,
                logical_key: Key::Enter,
                state,
                text: None,
                repeat,
                window,
            },
        );
        app.update();
        assert_eq!(
            WidgetryToggleButtonHeadless::selected(app.world(), root).unwrap(),
            expected
        );
    }
    assert_eq!(app.world().resource::<Changes>().0, [true, false]);
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
    assert!(!WidgetryToggleButtonHeadless::selected(app.world(), root).unwrap());
}
