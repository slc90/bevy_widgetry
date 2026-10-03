//! Coverage Map：本文件负责公开 Scene、真实 layout 收敛与 scroll/keyboard/wheel 协作。
//! headless.rs 保留数值/手填几何与最近 viewport 算法；layout.rs 保留 solver scheduling；style.rs 保留 thumb/theme。
//! State：axis 与 policy 固定、内容/可用尺寸变化、scroll offset；stimuli 为 Scene、layout、keyboard、wheel、IntoView。
//! Invariants：实际 layout offset 合法，原生 request 不回写、唯一 content、稳定无 redraw；keyboard=false 只关闭键盘入口。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::ecs::message::Messages;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input_focus::{
    FocusCause, FocusedInput, InputFocus, InputFocusSystems, dispatch_focused_input,
};
use bevy::picking::events::{Pointer, Scroll};
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar};
use bevy::window::{PrimaryWindow, RequestRedraw};
use bevy_widgetry_scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
    WidgetryScrollAreaContent, WidgetryScrollAreaPlugin, WidgetryScrollAreaProps,
    WidgetryScrollAreaViewport, WidgetryScrollIntoView,
};
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, add_ui_plugins, press_key, primary_click, scene_app, spawn_ui_camera,
};

#[derive(Resource, Default)]
struct AncestorKeyboardCount(usize);

#[test]
fn public_scene_exposes_viewport_and_respects_root_patch() {
    let mut app = scene_app();
    app.add_plugins(WidgetryScrollAreaPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryScrollArea {
            @axis: ScrollAxis::Horizontal,
            @scrollbar_visibility: {ScrollbarVisibility { horizontal: ScrollbarPolicy::Hidden, vertical: ScrollbarPolicy::Always }},
        }
        Node { width: px(180), height: px(90) }
    }).unwrap().id();
    let root_node = app.world().get::<Node>(root).unwrap();
    assert_eq!(root_node.width, px(180));
    assert_eq!(root_node.height, px(90));
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    assert!(app.world().get::<ScrollPosition>(viewport).is_some());
    let content = app.world().get::<Children>(viewport).unwrap()[0];
    assert!(
        app.world()
            .get::<WidgetryScrollAreaContent>(content)
            .is_some()
    );
    app.update();
    assert_eq!(app.world().get::<Node>(root).unwrap().width, px(180));
    let bars = app
        .world_mut()
        .query::<(Entity, &Scrollbar)>()
        .iter(app.world())
        .collect::<Vec<_>>();
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].1.target, viewport);
    assert_eq!(bars[0].1.orientation, ControlOrientation::Horizontal);
    assert_eq!(
        app.world().get::<Node>(bars[0].0).unwrap().display,
        Display::None
    );
}

#[test]
fn keyboard_scroll_switch_preserves_default_and_event_ownership() {
    assert!(WidgetryScrollAreaProps::default().keyboard_scroll);
    for enabled in [true, false] {
        let mut app = scene_app();
        app.init_resource::<bevy::ui::UiScale>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<AncestorKeyboardCount>()
            .add_message::<KeyboardInput>()
            .add_systems(
                PreUpdate,
                dispatch_focused_input::<KeyboardInput>.in_set(InputFocusSystems::Dispatch),
            )
            .add_plugins(WidgetryScrollAreaPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryScrollArea { @keyboard_scroll: enabled }
            })
            .unwrap()
            .id();
        let viewport = app.world().get::<Children>(root).unwrap()[0];
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::splat(100.0),
            content_size: Vec2::new(100.0, 400.0),
            ..default()
        });
        let parent = app.world_mut().spawn(Node::default()).id();
        app.world_mut().entity_mut(parent).add_child(root).observe(
            |_: On<FocusedInput<KeyboardInput>>, mut count: ResMut<AncestorKeyboardCount>| {
                count.0 += 1;
            },
        );
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(root, FocusCause::Navigated);
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::ArrowDown,
            logical_key: Key::ArrowDown,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        assert_eq!(
            app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
            if enabled { 100.0 } else { 0.0 }
        );
        assert_eq!(
            app.world().resource::<AncestorKeyboardCount>().0,
            usize::from(!enabled)
        );
    }
}

