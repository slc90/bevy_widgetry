#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_splitter::{WidgetrySplitter, WidgetrySplitterPlugin};
use bevy_widgetry_test_utils::{picking_app, spawn_ui_camera};

fn settle(app: &mut App) {
    for _ in 0..16 {
        app.update();
    }
}
fn width(world: &World, e: Entity) -> f32 {
    let n = world.get::<ComputedNode>(e).unwrap();
    n.size().x * n.inverse_scale_factor
}
#[test]
fn percentage_constraints_and_scale_factors_use_logical_content_space() {
    for scale in [1.0, 2.0] {
        let mut app = picking_app();
        app.add_plugins(WidgetrySplitterPlugin);
        spawn_ui_camera(&mut app, UVec2::splat(2400), scale);
        let root = spawn_scene(
            app.world_mut(),
            bsn! {
                @WidgetrySplitter {
                    @split_ratio: 0.1, @handle_thickness: 16.0,
                    @pane_a: bsn! { Node { min_width: percent(25), max_width: percent(75) } },
                    @pane_b: bsn! { Node { min_width: px(100) } },
                } Node { width: px(1016), height: px(200) }
            },
        )
        .unwrap();
        settle(&mut app);
        let a = app.world().get::<Children>(root).unwrap()[0];
        assert!((width(app.world(), a) - 254.0).abs() < 1.1);
        assert!((WidgetrySplitter::split_ratio(app.world(), root).unwrap() - 0.254).abs() < 0.001);
        assert!((width(app.world(), root) - 1016.0).abs() < 1.1);
    }
}

#[test]
fn external_scroll_area_owns_overflow_and_scroll_state() {
    use bevy_widgetry_scroll_area::{
        ScrollAxis, WidgetryScrollArea, WidgetryScrollAreaPlugin, WidgetryScrollAreaViewport,
    };
    let mut app = picking_app();
    app.add_plugins((WidgetrySplitterPlugin, WidgetryScrollAreaPlugin));
    spawn_ui_camera(&mut app, UVec2::splat(800), 1.0);
    let scroll = spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetryScrollArea {
                @axis: ScrollAxis::Horizontal,
                @children: bsn_list! {
                    Name::new("OverflowSplitter") @WidgetrySplitter {
                        @pane_a: bsn! { Node { min_width: px(300) } },
                        @pane_b: bsn! { Node { min_width: px(400) } },
                    } Node { width: px(100), height: px(100) }
                },
            } Node { width: px(200), height: px(150) }
        },
    )
    .unwrap();
    settle(&mut app);
    let root = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, n)| n.as_str() == "OverflowSplitter")
        .unwrap()
        .0;
    assert!((width(app.world(), root) - 708.0).abs() < 1.1);
    assert!((width(app.world(), scroll) - 200.0).abs() < 1.1);
    assert_eq!(
        app.world().get::<Node>(root).unwrap().overflow,
        Overflow::visible()
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(root).unwrap().0,
        Vec2::ZERO
    );
    assert!(
        app.world_mut()
            .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
            .iter(app.world())
            .any(|e| app.world().get::<ScrollPosition>(e).is_some())
    );
}

#[test]
fn dynamic_pixel_maxima_can_shrink_a_root_with_percentage_minima() {
    let mut app = picking_app();
    app.add_plugins(WidgetrySplitterPlugin);
    spawn_ui_camera(&mut app, UVec2::splat(1600), 1.25);
    let root = spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetrySplitter {
                @pane_a: bsn! {Node {min_width: percent(40)}},
                @pane_b: bsn! {Node {min_width: percent(40)}},
            } Node {width: px(1008), height: px(100)}
        },
    )
    .unwrap();
    settle(&mut app);
    let children = app.world().get::<Children>(root).unwrap();
    let [a, b] = [children[0], children[2]];
    app.world_mut().get_mut::<Node>(a).unwrap().max_width = px(300);
    app.world_mut().get_mut::<Node>(b).unwrap().max_width = px(300);
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |s| {
        s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let errors = bevy_widgetry_test_utils::ErrorCapture::default();
    errors.run(|| settle(&mut app));
    assert!(errors.take().is_empty());
    assert!((width(app.world(), root) - 608.0).abs() < 1.0);
    assert!((width(app.world(), a) - 300.0).abs() < 1.0);
    assert!((width(app.world(), b) - 300.0).abs() < 1.0);
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
}

#[test]
fn layout_pending_preserves_failure_until_constraints_actually_recover() {
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    let mut app = picking_app();
    app.add_plugins(WidgetrySplitterPlugin);
    spawn_ui_camera(&mut app, UVec2::splat(1600), 1.0);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |s| {
        s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let root = spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetrySplitter { @pane_a: bsn! {Node {min_width: px(400),max_width: px(300)}} }
            Node {width: px(1008),height:px(100)}
        },
    )
    .unwrap();
    let a = app.world().get::<Children>(root).unwrap()[0];
    let errors = ErrorCapture::default();
    let log = LogCapture::default();
    log.run(|| {
        errors.run(|| {
            settle(&mut app);
            app.world_mut().get_mut::<Node>(a).unwrap().min_width = px(500);
            settle(&mut app);
        })
    });
    let messages: Vec<_> = log
        .records()
        .iter()
        .map(|r| r.fields.get("message").cloned().unwrap_or_default())
        .collect();
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.contains("Splitter 布局失效"))
            .count(),
        1
    );
    assert!(!messages.iter().any(|m| m.contains("布局恢复正常")));
    app.world_mut().get_mut::<Node>(a).unwrap().min_width = px(0);
    log.run(|| errors.run(|| settle(&mut app)));
    assert_eq!(
        log.records()
            .iter()
            .filter(|r| r
                .fields
                .get("message")
                .is_some_and(|m| m.contains("布局恢复正常")))
            .count(),
        1
    );
}
