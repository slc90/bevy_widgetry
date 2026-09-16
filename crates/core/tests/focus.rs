use bevy::input_focus::{FocusGained, FocusLost};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    picking::pointer::PointerButton,
    prelude::*,
    text::EditableText,
};
use bevy_widgetry_core::WidgetryFocusPlugin;
use bevy_widgetry_test_utils::{primary_press, scene_app, text_input_app};

/// 记录真实焦点事件，区分保持焦点与先失焦再重新获得焦点。
#[derive(Resource, Default)]
struct FocusEvents {
    /// 本帧获得焦点的目标。
    gained: Vec<Entity>,
    /// 本帧失去焦点的目标。
    lost: Vec<Entity>,
}

// 非文本目标的主键按下清除已有焦点，次键和中键则保留。
#[test]
fn only_primary_press_on_non_editable_clears_focus() {
    for button in [
        PointerButton::Primary,
        PointerButton::Secondary,
        PointerButton::Middle,
    ] {
        let mut app = scene_app();
        app.add_plugins(WidgetryFocusPlugin);
        let text = app
            .world_mut()
            .spawn_scene(bsn! { EditableText })
            .unwrap()
            .id();
        let node = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(text, FocusCause::Navigated);
        let mut event = primary_press(node);
        event.event.button = button;
        app.world_mut().trigger(event);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            if button == PointerButton::Primary {
                None
            } else {
                Some(text)
            }
        );
    }
}

// Widgetry 不抢先清除文本目标的焦点，也不自行承担官方输入插件的切换职责。
#[test]
fn editable_targets_leave_focus_to_official_input() {
    let mut app = scene_app();
    app.add_plugins(WidgetryFocusPlugin);
    let first = app
        .world_mut()
        .spawn_scene(bsn! { EditableText })
        .unwrap()
        .id();
    let second = app
        .world_mut()
        .spawn_scene(bsn! { EditableText })
        .unwrap()
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(first, FocusCause::Navigated);
    for target in [first, second] {
        app.world_mut().trigger(primary_press(target));
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(first));
    }
}

// 官方输入插件与全局策略协作时，文本间切换仅产生一次 Lost/Gained，点击当前文本不重复通知。
#[test]
fn official_input_switches_focus_without_intermediate_clear() {
    let mut app = text_input_app();
    app.add_plugins(WidgetryFocusPlugin)
        .init_resource::<FocusEvents>();
    app.add_observer(|event: On<FocusGained>, mut events: ResMut<FocusEvents>| {
        if event.entity == event.original_event_target() {
            events.gained.push(event.entity);
        }
    });
    app.add_observer(|event: On<FocusLost>, mut events: ResMut<FocusEvents>| {
        if event.entity == event.original_event_target() {
            events.lost.push(event.entity);
        }
    });
    let first = app
        .world_mut()
        .spawn_scene(bsn! { EditableText })
        .unwrap()
        .id();
    let second = app
        .world_mut()
        .spawn_scene(bsn! { EditableText })
        .unwrap()
        .id();
    let node = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
    app.world_mut().entity_mut(node).add_child(second);
    app.world_mut().trigger(primary_press(first));
    app.update();
    app.world_mut().insert_resource(FocusEvents::default());
    app.world_mut().trigger(primary_press(second));
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(second));
    assert_eq!(app.world().resource::<FocusEvents>().lost, [first]);
    assert_eq!(app.world().resource::<FocusEvents>().gained, [second]);
    app.world_mut().insert_resource(FocusEvents::default());
    app.world_mut().trigger(primary_press(second));
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(second));
    assert!(app.world().resource::<FocusEvents>().lost.is_empty());
    assert!(app.world().resource::<FocusEvents>().gained.is_empty());
    app.world_mut().trigger(primary_press(node));
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert_eq!(app.world().resource::<FocusEvents>().lost, [second]);
}
