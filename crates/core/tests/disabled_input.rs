//! State 为祖先 enabled/disabled、Button activation、Checkbox/Radio 用户通知和 focus。
//! Pointer press/click 与 Enter/Space 在本地请求 flush 后驱动官方输入消费者。
//! Disabled 投影阻止真实消费者产出，保留 focus，恢复后后续输入正常产生通知。

// 测试通过断言验证 contract，仅在本文件允许测试所需的 panic lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input::mouse::MouseScrollUnit;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::events::{Pointer, Scroll};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition, UiScale};
use bevy::ui_widgets::{
    Activate, Button, ButtonPlugin, Checkbox, CheckboxPlugin, RadioButton, RadioGroup,
    RadioGroupPlugin, ScrollArea, ScrollAreaPlugin, ValueChange,
};
use bevy::window::PrimaryWindow;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, pointer_ids, press_key, primary_click, primary_press, scene_app,
};

#[derive(Resource, Default)]
struct Outputs(usize);

#[test]
fn official_pointer_and_keyboard_consumers_observe_inherited_disabled_after_flush() {
    let mut app = scene_app();
    app.add_plugins((ButtonPlugin, CheckboxPlugin, RadioGroupPlugin))
        .init_resource::<Outputs>();
    app.add_observer(|_: On<Activate>, mut outputs: ResMut<Outputs>| outputs.0 += 1)
        .add_observer(|_: On<ValueChange<bool>>, mut outputs: ResMut<Outputs>| outputs.0 += 1)
        .add_observer(|_: On<ValueChange<Entity>>, mut outputs: ResMut<Outputs>| outputs.0 += 1);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let parent = app.world_mut().spawn(Node::default()).id();
    let button = app
        .world_mut()
        .spawn((Node::default(), Button, ChildOf(parent)))
        .id();
    let checkbox = app
        .world_mut()
        .spawn((Node::default(), Checkbox, ChildOf(parent)))
        .id();
    let group = app
        .world_mut()
        .spawn((Node::default(), RadioGroup, ChildOf(parent)))
        .id();
    let radio = app
        .world_mut()
        .spawn((Node::default(), RadioButton, ChildOf(group)))
        .id();
    app.world_mut().flush();
    app.world_mut()
        .entity_mut(parent)
        .insert(InteractionDisabled);
    app.world_mut().flush();
    for entity in [button, checkbox, radio] {
        for pointer in pointer_ids() {
            let mut press = primary_press(entity);
            press.pointer_id = pointer;
            let mut click = primary_click(entity);
            click.pointer_id = pointer;
            app.world_mut().trigger(press);
            app.world_mut().trigger(click);
            app.world_mut().flush();
        }
        for key in [KeyCode::Enter, KeyCode::Space] {
            app.world_mut()
                .resource_mut::<InputFocus>()
                .set(entity, FocusCause::Navigated);
            press_key(&mut app, window, key);
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(entity));
        }
    }
    assert_eq!(app.world().resource::<Outputs>().0, 0);
    app.world_mut()
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    for entity in [button, checkbox, radio] {
        app.world_mut().trigger(primary_press(entity));
        app.world_mut().trigger(primary_click(entity));
        app.world_mut().flush();
    }
    assert_eq!(app.world().resource::<Outputs>().0, 3);
}

#[test]
fn official_wheel_consumer_is_blocked_without_stopping_programmatic_scroll() {
    let mut app = scene_app();
    app.init_resource::<UiScale>().add_plugins(ScrollAreaPlugin);
    let root = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    let viewport = app
        .world_mut()
        .spawn((
            Node {
                overflow: Overflow::scroll(),
                ..default()
            },
            ScrollArea,
            ScrollPosition::default(),
            ChildOf(root),
        ))
        .id();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::splat(100.0),
        content_size: Vec2::new(100.0, 500.0),
        ..default()
    });
    app.world_mut().flush();
    let click = primary_click(viewport);
    let scroll = Pointer::new(
        click.pointer_id,
        click.pointer_location.clone(),
        Scroll {
            x: 0.0,
            y: -20.0,
            unit: MouseScrollUnit::Pixel,
            hit: click.hit.clone(),
            phase: bevy::input::touch::TouchPhase::Moved,
        },
        viewport,
    );
    app.world_mut().trigger(scroll.clone());
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 30.0;
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    app.world_mut().trigger(scroll);
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        50.0
    );
}

#[test]
fn deferred_requests_are_applied_before_keyboard_dispatch() {
    let mut app = scene_app();
    app.add_plugins(ButtonPlugin).init_resource::<Outputs>();
    app.add_observer(|_: On<Activate>, mut outputs: ResMut<Outputs>| outputs.0 += 1);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let parent = app.world_mut().spawn(Node::default()).id();
    let button = app
        .world_mut()
        .spawn((Node::default(), Button, ChildOf(parent)))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(button, FocusCause::Navigated);
    app.add_systems(
        PreUpdate,
        (move |mut commands: Commands| {
            commands.entity(parent).insert(InteractionDisabled);
        })
        .before(WidgetryUiSystems::Disabled),
    );
    press_key(&mut app, window, KeyCode::Enter);
    assert_eq!(app.world().resource::<Outputs>().0, 0);
    assert!(app.world().get::<InteractionDisabled>(button).is_some());
}
