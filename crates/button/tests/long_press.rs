#![cfg(test)]

use bevy::{
    app::App,
    ecs::{observer::On, resource::Resource, system::ResMut},
    time::{TimePlugin, TimeUpdateStrategy},
    ui::InteractionDisabled,
};
use bevy_widgetry_button::{LongPressButton, LongPressEvent, LongPressPlugin};
use bevy_widgetry_test_utils::{cancel, drag_end, press, release};
use std::time::Duration;

#[derive(Resource, Default)]
struct LongPressCount(usize);

fn record_long_press(_: On<LongPressEvent>, mut count: ResMut<LongPressCount>) {
    count.0 += 1;
}

fn setup_app(frame_time: Duration) -> App {
    let mut app = App::new();

    app.add_plugins(TimePlugin);
    app.add_plugins(LongPressPlugin);

    app.insert_resource(TimeUpdateStrategy::ManualDuration(frame_time));
    app.init_resource::<LongPressCount>();
    app.add_observer(record_long_press);
    app
}

// 固定步长推进时间，验证阈值前无事件且到期只触发一次。
#[test]
fn long_press_does_not_fire_before_threshold() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time，不算作长按时间
    app.update();

    let button = app.world_mut().spawn(LongPressButton::default()).id();

    press(&mut app, button);

    for _ in 0..4 {
        app.update();
        assert_eq!(app.world().resource::<LongPressCount>().0, 0);
    }

    app.update();

    assert_eq!(app.world().resource::<LongPressCount>().0, 1);

    for _ in 0..5 {
        app.update();
        assert_eq!(app.world().resource::<LongPressCount>().0, 1);
    }
}

// 计时途中释放后继续推进，验证旧计时不会再产生事件。
#[test]
fn release_before_threshold_cancels_long_press() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app.world_mut().spawn(LongPressButton::default()).id();

    press(&mut app, button);

    for _ in 0..2 {
        app.update();
    }

    release(&mut app, button);

    for _ in 0..5 {
        app.update();
    }

    assert_eq!(app.world().resource::<LongPressCount>().0, 0);
}

// 计时途中取消指针后继续推进，验证未完成交互不触发长按。
#[test]
fn cancel_before_threshold_cancels_long_press() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app.world_mut().spawn(LongPressButton::default()).id();

    press(&mut app, button);

    for _ in 0..2 {
        app.update();
    }

    cancel(&mut app, button);

    for _ in 0..5 {
        app.update();
    }

    assert_eq!(app.world().resource::<LongPressCount>().0, 0);
}

// 拖动结束打断计时后继续推进，验证旧按压不再生效。
#[test]
fn drag_end_before_threshold_cancels_long_press() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app.world_mut().spawn(LongPressButton::default()).id();

    press(&mut app, button);

    for _ in 0..2 {
        app.update();
    }

    drag_end(&mut app, button);

    for _ in 0..5 {
        app.update();
    }

    assert_eq!(app.world().resource::<LongPressCount>().0, 0);
}

// 对禁用按钮发送按压并跨过阈值，验证没有长按通知。
#[test]
fn disabled_button_does_not_fire_long_press() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app
        .world_mut()
        .spawn((LongPressButton::default(), InteractionDisabled))
        .id();

    press(&mut app, button);

    for _ in 0..10 {
        app.update();
    }

    assert_eq!(app.world().resource::<LongPressCount>().0, 0);
}

// 配置非默认阈值并分段推进时间，验证计时采用组件提供的持续时间。
#[test]
fn custom_press_duration_changes_trigger_time() {
    let mut app = setup_app(Duration::from_millis(100));
    // 初始化 Time
    app.update();

    let button = app
        .world_mut()
        .spawn(LongPressButton {
            press_duration: 300,
        })
        .id();

    press(&mut app, button);

    for _ in 0..2 {
        app.update();
        assert_eq!(app.world().resource::<LongPressCount>().0, 0);
    }

    app.update();

    assert_eq!(app.world().resource::<LongPressCount>().0, 1);
}
