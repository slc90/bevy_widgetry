#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input_focus::{
    FocusCause, InputFocus,
    tab_navigation::{TabGroup, TabIndex},
};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_collapsible::{
    WidgetryCollapsible, WidgetryCollapsibleChanged, WidgetryCollapsiblePlugin,
};
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, press_key, primary_click, primary_press, primary_release, scene_app,
};

#[derive(Resource, Default)]
struct Changes(Vec<bool>);

fn fixture() -> (App, Entity, Entity, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<WidgetryCollapsibleChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                WidgetryCollapsible::expanded(&world, event.entity).unwrap(),
                event.expanded
            );
            world.resource_mut::<Changes>().0.push(event.expanded);
        },
    );
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let outer = app.world_mut().spawn_scene(bsn! {
        Node TabGroup::default() Children [
            @WidgetryCollapsible {
                @header: {Some(Box::new(bsn! { @bevy_widgetry_button::WidgetryButton }) as Box<dyn Scene>)},
                @children: {Some(Box::new(bsn! { Node Children [Node TabIndex(0)] }) as Box<dyn SceneList>)},
            }
            -- Node TabIndex(0)
        ]
    }).unwrap().id();
    app.update();
    let root = app.world().get::<Children>(outer).unwrap()[0];
    let header = app.world().get::<Children>(root).unwrap()[0];
    let content = app.world().get::<Children>(root).unwrap()[1];
    let button = app.world().get::<Children>(header).unwrap()[1];
    let custom = app
        .world()
        .get::<Children>(app.world().get::<Children>(header).unwrap()[0])
        .unwrap()[0];
    let editor = app
        .world()
        .get::<Children>(app.world().get::<Children>(content).unwrap()[0])
        .unwrap()[0];
    (app, window, root, button, custom, editor)
}

fn click(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().flush();
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().flush();
}

#[test]
fn only_the_fixed_button_toggles_and_disabled_user_activation_is_ignored() {
    let (mut app, _, root, button, custom, _) = fixture();
    click(&mut app, custom);
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    assert!(app.world().resource::<Changes>().0.is_empty());
    click(&mut app, button);
    assert!(!WidgetryCollapsible::expanded(app.world(), root).unwrap());
    click(&mut app, button);
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    click(&mut app, button);
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<Changes>().0, vec![false, true]);
}

#[test]
fn tab_navigation_and_native_enter_space_activate_the_same_state() {
    let (mut app, window, root, button, _, _) = fixture();
    press_key(&mut app, window, KeyCode::Tab);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    press_key(&mut app, window, KeyCode::Enter);
    assert!(!WidgetryCollapsible::expanded(app.world(), root).unwrap());
    press_key(&mut app, window, KeyCode::Space);
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    press_key(&mut app, window, KeyCode::ArrowDown);
    bevy_widgetry_test_utils::queue_key(
        &mut app,
        bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: bevy::input::keyboard::Key::Enter,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: true,
            window,
        },
    );
    app.update();
    assert_eq!(app.world().resource::<Changes>().0, vec![false, true]);
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap();
    press_key(&mut app, window, KeyCode::Tab);
    let outer = app.world().get::<ChildOf>(root).unwrap().parent();
    let outside = app.world().get::<Children>(outer).unwrap()[1];
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(outside));
}

#[test]
fn hidden_toggle_cannot_receive_content_focus_or_activate() {
    let (mut app, _, root, button, _, editor) = fixture();
    app.world_mut().entity_mut(root).insert(Visibility::Hidden);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: button });
    app.world_mut().flush();
    assert!(!WidgetryCollapsible::expanded(app.world(), root).unwrap());
}

#[test]
fn public_collapse_moves_nested_focus_clears_when_disabled_and_preserves_outside_focus() {
    let (mut app, _, root, button, _, _) = fixture();
    let content = app.world().get::<Children>(root).unwrap()[1];
    let editor = app
        .world()
        .get::<Children>(app.world().get::<Children>(content).unwrap()[0])
        .unwrap()[0];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, true).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    let outside = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(outside, FocusCause::Navigated);
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, true).unwrap();
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(outside));
}
