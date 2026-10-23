#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{ecs::error::Severity, prelude::*, ui_widgets::ValueChange};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_splitter::{
    WidgetrySplitter, WidgetrySplitterOrientation as Orientation, WidgetrySplitterPlugin,
};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, picking_app, spawn_ui_camera};

fn frames(app: &mut App) {
    for _ in 0..12 {
        app.update();
    }
}
fn size(world: &World, entity: Entity) -> Vec2 {
    let node = world.get::<ComputedNode>(entity).unwrap();
    node.size() * node.inverse_scale_factor
}
fn parts(world: &World, root: Entity) -> [Entity; 3] {
    let children = world.get::<Children>(root).unwrap();
    [children[0], children[1], children[2]]
}
fn app() -> App {
    let mut app = picking_app();
    app.add_plugins(WidgetrySplitterPlugin);
    spawn_ui_camera(&mut app, UVec2::splat(1600), 1.0);
    app
}
fn fixture(orientation: Orientation, ratio: f32, min: f32, max: Val) -> (App, Entity, Entity) {
    let mut app = app();
    let parent = spawn_scene(app.world_mut(), bsn! {
        Node { width: px(1008), height: px(1008) }
        Children [Name::new("CallerSplitter") @WidgetrySplitter {
            @orientation: {orientation}, @split_ratio: ratio,
            @pane_a: bsn! { Name::new("CallerPaneA") Node { min_width: px(min), min_height: px(min), max_width: max, max_height: max } Children [Name::new("Content") Node { width: px(10), height: px(10) }] },
            @pane_b: bsn! { Name::new("CallerPaneB") Node { min_width: px(0), min_height: px(0) } },
        }]
    }).unwrap();
    let root = app.world().get::<Children>(parent).unwrap()[0];
    frames(&mut app);
    (app, parent, root)
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 1.0, "{actual} != {expected}");
}

#[test]
fn real_layout_respects_bsn_pane_patch_and_axis() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let (app, _, root) = fixture(orientation, 0.3, 0.0, Val::Auto);
        let [a, h, b] = parts(app.world(), root);
        let axis = |v: Vec2| {
            if orientation == Orientation::Horizontal {
                v.x
            } else {
                v.y
            }
        };
        close(axis(size(app.world(), a)), 300.0);
        close(axis(size(app.world(), b)), 700.0);
        close(axis(size(app.world(), h)), 8.0);
        assert_eq!(
            app.world().get::<Name>(root).unwrap().as_str(),
            "CallerSplitter"
        );
        assert_eq!(app.world().get::<Name>(a).unwrap().as_str(), "CallerPaneA");
        assert_eq!(app.world().get::<Name>(b).unwrap().as_str(), "CallerPaneB");
        let positions: Vec<_> = [a, h, b]
            .map(|e| app.world().get::<UiGlobalTransform>(e).unwrap().translation)
            .into();
        assert!(axis(positions[0]) < axis(positions[1]) && axis(positions[1]) < axis(positions[2]));
        assert_eq!(
            WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
            0.3
        );
    }
}

#[test]
fn passive_resize_and_constraint_changes_keep_initialized_target() {
    let (mut app, parent, root) = fixture(Orientation::Horizontal, 0.3, 210.0, Val::Auto);
    let [a, _, b] = parts(app.world(), root);
    for width in [608.0_f32, 1008.0] {
        app.world_mut().get_mut::<Node>(parent).unwrap().width = px(width);
        frames(&mut app);
        close(
            size(app.world(), a).x,
            (width - 8.0).mul_add(0.3, 0.0).max(210.0),
        );
        assert_eq!(
            WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
            0.3
        );
        assert_eq!(app.world().get::<Node>(a).unwrap().min_width, px(210));
    }
    app.world_mut().get_mut::<Node>(a).unwrap().max_width = px(300);
    app.world_mut().get_mut::<Node>(b).unwrap().max_width = px(300);
    frames(&mut app);
    close(size(app.world(), root).x, 608.0);
    close(size(app.world(), a).x, 300.0);
    close(size(app.world(), b).x, 300.0);
    app.world_mut().get_mut::<Node>(b).unwrap().max_width = Val::Auto;
    frames(&mut app);
    close(size(app.world(), root).x, 1008.0);
    assert_eq!(app.world().get::<Node>(root).unwrap().max_width, Val::Auto);
    app.world_mut().get_mut::<Node>(b).unwrap().min_width = px(250);
    app.world_mut().get_mut::<Node>(parent).unwrap().width = px(100);
    frames(&mut app);
    close(size(app.world(), root).x, 468.0);
    close(size(app.world(), a).x, 210.0);
    close(size(app.world(), b).x, 250.0);
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.3
    );
}

