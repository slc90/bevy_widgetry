//! Coverage Map：本文件负责公开 Scene、真实 layout 收敛与 scroll/keyboard/wheel 协作。
//! headless.rs 保留数值/手填几何与最近 viewport 算法。
//! layout.rs 保留 solver scheduling。
//! style.rs 保留 thumb/theme。
//! 本文件也覆盖 Mouse/Custom 的真实 thumb drag 终止，旧高层 Cancel 用受控派发回归。
//! State：axis 与 policy 固定、内容/可用尺寸变化、scroll offset。
//! stimuli 为 Scene、layout、keyboard、wheel、IntoView。
//! Invariants：实际 layout offset 合法，原生 request 不回写、唯一 content、稳定无 redraw。
//! keyboard=false 只关闭键盘入口；祖先禁用保留程序化滚动与 thumb 几何更新。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
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
use bevy::ui::{InteractionDisabled, ScrollPosition};
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

#[test]
fn inherited_disabled_blocks_keyboard_scroll_and_preserves_programmatic_scroll() {
    let mut app = scene_app();
    app.init_resource::<bevy::ui::UiScale>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(WidgetryScrollAreaPlugin);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let parent = app
        .world_mut()
        .spawn((Node::default(), InteractionDisabled))
        .id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryScrollArea template(move |_| Ok(ChildOf(parent))) })
        .unwrap()
        .id();
    let viewport = parts(&app, root).0;
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::splat(100.0),
        content_size: Vec2::new(100.0, 500.0),
        ..default()
    });
    app.world_mut().flush();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    press_key(&mut app, window, KeyCode::ArrowDown);
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
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        30.0
    );
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

#[derive(Resource, Default)]
struct StaleThumbCancel(Option<Pointer<bevy::picking::events::Cancel>>);

