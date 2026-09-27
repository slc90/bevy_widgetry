use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar};
use bevy_widgetry_scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea, WidgetryScrollAreaPlugin,
    WidgetryScrollAreaViewport,
};
use bevy_widgetry_test_utils::scene_app;

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
