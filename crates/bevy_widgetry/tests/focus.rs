use bevy::{
    input_focus::{
        FocusLost, InputFocus,
        tab_navigation::{TabIndex, TabNavigationPlugin},
    },
    prelude::*,
    ui::Pressed,
    window::PrimaryWindow,
};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonPlugin},
    text_field::{WidgetryTextField, WidgetryTextFieldPlugin},
};
use bevy_widgetry_test_utils::{press, text_input_app};

/// 记录实际 dispatch 的 focus lost 目标，避免仅凭 resource 值判断集成成功。
#[derive(Resource, Default)]
struct LostFocus(Vec<Entity>);

// TextField 与 Button 的真实组合通过官方 pointer focus 将 focus 转到 TabIndex(-1) Button。
#[test]
fn button_press_focuses_button_even_when_propagation_stops() {
    for button_first in [false, true] {
        let mut app = text_input_app();
        if button_first {
            app.add_plugins(WidgetryButtonPlugin);
            app.add_plugins(WidgetryTextFieldPlugin);
        } else {
            app.add_plugins(WidgetryTextFieldPlugin);
            app.add_plugins(WidgetryButtonPlugin);
        }
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.init_resource::<LostFocus>();
        assert!(app.is_plugin_added::<TabNavigationPlugin>());
        app.add_observer(|event: On<FocusLost>, mut lost: ResMut<LostFocus>| {
            lost.0.push(event.entity)
        });
        let text = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTextField })
            .unwrap()
            .id();
        let button = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();
        press(&mut app, text);
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(text));
        press(&mut app, button);
        app.update();
        assert!(app.world().get::<Pressed>(button).is_some());
        assert_eq!(app.world().get::<TabIndex>(button), Some(&TabIndex(-1)));
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
        assert_eq!(app.world().resource::<LostFocus>().0, [text]);
    }
}

// 从 TextField 点击无 TabIndex ancestor 的普通 Node 时，官方 AcquireFocus 清除 focus。
#[test]
fn plain_node_press_clears_text_focus() {
    let mut app = text_input_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    let text = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    let node = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
    press(&mut app, text);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(text));
    press(&mut app, node);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}