#[derive(Resource, Default)]
struct Notifications(usize);

#[test]
fn first_valid_layout_clamps_once_and_initialization_is_silent() {
    let mut app = app();
    app.init_resource::<Notifications>();
    app.world_mut()
        .add_observer(|_: On<ValueChange<f32>>, mut events: ResMut<Notifications>| events.0 += 1);
    for (ratio, expected) in [(0.1_f32, 0.3_f32), (0.9, 0.8)] {
        let root = spawn_scene(
            app.world_mut(),
            bsn! {
                @WidgetrySplitter {
                    @split_ratio: ratio,
                    @pane_a: bsn! { Node { min_width: px(300), max_width: px(800) } },
                } Node { width: px(1008), height: px(100) }
            },
        )
        .unwrap();
        frames(&mut app);
        close(
            WidgetrySplitter::split_ratio(app.world(), root).unwrap() * 1000.0,
            expected * 1000.0,
        );
    }
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetrySplitter Node {width: px(1008), height: px(100)} },
    )
    .unwrap();
    frames(&mut app);
    close(size(app.world(), parts(app.world(), root)[0]).x, 500.0);
    assert_eq!(app.world().resource::<Notifications>().0, 0);
}

#[test]
fn nested_splitters_preserve_separate_panes_and_ratios() {
    let mut app = app();
    let outer = spawn_scene(app.world_mut(), bsn! {
        @WidgetrySplitter {
            @split_ratio: 0.3,
            @pane_b: bsn! { Children [Name::new("Nested") @WidgetrySplitter { @orientation: Orientation::Vertical, @split_ratio: 0.25 }] },
        } Node {width: px(1008), height: px(408)}
    }).unwrap();
    frames(&mut app);
    let b = parts(app.world(), outer)[2];
    let nested = app.world().get::<Children>(b).unwrap()[0];
    let [a, h, b] = parts(app.world(), nested);
    close(size(app.world(), a).y, 100.0);
    close(size(app.world(), b).y, 300.0);
    close(size(app.world(), h).y, 8.0);
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), outer).unwrap(),
        0.3
    );
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), nested).unwrap(),
        0.25
    );
}

#[test]
fn invalid_construction_and_runtime_constraints_reach_error_channel() {
    let mut app = app();
    let before = app.world().entities().len();
    let log = LogCapture::default();
    log.run(|| {
        for ratio in [f32::NAN, -0.1, 1.1, f32::INFINITY] {
            assert!(
                spawn_scene(
                    app.world_mut(),
                    bsn! { @WidgetrySplitter { @split_ratio: ratio } }
                )
                .is_err()
            );
        }
        for thickness in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(
                spawn_scene(
                    app.world_mut(),
                    bsn! { @WidgetrySplitter { @handle_thickness: thickness } }
                )
                .is_err()
            );
        }
    });
    assert_eq!(app.world().entities().len(), before);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let errors = ErrorCapture::default();
    let root = spawn_scene(app.world_mut(), bsn! { @WidgetrySplitter { @pane_a: bsn! { Node { min_width: px(400), max_width: px(300) } } } Node {width: px(1008),height: px(100)} }).unwrap();
    errors.run(|| frames(&mut app));
    let failures = errors.take();
    assert!(!failures.is_empty());
    assert!(
        failures
            .iter()
            .all(|error| error.severity() == Severity::Error)
    );
    let pane = parts(app.world(), root)[0];
    app.world_mut().get_mut::<Node>(pane).unwrap().min_width = px(0);
    frames(&mut app);
    let handle = parts(app.world(), root)[1];
    app.world_mut().despawn(handle);
    errors.run(|| frames(&mut app));
    assert!(!errors.take().is_empty());
}
