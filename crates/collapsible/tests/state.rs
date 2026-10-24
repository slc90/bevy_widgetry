#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::expect_used)]

use bevy::prelude::*;
use bevy_widgetry_collapsible::{
    WidgetryCollapsible, WidgetryCollapsibleChanged, WidgetryCollapsiblePlugin,
};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, bool)>);

fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin)
        .init_resource::<Changes>()
        .add_observer(
            |event: On<WidgetryCollapsibleChanged>, mut world: bevy::ecs::world::DeferredWorld| {
                assert_eq!(
                    WidgetryCollapsible::expanded(&world, event.entity).unwrap(),
                    event.expanded
                );
                world
                    .resource_mut::<Changes>()
                    .0
                    .push((event.entity, event.expanded));
            },
        );
    app
}

fn scene(expanded: bool) -> impl Scene {
    bsn! {
        @WidgetryCollapsible {
            @expanded: expanded,
            @header: {Some(Box::new(bsn! { Text("Settings") }) as Box<dyn Scene>)},
            @children: {Some(Box::new(bsn! { Node { display: Display::Grid } bevy::ui::ScrollPosition(Vec2::new(0.0, 24.0)) Children [Node bevy::text::EditableText::new("Retained") -- Node Visibility::Hidden] }) as Box<dyn SceneList>)},
        }
    }
}

fn content(world: &World, root: Entity) -> Entity {
    world.get::<Children>(root).unwrap()[1]
}

fn assert_projection(world: &World, root: Entity, expanded: bool) {
    let content = content(world, root);
    assert_eq!(
        WidgetryCollapsible::expanded(world, root).unwrap(),
        expanded
    );
    assert_eq!(
        world.get::<Node>(content).unwrap().display,
        if expanded {
            Display::Flex
        } else {
            Display::None
        }
    );
    assert_eq!(
        *world.get::<Visibility>(content).unwrap(),
        if expanded {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    );
    assert_eq!(
        world.get::<InheritedVisibility>(content).unwrap().get(),
        expanded
    );
}

#[test]
fn default_and_initial_collapsed_do_not_notify() {
    let mut app = app();
    let default = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCollapsible })
        .unwrap()
        .id();
    let collapsed = app.world_mut().spawn_scene(scene(false)).unwrap().id();
    app.update();
    assert_projection(app.world(), default, true);
    assert_projection(app.world(), collapsed, false);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn setters_commit_before_notifications_keep_children_and_do_not_notify_noops() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(scene(true)).unwrap().id();
    app.update();
    let content = content(app.world(), root);
    let layout = app.world().get::<Children>(content).unwrap()[0];
    let editor = app.world().get::<Children>(layout).unwrap()[0];
    let hidden = app.world().get::<Children>(layout).unwrap()[1];
    for expanded in [false, true, false] {
        assert!(
            WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, expanded).unwrap()
        );
        assert_projection(app.world(), root, expanded);
        assert_eq!(
            app.world()
                .get::<InheritedVisibility>(editor)
                .unwrap()
                .get(),
            expanded
        );
        assert!(
            !app.world()
                .get::<InheritedVisibility>(hidden)
                .unwrap()
                .get()
        );
        assert_eq!(
            app.world().get::<Node>(layout).unwrap().display,
            Display::Grid
        );
        assert_eq!(
            app.world()
                .get::<bevy::text::EditableText>(editor)
                .unwrap()
                .value()
                .to_string(),
            "Retained"
        );
        assert_eq!(
            app.world()
                .get::<bevy::ui::ScrollPosition>(layout)
                .unwrap()
                .0
                .y,
            24.0
        );
        assert!(
            !WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, expanded).unwrap()
        );
    }
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryCollapsible::set_expanded(&mut Commands::new(&mut queue, app.world()), root, true);
    assert_projection(app.world(), root, false);
    queue.apply(app.world_mut());
    assert_projection(app.world(), root, true);
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, false), (root, true), (root, false), (root, true)]
    );
}

#[test]
fn invalid_requests_log_and_report_errors_without_committing() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app.world_mut().spawn_scene(scene(true)).unwrap().id();
    app.update();
    let wrong = app.world_mut().spawn(Node::default()).id();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    let logs = LogCapture::default();
    logs.run(|| {
        for invalid in [wrong, stale] {
            let err = WidgetryCollapsible::set_expanded_in_world(app.world_mut(), invalid, false)
                .unwrap_err();
            assert_eq!(err.severity(), bevy::ecs::error::Severity::Error);
        }
    });
    assert!(
        logs.records()
            .iter()
            .any(|entry| entry.level == bevy::log::Level::ERROR)
    );
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryCollapsible::set_expanded(&mut Commands::new(&mut queue, app.world()), root, false);
    let content = content(app.world(), root);
    app.world_mut().entity_mut(content).remove::<Node>();
    let errors = ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    assert_eq!(
        errors.take()[0].severity(),
        bevy::ecs::error::Severity::Error
    );
    app.world_mut().entity_mut(content).insert(Node::default());
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn collapsed_content_has_no_layout_height() {
    let mut app = app();
    bevy_widgetry_test_utils::add_ui_plugins(&mut app);
    bevy_widgetry_test_utils::spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryCollapsible { @children: {Some(Box::new(bsn! { Node { height: px(80), width: px(80) } }) as Box<dyn SceneList>)} }
    }).unwrap().id();
    app.update();
    let content = content(app.world(), root);
    assert!(app.world().get::<ComputedNode>(content).unwrap().size().y >= 80.0);
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<ComputedNode>(content).unwrap().size().y,
        0.0
    );
}
