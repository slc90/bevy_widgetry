use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{
    FocusCause, FocusedInput, InputFocus, InputFocusSystems, dispatch_focused_input,
};
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar};
use bevy::window::PrimaryWindow;
use bevy_widgetry_scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
    WidgetryScrollAreaContent, WidgetryScrollAreaPlugin, WidgetryScrollAreaProps,
    WidgetryScrollAreaViewport,
};
use bevy_widgetry_test_utils::scene_app;

/// 记录未被 ScrollArea 消费且抵达 ancestor 的 keyboard event 数量。
#[derive(Resource, Default)]
struct AncestorKeyboardCount(usize);

/// 外部调用方通过公开 Props 构造 Horizontal ScrollArea，并查询 Viewport 的原生滚动 state。
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

/// 默认 keyboard scroll 保持现状；关闭后不改变 ScrollPosition 且不截断 ancestor 的 keyboard 输入。
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
