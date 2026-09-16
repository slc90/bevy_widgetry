use bevy::{
    input_focus::{FocusLost, InputFocus},
    prelude::*,
    ui::Pressed,
};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonPlugin},
    style::WidgetryFocusPlugin,
    text_field::{WidgetryTextField, WidgetryTextFieldPlugin},
};
use bevy_widgetry_test_utils::{press, text_input_app};

/// 记录实际派发的失焦目标，避免仅凭资源值判断集成成功。
#[derive(Resource, Default)]
struct LostFocus(Vec<Entity>);

// 真实 Button 会停止 pointer 冒泡，两种插件注册顺序下均须使已聚焦 TextField 失焦。
#[test]
fn button_press_clears_text_focus_even_when_propagation_stops() {
    for focus_first in [false, true] {
        let mut app = text_input_app();
        if focus_first {
            app.add_plugins((WidgetryFocusPlugin, WidgetryButtonPlugin));
        } else {
            app.add_plugins((WidgetryButtonPlugin, WidgetryFocusPlugin));
        }
        app.add_plugins(WidgetryTextFieldPlugin)
            .init_resource::<LostFocus>();
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
        assert_eq!(app.world().resource::<InputFocus>().get(), None);
        assert_eq!(app.world().resource::<LostFocus>().0, [text]);
    }
}