#[test]
fn real_thumb_drag_cancel_and_invalid_source_clear_dragging() {
    use bevy::picking::{
        hover::HoverMap,
        pointer::{Location, PointerAction, PointerButton, PointerId, PointerLocation},
    };
    use bevy::ui_widgets::{ScrollbarDragState, ScrollbarThumb};
    use bevy_widgetry_test_utils::{picking_app, pointer_ids, queue_pointer, spawn_picking_camera};
    for id in pointer_ids() {
        for failure in [
            "cancel",
            "location",
            "pointer",
            "window",
            "release",
            "foreign",
            "ancestor",
            "thumb",
            "thumb-start",
            "viewport",
            "viewport-start",
        ] {
            let mut app = picking_app();
            app.add_plugins(WidgetryScrollAreaPlugin)
                .init_resource::<StaleThumbCancel>()
                .add_systems(
                    PreUpdate,
                    (|mut stale: ResMut<StaleThumbCancel>, mut commands: Commands| {
                        if let Some(event) = stale.0.take() {
                            commands.trigger(event);
                        }
                    })
                    .after(bevy::picking::PickingSystems::Hover)
                    .before(bevy::picking::PickingSystems::PostHover),
                );
            let window = app
                .world_mut()
                .spawn((
                    Window {
                        resolution: (400, 400).into(),
                        ..default()
                    },
                    PrimaryWindow,
                ))
                .id();
            let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
            app.world_mut().spawn_scene(bsn! {
                @WidgetryScrollArea { @axis: ScrollAxis::Vertical, @children: bsn_list![(Node {width:px(120),height:px(500),flex_shrink:0.0})] }
                Node {width:px(150),height:px(100)} template(move |_|Ok(UiTargetCamera(camera)))
            }).unwrap();
            if id != PointerId::Mouse {
                app.world_mut().spawn(id);
            }
            for _ in 0..4 {
                app.update();
            }
            let thumb = app
                .world_mut()
                .query_filtered::<Entity, With<ScrollbarThumb>>()
                .single(app.world())
                .unwrap();
            let mut location = Location {
                target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                    .normalize(None)
                    .unwrap(),
                position: app
                    .world()
                    .get::<UiGlobalTransform>(thumb)
                    .unwrap()
                    .translation,
            };
            if matches!(failure, "thumb-start" | "viewport-start") {
                let disabled = if failure == "thumb-start" {
                    thumb
                } else {
                    app.world_mut()
                        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
                        .single(app.world())
                        .unwrap()
                };
                app.world_mut()
                    .entity_mut(disabled)
                    .insert(InteractionDisabled);
                app.world_mut().flush();
            }
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Move {
                    delta: location.position,
                },
            );
            app.update();
            assert!(app.world().resource::<HoverMap>()[&id].contains_key(&thumb));
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Press(PointerButton::Primary),
            );
            app.update();
            location.position.y += 20.0;
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Move {
                    delta: Vec2::new(0.0, 20.0),
                },
            );
            app.update();
            if matches!(failure, "thumb-start" | "viewport-start") {
                let viewport = app
                    .world_mut()
                    .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
                    .single(app.world())
                    .unwrap();
                let bar = app.world().get::<ChildOf>(thumb).unwrap().parent();
                assert!(
                    !app.world()
                        .get::<ScrollbarDragState>(thumb)
                        .unwrap()
                        .dragging
                );
                assert_eq!(
                    app.world().get::<ScrollPosition>(viewport).unwrap().0,
                    Vec2::ZERO
                );
                assert!(app.world().get::<Scrollbar>(bar).is_none());
                queue_pointer(
                    &mut app,
                    id,
                    location.clone(),
                    PointerAction::Release(PointerButton::Primary),
                );
                app.update();
                if failure == "viewport-start" {
                    location.position = app
                        .world()
                        .get::<UiGlobalTransform>(bar)
                        .unwrap()
                        .translation;
                    for enabled in [false, true] {
                        if enabled {
                            app.world_mut()
                                .entity_mut(viewport)
                                .remove::<InteractionDisabled>();
                            app.world_mut().flush();
                        }
                        for action in [
                            PointerAction::Move { delta: Vec2::ZERO },
                            PointerAction::Press(PointerButton::Primary),
                            PointerAction::Release(PointerButton::Primary),
                        ] {
                            queue_pointer(&mut app, id, location.clone(), action);
                            app.update();
                        }
                        let offset = app.world().get::<ScrollPosition>(viewport).unwrap().0.y;
                        if enabled {
                            assert!(offset > 0.0, "{id:?} track 恢复后可滚动");
                        } else {
                            assert_eq!(offset, 0.0, "{id:?} 禁用 viewport 阻止 track press");
                        }
                    }
                } else {
                    app.world_mut()
                        .entity_mut(thumb)
                        .remove::<InteractionDisabled>();
                    app.world_mut().flush();
                }
                assert!(app.world().get::<Scrollbar>(bar).is_some());
                continue;
            }
            assert!(
                app.world()
                    .get::<ScrollbarDragState>(thumb)
                    .unwrap()
                    .dragging
            );
            let pointer = app
                .world_mut()
                .query::<(Entity, &PointerId)>()
                .iter(app.world())
                .find(|(_, pointer)| **pointer == id)
                .unwrap()
                .0;
            if failure == "cancel" {
                location.position = Vec2::splat(350.0);
                queue_pointer(
                    &mut app,
                    id,
                    location.clone(),
                    PointerAction::Move {
                        delta: Vec2::splat(200.0),
                    },
                );
                app.update();
            }
            match failure {
                "thumb" | "ancestor" | "viewport" => {
                    let bar = app.world().get::<ChildOf>(thumb).unwrap().parent();
                    let root = app.world().get::<ChildOf>(bar).unwrap().parent();
                    let ancestor = app.world_mut().spawn(Node::default()).id();
                    app.world_mut().entity_mut(root).insert(ChildOf(ancestor));
                    let disabled = match failure {
                        "thumb" => thumb,
                        "viewport" => app
                            .world_mut()
                            .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
                            .single(app.world())
                            .unwrap(),
                        _ => ancestor,
                    };
                    app.world_mut()
                        .entity_mut(disabled)
                        .insert(InteractionDisabled);
                    app.world_mut().flush();
                    assert!(
                        !app.world()
                            .get::<ScrollbarDragState>(thumb)
                            .unwrap()
                            .dragging
                    );
                    let viewport = app
                        .world_mut()
                        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
                        .single(app.world())
                        .unwrap();
                    let before = app.world().get::<ScrollPosition>(viewport).unwrap().0;
                    location.position.y += 20.0;
                    queue_pointer(
                        &mut app,
                        id,
                        location.clone(),
                        PointerAction::Move {
                            delta: Vec2::new(0.0, 20.0),
                        },
                    );
                    app.update();
                    assert_eq!(
                        app.world().get::<ScrollPosition>(viewport).unwrap().0,
                        before
                    );
                }
                "cancel" => queue_pointer(&mut app, id, location, PointerAction::Cancel),
                "location" => {
                    app.world_mut()
                        .get_mut::<PointerLocation>(pointer)
                        .unwrap()
                        .location = None
                }
                "pointer" => {
                    app.world_mut().despawn(pointer);
                }
                "window" => {
                    app.world_mut().despawn(window);
                }
                "foreign" => {
                    let mut stale = bevy_widgetry_test_utils::primary_cancel(thumb);
                    stale.pointer_id = PointerId::Touch(7);
                    stale.pointer_location = location.clone();
                    app.world_mut().resource_mut::<StaleThumbCancel>().0 = Some(stale);
                    app.update();
                    assert!(
                        app.world()
                            .get::<ScrollbarDragState>(thumb)
                            .unwrap()
                            .dragging
                    );
                    queue_pointer(
                        &mut app,
                        id,
                        location.clone(),
                        PointerAction::Release(PointerButton::Primary),
                    );
                }
                _ => queue_pointer(
                    &mut app,
                    id,
                    location,
                    PointerAction::Release(PointerButton::Primary),
                ),
            }
            app.update();
            app.update();
            assert!(
                !app.world()
                    .get::<ScrollbarDragState>(thumb)
                    .unwrap()
                    .dragging,
                "{id:?}/{failure}"
            );
        }
    }
}

