#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_splitter::{
    WidgetrySplitter, WidgetrySplitterOrientation as Orientation, WidgetrySplitterPlugin,
};
use bevy_widgetry_test_utils::{picking_app, spawn_ui_camera};
fn settle(app: &mut App) {
    for _ in 0..20 {
        app.update();
    }
}
fn extent(world: &World, entity: Entity, orientation: Orientation) -> f32 {
    let c = world.get::<ComputedNode>(entity).unwrap();
    let v = c.unrounded_size * c.inverse_scale_factor;
    if orientation == Orientation::Horizontal {
        v.x
    } else {
        v.y
    }
}

#[test]
fn fractional_minimum_initialization_uses_unrounded_geometry() {
    for scale in [1.0, 1.25, 1.5] {
        let mut app = picking_app();
        app.add_plugins(WidgetrySplitterPlugin);
        spawn_ui_camera(&mut app, UVec2::splat(1200), scale);
        let root = spawn_scene(
            app.world_mut(),
            bsn! {
                @WidgetrySplitter { @split_ratio: 0.1,
                    @pane_a: bsn! { Node { min_width: px(100.1) } },
                    @pane_b: bsn! { Node { min_width: px(100.1) } },
                } Node { width: px(100), height: px(100) }
            },
        )
        .unwrap();
        settle(&mut app);
        assert!((extent(app.world(), root, Orientation::Horizontal) - 208.2).abs() < 0.001);
        assert!((WidgetrySplitter::split_ratio(app.world(), root).unwrap() - 0.5).abs() < 0.0001);
    }
}

#[test]
fn content_box_panes_and_root_count_insets_once() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let mut app = picking_app();
        app.add_plugins(WidgetrySplitterPlugin);
        spawn_ui_camera(&mut app, UVec2::splat(1600), 1.25);
        let root = spawn_scene(app.world_mut(), bsn! {
            @WidgetrySplitter { @orientation: {orientation}, @split_ratio: 0.3,
                @pane_a: bsn! { Node { box_sizing: BoxSizing::ContentBox, padding: UiRect::all(px(10)), border: UiRect::all(px(2)) } },
                @pane_b: bsn! { Node { box_sizing: BoxSizing::ContentBox, padding: UiRect::all(px(10)), border: UiRect::all(px(2)) } },
            } Node { width: px(1008), height: px(1008), padding: UiRect::all(px(5)), box_sizing: BoxSizing::ContentBox }
        }).unwrap();
        settle(&mut app);
        let children = app.world().get::<Children>(root).unwrap();
        assert!(
            (extent(app.world(), children[0], orientation) - 300.0).abs() < 0.01,
            "{orientation:?}: root={:?} pane={:?}, ratio={:?}",
            app.world().get::<ComputedNode>(root),
            app.world().get::<ComputedNode>(children[0]),
            WidgetrySplitter::split_ratio(app.world(), root)
        );
        assert!((extent(app.world(), children[2], orientation) - 700.0).abs() < 0.01);
        let [a, b] = [children[0], children[2]];
        for e in [a, b] {
            let mut n = app.world_mut().get_mut::<Node>(e).unwrap();
            n.max_width = px(300);
            n.max_height = px(300);
        }
        settle(&mut app);
        assert!((extent(app.world(), root, orientation) - 664.4).abs() < 0.01);
        for e in [a, b] {
            assert!((extent(app.world(), e, orientation) - 323.2).abs() < 0.01);
        }
    }
}

#[test]
fn percentage_content_box_insets_do_not_initialize_against_transient_root_bounds() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let mut app = picking_app();
        app.add_plugins(WidgetrySplitterPlugin);
        spawn_ui_camera(&mut app, UVec2::splat(1600), 1.25);
        let root = spawn_scene(app.world_mut(), bsn! {
            @WidgetrySplitter { @orientation: {orientation}, @split_ratio: 0.3,
                @pane_a: bsn! { Node { box_sizing: BoxSizing::ContentBox, padding: UiRect::all(percent(10)), max_width: px(300), max_height: px(300) } },
                @pane_b: bsn! { Node { box_sizing: BoxSizing::ContentBox, padding: UiRect::all(percent(10)), max_width: px(300), max_height: px(300) } },
            } Node { width: px(1008), height: px(1008) }
        }).unwrap();
        settle(&mut app);
        assert!((extent(app.world(), root, orientation) - 1008.0).abs() < 0.01);
        assert!(
            (WidgetrySplitter::split_ratio(app.world(), root).unwrap() - 0.4984).abs() < 0.0001,
            "{orientation:?}"
        );
    }
}
