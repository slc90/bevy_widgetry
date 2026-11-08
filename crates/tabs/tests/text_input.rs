#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy_widgetry_tabs::{WidgetryTab, WidgetryTabs, WidgetryTabsPlugin};
use bevy_widgetry_test_utils::{add_keyboard_dispatch, primary_click, text_edit_app};
use bevy_widgetry_text_field::WidgetryTextFieldHeadless;

#[test]
fn disabling_a_selected_header_keeps_its_editor_enabled_but_root_disabled_blocks_edits() {
    let mut app = text_edit_app();
    app.add_plugins((WidgetryTabsPlugin, WidgetryTextFieldPlugin));
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let header: Box<dyn Scene> = Box::new(bsn! { Text("Header") });
    let content: Box<dyn Scene> = Box::new(bsn! { @WidgetryTextField });
    let items: Box<dyn SceneList> =
        Box::new(bsn_list! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } });
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTabs { @children: {Some(items)} } })
        .unwrap()
        .id();
    app.update();
    let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
    let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
    let field = app.world().get::<Children>(page).unwrap()[0];
    app.world_mut()
        .entity_mut(header)
        .insert(bevy::ui::InteractionDisabled);
    WidgetryTextFieldHeadless::focus_in_world(app.world_mut(), field).unwrap();
    let key = || KeyboardInput {
        key_code: KeyCode::KeyX,
        logical_key: Key::Character("x".into()),
        text: Some("x".into()),
        state: ButtonState::Pressed,
        repeat: false,
        window,
    };
    app.world_mut().write_message(key());
    app.update();
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "x"
    );
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    app.world_mut().write_message(key());
    app.update();
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "x"
    );
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
    assert_eq!(
        app.world().get::<Node>(page).unwrap().display,
        Display::Flex
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<bevy::ui::InteractionDisabled>();
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(header)
            .is_some()
    );
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(field)
            .is_none()
    );
}

#[test]
fn pointer_focus_and_typing_follow_the_active_page() {
    let mut app = text_edit_app();
    app.add_plugins((WidgetryTabsPlugin, WidgetryTextFieldPlugin));
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let items: Vec<Box<dyn Scene>> = (0..2)
        .map(|_| {
            let header: Box<dyn Scene> = Box::new(bsn! { Text("Header") });
            let content: Box<dyn Scene> = Box::new(bsn! { @WidgetryTextField });
            Box::new(bsn! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } })
                as Box<dyn Scene>
        })
        .collect();
    let items: Box<dyn SceneList> = Box::new(items);
    let container = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTabs { @children: {Some(items)} }
        })
        .unwrap()
        .id();
    app.update();
    let root = container;
    for selected in [0, 1] {
        WidgetryTabs::set_selected_in_world(app.world_mut(), root, selected).unwrap();
        if selected != 0 {
            assert_eq!(app.world().resource::<InputFocus>().get(), None);
        }
        let page = WidgetryTabs::page(app.world(), root, selected).unwrap();
        let field = app.world().get::<Children>(page).unwrap()[0];
        let editor = app
            .world()
            .get::<bevy_widgetry_core::input_target::WidgetryInputTarget>(field)
            .unwrap()
            .target;
        app.world_mut()
            .trigger(bevy_widgetry_test_utils::primary_press(editor));
        app.world_mut().flush();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyX,
            logical_key: Key::Character("x".into()),
            text: Some("x".into()),
            state: ButtonState::Pressed,
            repeat: false,
            window,
        });
        app.update();
        assert_eq!(
            WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
            "x"
        );
    }
}
use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};

#[test]
fn hidden_text_field_stops_receiving_keys_and_returns_with_its_input_intact() {
    let mut app = text_edit_app();
    app.add_plugins((WidgetryTabsPlugin, WidgetryTextFieldPlugin));
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let items: Vec<Box<dyn Scene>> = (0..2)
        .map(|index| {
            let header: Box<dyn Scene> = Box::new(bsn! { Children [Text(format!("Tab {index}"))] });
            let content: Box<dyn Scene> =
                Box::new(bsn! { @WidgetryTextField Node { width: px(180) } });
            Box::new(bsn! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } })
                as Box<dyn Scene>
        })
        .collect();
    let items: Box<dyn SceneList> = Box::new(items);
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryTabs { @children: {Some(items)} } Node { width: px(400), height: px(200) } }).unwrap().id();
    app.update();
    let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
    let field = app.world().get::<Children>(page).unwrap()[0];
    WidgetryTextFieldHeadless::focus_in_world(app.world_mut(), field).unwrap();
    let key = |text: &str| KeyboardInput {
        key_code: KeyCode::KeyX,
        logical_key: Key::Character(text.into()),
        text: Some(text.into()),
        state: ButtonState::Pressed,
        repeat: false,
        window,
    };
    app.world_mut().write_message(key("x"));
    app.update();
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "x"
    );
    WidgetryTabs::set_selected_in_world(app.world_mut(), root, 1).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut().write_message(key("hidden"));
    app.update();
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "x"
    );
    let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
    app.world_mut().trigger(primary_click(header));
    app.world_mut().flush();
    assert_eq!(WidgetryTabs::page(app.world(), root, 0).unwrap(), page);
    assert_eq!(
        WidgetryTextFieldHeadless::text(app.world(), field).unwrap(),
        "x"
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}