#[test]
fn disabled_thumb_keeps_layout_and_programmatic_scroll_current() {
    use bevy::ui::UiGlobalTransform;
    use bevy::ui_widgets::ScrollbarThumb;

    for local_viewport in [false, true] {
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
        app.add_plugins(WidgetryScrollAreaPlugin);
        let ancestor = app.world_mut().spawn(Node::default()).id();
        let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryScrollArea { @axis: ScrollAxis::Vertical, @children: bsn_list![(Node { width: px(80), height: px(500), flex_shrink: 0.0 })] }
        Node { width: px(100), height: px(100) } template(move |_| Ok(ChildOf(ancestor)))
    }).unwrap().id();
        let disabled = if local_viewport {
            parts(&app, root).0
        } else {
            ancestor
        };
        app.world_mut()
            .entity_mut(disabled)
            .insert(InteractionDisabled);
        app.world_mut().flush();
        for _ in 0..5 {
            app.update();
        }
        let viewport = parts(&app, root).0;
        let thumb = app
            .world_mut()
            .query_filtered::<Entity, With<ScrollbarThumb>>()
            .single(app.world())
            .unwrap();
        let bar = app.world().get::<ChildOf>(thumb).unwrap().parent();
        assert!(app.world().get::<Scrollbar>(bar).is_none());
        let initial = *app.world().get::<UiGlobalTransform>(thumb).unwrap();
        let initial_size = app.world().get::<ComputedNode>(thumb).unwrap().size();
        assert!(initial_size.y > 0.0 && initial_size.y < 100.0);
        app.world_mut()
            .get_mut::<ScrollPosition>(viewport)
            .unwrap()
            .0
            .y = 200.0;
        for _ in 0..3 {
            app.update();
        }
        let scrolled = *app.world().get::<UiGlobalTransform>(thumb).unwrap();
        assert!(scrolled.translation.y > initial.translation.y);
        assert_eq!(
            app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
            200.0
        );
        app.world_mut()
            .entity_mut(disabled)
            .remove::<InteractionDisabled>();
        app.world_mut().flush();
        for _ in 0..3 {
            app.update();
        }
        assert!(app.world().get::<Scrollbar>(bar).is_some());
        assert_eq!(
            *app.world().get::<UiGlobalTransform>(thumb).unwrap(),
            scrolled
        );
        assert_eq!(
            app.world().get::<ComputedNode>(thumb).unwrap().size(),
            initial_size
        );
    }
}
