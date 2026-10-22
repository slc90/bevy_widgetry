#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy_widgetry_tabs::{WidgetryTab, WidgetryTabs, WidgetryTabsPlugin};
use bevy_widgetry_test_utils::{add_keyboard_dispatch, primary_click, text_edit_app};

#[test]
fn tab_and_shift_tab_cannot_enter_hidden_pages() {
    use bevy::input_focus::{
        FocusCause,
        tab_navigation::{TabGroup, TabIndex},
    };
    use bevy_widgetry_test_utils::press_key;
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
            let content: Box<dyn Scene> = Box::new(bsn! { @WidgetryTextField TabIndex(0) });
            Box::new(bsn! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } })
                as Box<dyn Scene>
        })
        .collect();
    let items: Box<dyn SceneList> = Box::new(items);
    let container = app
        .world_mut()
        .spawn_scene(bsn! {
            Node TabGroup::default() Children [
                Node TabIndex(0)--
                @WidgetryTabs { @children: {Some(items)} }--
                Node TabIndex(0)
            ]
        })
        .unwrap()
        .id();
    app.update();
    let children = app.world().get::<Children>(container).unwrap().to_vec();
    let root = children[1];
    for selected in [0, 1] {
        WidgetryTabs::set_selected_in_world(app.world_mut(), root, selected).unwrap();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(children[0], FocusCause::Navigated);
        press_key(&mut app, window, KeyCode::Tab);
        let page = WidgetryTabs::page(app.world(), root, selected).unwrap();
        let field = app.world().get::<Children>(page).unwrap()[0];
        let focus = app.world().resource::<InputFocus>().get().unwrap();
        assert!(
            focus == field
                || app.world().get::<ChildOf>(focus).is_some_and(|parent| app
                    .world()
                    .get::<ChildOf>(parent.parent())
                    .is_some_and(|parent| parent.parent() == field))
        );
        press_key(&mut app, window, KeyCode::Tab);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(children[2])
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        press_key(&mut app, window, KeyCode::Tab);
        press_key(&mut app, window, KeyCode::Tab);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(children[0])
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftLeft);
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
    WidgetryTextField::focus_in_world(app.world_mut(), field).unwrap();
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
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "x");
    WidgetryTabs::set_selected_in_world(app.world_mut(), root, 1).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut().write_message(key("hidden"));
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "x");
    let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
    app.world_mut().trigger(primary_click(header));
    app.world_mut().flush();
    assert_eq!(WidgetryTabs::page(app.world(), root, 0).unwrap(), page);
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "x");
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}