fn parts(app: &App, root: Entity) -> (Entity, Entity, Vec<Entity>) {
    let children = app.world().get::<Children>(root).unwrap();
    let viewport = children
        .iter()
        .find(|child| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(*child)
                .is_some()
        })
        .unwrap();
    let content = app.world().get::<Children>(viewport).unwrap()[0];
    let bars = children
        .iter()
        .filter(|child| app.world().get::<Scrollbar>(*child).is_some())
        .collect();
    (viewport, content, bars)
}

fn settle(app: &mut App) {
    for _ in 0..5 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<Messages<RequestRedraw>>()
        .clear();
    app.update();
    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
}

fn assert_layout(
    app: &App,
    root: Entity,
    viewport: Entity,
    content: Entity,
    bars: &[Entity],
    expected: Vec2,
    shown: bool,
) {
    let node = app.world().get::<ComputedNode>(viewport).unwrap();
    assert_eq!(node.size() * node.inverse_scale_factor, expected);
    let offset = node.scroll_position * node.inverse_scale_factor;
    let range = (node.content_size() - node.size()).max(Vec2::ZERO) * node.inverse_scale_factor;
    assert!(
        offset.cmpge(Vec2::ZERO).all() && offset.cmple(range).all(),
        "{offset:?} / {range:?}"
    );
    assert_eq!(
        app.world().get::<Children>(viewport).unwrap().to_vec(),
        vec![content]
    );
    assert_eq!(app.world().get::<ChildOf>(viewport).unwrap().parent(), root);
    for bar in bars {
        assert_eq!(app.world().get::<Scrollbar>(*bar).unwrap().target, viewport);
        assert_eq!(
            app.world().get::<Node>(*bar).unwrap().display,
            if shown { Display::Flex } else { Display::None }
        );
    }
}

