#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    ecs::{
        error::Severity,
        world::{CommandQueue, DeferredWorld},
    },
    prelude::*,
    ui_widgets::ValueChange,
};
use bevy_widgetry_core::scene::{WidgetrySceneCommandsExt, spawn_scene};
use bevy_widgetry_splitter::{
    WidgetrySplitter, WidgetrySplitterOrientation as Orientation, WidgetrySplitterPlugin,
};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, picking_app, spawn_ui_camera};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, f32, bool)>);
fn settle(app: &mut App) {
    for _ in 0..12 {
        app.update();
    }
}
fn fixture(orientation: Orientation) -> (App, Entity) {
    let mut app = picking_app();
    app.add_plugins(WidgetrySplitterPlugin)
        .init_resource::<Changes>();
    spawn_ui_camera(&mut app, UVec2::splat(1600), 1.25);
    app.add_observer(|event: On<ValueChange<f32>>, mut world: DeferredWorld| {
        assert_eq!(
            WidgetrySplitter::split_ratio(&world, event.source).unwrap(),
            event.value
        );
        world
            .resource_mut::<Changes>()
            .0
            .push((event.source, event.value, event.is_final));
    });
    let root=spawn_scene(app.world_mut(),bsn! {
        @WidgetrySplitter { @orientation: {orientation},
            @pane_a: bsn! {Node {min_width:px(300),max_width:px(800),min_height:px(300),max_height:px(800)}},
        } Node {width:px(1008),height:px(1008)}
    }).unwrap();
    settle(&mut app);
    (app, root)
}

#[test]
fn world_setter_clamps_and_notifies_after_commit_in_both_axes() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let (mut app, root) = fixture(orientation);
        for (request, expected) in [(0.1_f32, 0.3_f32), (0.9, 0.8), (0.5, 0.5)] {
            assert!(
                WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, request).unwrap()
            );
            assert_eq!(
                WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
                expected
            );
            assert!(
                !WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, request)
                    .unwrap()
            );
            settle(&mut app);
            let a = app.world().get::<Children>(root).unwrap()[0];
            let c = app.world().get::<ComputedNode>(a).unwrap();
            let v = c.unrounded_size * c.inverse_scale_factor;
            let extent = if orientation == Orientation::Horizontal {
                v.x
            } else {
                v.y
            };
            assert!((extent - expected * 1000.0).abs() < 0.01);
        }
        assert_eq!(
            app.world().resource::<Changes>().0,
            vec![(root, 0.3, true), (root, 0.8, true), (root, 0.5, true)]
        );
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(608);
        app.world_mut().get_mut::<Node>(root).unwrap().height = px(608);
        settle(&mut app);
        assert_eq!(
            WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
            0.5
        );
        assert_eq!(app.world().resource::<Changes>().0.len(), 3);
    }
}

#[test]
fn invalid_input_layout_and_structure_never_commit_or_notify() {
    let (mut app, root) = fixture(Orientation::Horizontal);
    let log = LogCapture::default();
    log.run(|| {
        for ratio in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1] {
            assert_eq!(
                WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, ratio)
                    .unwrap_err()
                    .severity(),
                Severity::Error
            );
            assert_eq!(
                WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
                0.5
            );
        }
    });
    assert_eq!(log.records().len(), 5);
    let unlaid = spawn_scene(app.world_mut(), bsn! {@WidgetrySplitter}).unwrap();
    let bare = app.world_mut().spawn_empty().id();
    let incomplete = app.world_mut().spawn(WidgetrySplitter).id();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    for target in [unlaid, bare, incomplete, stale] {
        assert_eq!(
            WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), target, 0.3)
                .unwrap_err()
                .severity(),
            Severity::Error
        );
    }
    let handle = app.world().get::<Children>(root).unwrap()[1];
    app.world_mut().despawn(handle);
    assert!(WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 0.3).is_err());
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn commands_validate_at_execution_and_failures_reach_host() {
    let (mut app, root) = fixture(Orientation::Horizontal);
    app.set_error_handler(ErrorCapture::handler());
    let stale = spawn_scene(app.world_mut(), bsn! {@WidgetrySplitter}).unwrap();
    let mut queue = CommandQueue::default();
    let reserved = {
        let mut commands = Commands::new(&mut queue, app.world());
        WidgetrySplitter::set_split_ratio(&mut commands, root, 0.9);
        WidgetrySplitter::set_split_ratio(&mut commands, root, f32::NAN);
        WidgetrySplitter::set_split_ratio(&mut commands, stale, 0.4);
        let e = commands
            .spawn_scene_with_error_handler(bsn! {@WidgetrySplitter})
            .id();
        WidgetrySplitter::set_split_ratio(&mut commands, e, 0.4);
        e
    };
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
    app.world_mut().despawn(stale);
    let errors = ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    let failures = errors.take();
    assert_eq!(failures.len(), 3);
    assert!(failures.iter().all(|e| e.severity() == Severity::Error));
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.8
    );
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), reserved).unwrap(),
        0.5
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 0.8, true)]);
}

#[test]
fn fractional_endpoint_clamps_never_escape_unit_interval() {
    for minimum in [0.001_f32, 0.002, 0.1, 100.1, 300.3] {
        let mut app = picking_app();
        app.add_plugins(WidgetrySplitterPlugin);
        spawn_ui_camera(&mut app, UVec2::splat(1600), 1.25);
        let root = spawn_scene(
            app.world_mut(),
            bsn! {
                @WidgetrySplitter { @split_ratio:0.1,
                    @pane_a: bsn! {Node {min_width:px(minimum)}},
                    @pane_b: bsn! {Node {max_width:px(0)}},
                } Node {width:px(minimum+8.0),height:px(100)}
            },
        )
        .unwrap();
        settle(&mut app);
        let ratio = WidgetrySplitter::split_ratio(app.world(), root).unwrap();
        assert!((0.0..=1.0).contains(&ratio), "{minimum}: {ratio}");
        WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 1.0).unwrap();
        assert_eq!(
            WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
            1.0
        );
    }
}

#[test]
fn hidden_layout_and_new_conflicting_constraints_reject_without_change() {
    let (mut app, root) = fixture(Orientation::Horizontal);
    app.world_mut().get_mut::<Node>(root).unwrap().display = Display::None;
    settle(&mut app);
    assert!(WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 0.8).is_err());
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
    app.world_mut().get_mut::<Node>(root).unwrap().display = Display::Flex;
    settle(&mut app);
    let a = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut().get_mut::<Node>(a).unwrap().max_width = px(200);
    assert!(WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 0.8).is_err());
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn active_clamp_replaces_a_passively_unreachable_saved_target() {
    let (mut app, root) = fixture(Orientation::Horizontal);
    WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 0.3).unwrap();
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(608);
    settle(&mut app);
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.3
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
    assert!(WidgetrySplitter::set_split_ratio_in_world(app.world_mut(), root, 0.1).unwrap());
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(1008);
    settle(&mut app);
    assert_eq!(
        WidgetrySplitter::split_ratio(app.world(), root).unwrap(),
        0.5
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, 0.3, true), (root, 0.5, true)]
    );
}
