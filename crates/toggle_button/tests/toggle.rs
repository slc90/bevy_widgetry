#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_toggle_button::{WidgetryToggleButton, WidgetryToggleButtonChanged};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, bool)>);
fn record(
    event: On<WidgetryToggleButtonChanged>,
    query: Query<Has<bevy::ui::Checked>>,
    mut changes: ResMut<Changes>,
) {
    assert_eq!(query.get(event.entity).unwrap(), event.selected);
    changes.0.push((event.entity, event.selected));
}
#[test]
fn setter_commits_before_notification_and_same_value_is_silent() {
    let mut app = App::new();
    app.init_resource::<Changes>().add_observer(record);
    let root = app.world_mut().spawn(WidgetryToggleButton).id();
    assert!(WidgetryToggleButton::set_selected_in_world(app.world_mut(), root, true).unwrap());
    assert!(!WidgetryToggleButton::set_selected_in_world(app.world_mut(), root, true).unwrap());
    WidgetryToggleButton::set_selected(&mut app.world_mut().commands(), root, false);
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Changes>().0,
        [(root, true), (root, false)]
    );
}

use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
use bevy::picking::hover::Hovered;
use bevy::ui::{Checked, InteractionDisabled, Pressed};
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, add_ui_plugins, cancel, primary_click, primary_press, primary_release,
    queue_key, scene_app, switch_theme,
};
use bevy_widgetry_theme::WidgetryThemeMode;
use bevy_widgetry_toggle_button::{WidgetryToggleButtonColorOverrides, WidgetryToggleButtonPlugin};

fn toggle_app(selected: bool) -> (App, Entity) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins(WidgetryToggleButtonPlugin)
        .init_resource::<Changes>()
        .add_observer(record);
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryToggleButton { @selected: selected } Children [Text("Toggle") bevy_widgetry_core::text::WidgetryText] }).unwrap().id();
    app.update();
    (app, root)
}