#[test]
fn public_auto_layout_converges_and_recovers_after_content_changes() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
    app.add_plugins(WidgetryScrollAreaPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryScrollArea { @axis: ScrollAxis::Both, @children: bsn_list![(Node { width: px(98), height: px(150), flex_shrink: 0.0 })] }
        Node { width: px(100), height: px(100), border: UiRect::all(px(1)) }
    }).unwrap().id();
    let (viewport, content, bars) = parts(&app, root);
    let child = app.world().get::<Children>(content).unwrap()[0];
    app.update();
    assert_eq!(
        app.world().get::<ComputedNode>(viewport).unwrap().size(),
        Vec2::splat(98.0)
    );
    for bar in &bars {
        let vertical =
            app.world().get::<Scrollbar>(*bar).unwrap().orientation == ControlOrientation::Vertical;
        assert_eq!(
            app.world().get::<Node>(*bar).unwrap().display,
            if vertical {
                Display::Flex
            } else {
                Display::None
            }
        );
    }
    app.update();
    assert_eq!(
        app.world().get::<ComputedNode>(viewport).unwrap().size(),
        Vec2::new(86.0, 98.0)
    );
    settle(&mut app);
    assert_layout(
        &app,
        root,
        viewport,
        content,
        &bars,
        Vec2::splat(86.0),
        true,
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0 = Vec2::new(12.0, 60.0);
    for extent in [Vec2::splat(10.0), Vec2::splat(98.0)] {
        {
            let mut node = app.world_mut().get_mut::<Node>(child).unwrap();
            node.width = px(extent.x);
            node.height = px(extent.y);
        }
        settle(&mut app);
        assert_layout(
            &app,
            root,
            viewport,
            content,
            &bars,
            Vec2::splat(98.0),
            false,
        );
        assert_eq!(
            app.world().get::<ScrollPosition>(viewport).unwrap().0,
            Vec2::new(12.0, 60.0)
        );
        assert_eq!(
            app.world()
                .get::<ComputedNode>(viewport)
                .unwrap()
                .scroll_position,
            Vec2::ZERO
        );
    }
    app.world_mut().despawn(child);
    settle(&mut app);
    assert_layout(
        &app,
        root,
        viewport,
        content,
        &bars,
        Vec2::splat(98.0),
        false,
    );
    let large = app
        .world_mut()
        .spawn_scene(bsn! { Node { width: px(200), height: px(200), flex_shrink: 0.0 } })
        .unwrap()
        .id();
    app.world_mut().entity_mut(content).add_child(large);
    settle(&mut app);
    assert_layout(
        &app,
        root,
        viewport,
        content,
        &bars,
        Vec2::splat(86.0),
        true,
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0 = Vec2::splat(110.0);
    {
        let mut node = app.world_mut().get_mut::<Node>(root).unwrap();
        node.width = px(150);
        node.height = px(150);
    }
    settle(&mut app);
    assert_layout(
        &app,
        root,
        viewport,
        content,
        &bars,
        Vec2::splat(136.0),
        true,
    );
}

#[test]
fn real_scene_keyboard_switch_keeps_wheel_and_into_view() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(600), 2.0);
    add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryScrollAreaPlugin);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryScrollArea { @keyboard_scroll: false, @children: bsn_list![
                (Node { height: px(80), flex_shrink: 0.0 }),
                (Node { height: px(40), flex_shrink: 0.0 }),
                (Node { height: px(180), flex_shrink: 0.0 }),
            ] }
            Node { width: px(100), height: px(100) }
        })
        .unwrap()
        .id();
    let (viewport, content, _) = parts(&app, root);
    settle(&mut app);
    let targets = app.world().get::<Children>(content).unwrap().to_vec();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    press_key(&mut app, window, KeyCode::ArrowDown);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0,
        Vec2::ZERO
    );
    let click = primary_click(targets[0]);
    app.world_mut().trigger(Pointer::new(
        click.pointer_id,
        click.pointer_location.clone(),
        Scroll {
            x: 0.0,
            y: -20.0,
            unit: MouseScrollUnit::Pixel,
            hit: click.hit.clone(),
            phase: bevy::input::touch::TouchPhase::Moved,
        },
        targets[0],
    ));
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        20.0
    );
    app.update();
    app.world_mut()
        .trigger(WidgetryScrollIntoView { entity: targets[1] });
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        20.0
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 0.0;
    app.update();
    app.world_mut()
        .trigger(WidgetryScrollIntoView { entity: targets[1] });
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        80.0
    );
    let outside = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
    app.world_mut()
        .trigger(WidgetryScrollIntoView { entity: outside });
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        80.0
    );
}

#[test]
fn nested_public_scenes_route_into_view_to_nearest_viewport() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(800), 2.0);
    app.add_plugins(WidgetryScrollAreaPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryScrollArea { @children: bsn_list![
                (Node { height: px(80), flex_shrink: 0.0 }),
                (@WidgetryScrollArea { @children: bsn_list![
                    (Node { height: px(80), flex_shrink: 0.0 }),
                    (Node { height: px(40), flex_shrink: 0.0 }),
                    (Node { height: px(180), flex_shrink: 0.0 }),
                ] } Node { width: px(80), height: px(100), flex_shrink: 0.0 }),
                (Node { height: px(180), flex_shrink: 0.0 }),
            ] }
            Node { width: px(100), height: px(100) }
        })
        .unwrap()
        .id();
    let (outer, outer_content, _) = parts(&app, root);
    let inner_root = app.world().get::<Children>(outer_content).unwrap()[1];
    let (inner, content, _) = parts(&app, inner_root);
    settle(&mut app);
    let target = app.world().get::<Children>(content).unwrap()[1];
    app.world_mut()
        .trigger(WidgetryScrollIntoView { entity: target });
    app.world_mut().flush();
    assert_eq!(app.world().get::<ScrollPosition>(inner).unwrap().0.y, 80.0);
    assert_eq!(app.world().get::<ScrollPosition>(outer).unwrap().0.y, 0.0);
}