#[test]
fn native_input_toggles_persistently_and_disabled_inherits() {
    let (mut app, root) = toggle_app(false);
    assert_eq!(
        app.world().get::<AccessibilityNode>(root).unwrap().role(),
        Role::Button
    );
    assert_eq!(
        app.world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .toggled(),
        Some(Toggled::False)
    );
    app.world_mut().trigger(primary_press(root));
    app.world_mut().flush();
    assert!(!WidgetryToggleButton::selected(app.world(), root).unwrap());
    assert!(app.world().get::<Pressed>(root).is_some());
    app.world_mut().trigger(primary_release(root));
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    app.update();
    assert!(WidgetryToggleButton::selected(app.world(), root).unwrap());
    assert_eq!(
        app.world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .toggled(),
        Some(Toggled::True)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        WidgetryThemeMode::Dark
            .colors()
            .check_box
            .checked
            .normal
            .background
    );
    assert!(app.world().get::<Pressed>(root).is_none());
    let parent = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    app.world_mut().entity_mut(root).insert(ChildOf(parent));
    app.update();
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    assert!(WidgetryToggleButton::selected(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
    assert!(
        app.world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .is_disabled()
    );
    // Explicit programmatic updates remain available while user input is disabled.
    WidgetryToggleButton::set_selected_in_world(app.world_mut(), root, false).unwrap();
    app.world_mut()
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
    app.update();
    app.world_mut().trigger(primary_press(root));
    app.world_mut().flush();
    cancel(&mut app, root);
    app.update();
    assert!(app.world().get::<Pressed>(root).is_none());
    assert!(!WidgetryToggleButton::selected(app.world(), root).unwrap());
    app.world_mut().trigger(primary_click(root));
    app.world_mut().flush();
    assert!(WidgetryToggleButton::selected(app.world(), root).unwrap());
}

#[test]
fn keyboard_focus_repeat_release_and_disabled_guards() {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput, NativeKey},
    };
    use bevy::input_focus::{FocusCause, InputFocus};
    let (mut app, root) = toggle_app(false);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Pressed);
    for (key, state, repeat, disabled, expected) in [
        (KeyCode::Space, ButtonState::Pressed, false, false, true),
        (KeyCode::Enter, ButtonState::Pressed, true, false, true),
        (KeyCode::Space, ButtonState::Released, false, false, true),
        (KeyCode::KeyX, ButtonState::Pressed, false, false, true),
        (KeyCode::Space, ButtonState::Pressed, false, true, true),
        (KeyCode::Enter, ButtonState::Pressed, false, false, false),
    ] {
        if disabled {
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
        } else {
            app.world_mut()
                .entity_mut(root)
                .remove::<InteractionDisabled>();
        }
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: key,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat,
                window,
            },
        );
        app.update();
        assert_eq!(
            WidgetryToggleButton::selected(app.world(), root).unwrap(),
            expected
        );
    }
    assert_eq!(
        app.world().resource::<Changes>().0,
        [(root, true), (root, false)]
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    queue_key(
        &mut app,
        KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    app.update();
    assert!(!WidgetryToggleButton::selected(app.world(), root).unwrap());
}

#[test]
fn selected_theme_overrides_and_native_checked_removal_refresh_content() {
    let (mut app, root) = toggle_app(true);
    let child = app.world().get::<Children>(root).unwrap()[0];
    for mode in [
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::Light,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        switch_theme(&mut app, mode);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            mode.colors().check_box.checked.normal.background
        );
        assert_eq!(
            app.world().get::<TextColor>(child).unwrap().0,
            mode.colors().check_box.checked.normal.mark
        );
    }
    let mut colors = WidgetryToggleButtonColorOverrides::default();
    colors.selected.background = Some(Color::BLACK);
    colors.selected_hovered.background = Some(Color::WHITE);
    colors.selected_pressed.background = Some(Color::srgb(1.0, 0.0, 0.0));
    colors.selected_disabled.background = Some(Color::srgb(0.0, 1.0, 0.0));
    WidgetryToggleButtonColorOverrides::set_in_world(app.world_mut(), root, colors.clone())
        .unwrap();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::BLACK
    );
    app.world_mut().entity_mut(root).insert(Hovered(true));
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::WHITE
    );
    app.world_mut().entity_mut(root).insert(Pressed);
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::srgb(1.0, 0.0, 0.0)
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::srgb(0.0, 1.0, 0.0)
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<(InteractionDisabled, Pressed, Checked)>();
    app.world_mut().run_schedule(PostUpdate);
    let mode = *app.world().resource::<WidgetryThemeMode>();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        mode.colors().button.hovered.background
    );
    assert_eq!(
        app.world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .toggled(),
        Some(Toggled::False)
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    colors.normal.background = Some(Color::srgba(f32::NAN, 0.0, 0.0, 1.0));
    assert!(
        WidgetryToggleButtonColorOverrides::set_in_world(app.world_mut(), root, colors).is_err()
    );
    assert!(WidgetryToggleButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
    assert!(!WidgetryToggleButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
}

#[test]
fn invalid_targets_are_errors_and_queued_setters_validate_at_execution() {
    let (mut app, root) = toggle_app(false);
    let wrong = app.world_mut().spawn_empty().id();
    assert!(WidgetryToggleButton::selected(app.world(), wrong).is_err());
    assert!(WidgetryToggleButton::set_selected_in_world(app.world_mut(), wrong, true).is_err());
    WidgetryToggleButton::set_selected(&mut app.world_mut().commands(), root, true);
    assert!(!WidgetryToggleButton::selected(app.world(), root).unwrap());
    app.world_mut().flush();
    assert!(WidgetryToggleButton::selected(app.world(), root).unwrap());
    app.world_mut().despawn(root);
    assert!(WidgetryToggleButton::set_selected_in_world(app.world_mut(), root, false).is_err());
}
