#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::expect_used)]

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_tabs::{WidgetryTab, WidgetryTabs, WidgetryTabsPlugin};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, primary_click, scene_app};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize)>);

fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryTabsPlugin)
        .init_resource::<Changes>()
        .add_observer(
            |event: On<ValueChange<usize>>, mut world: bevy::ecs::world::DeferredWorld| {
                assert_eq!(
                    WidgetryTabs::selected_index(&world, event.source).unwrap(),
                    event.value
                );
                assert_selected(&world, event.source, event.value, 3);
                assert!(event.is_final);
                world
                    .resource_mut::<Changes>()
                    .0
                    .push((event.source, event.value));
            },
        );
    app
}

fn item(index: usize) -> impl Scene {
    bsn! {
        @WidgetryTab {
            @header: {Some(Box::new(bsn! { Name::new(format!("Header {index}")) Children [Text(format!("Tab {index}"))] }) as Box<dyn Scene>)},
            @content: {Some(Box::new(bsn! { Name::new(format!("Content {index}")) Node bevy::ui::ScrollPosition(Vec2::new(0.0, 24.0)) Children [bevy::text::EditableText::new(format!("Value {index}"))] }) as Box<dyn Scene>)},
        }
    }
}

fn scene(selected: usize, count: usize) -> impl Scene {
    let items: Vec<Box<dyn Scene>> = (0..count)
        .map(|index| Box::new(item(index)) as Box<dyn Scene>)
        .collect();
    bsn! { @WidgetryTabs { @selected_index: selected, @children: {Some(Box::new(items) as Box<dyn SceneList>)} } }
}

fn assert_selected(world: &World, root: Entity, selected: usize, count: usize) {
    assert_eq!(WidgetryTabs::selected_index(world, root).unwrap(), selected);
    for index in 0..count {
        let header = WidgetryTabs::header(world, root, index).unwrap();
        let page = WidgetryTabs::page(world, root, index).unwrap();
        assert_eq!(world.get::<Checked>(header).is_some(), index == selected);
        assert_eq!(
            world.get::<Node>(page).unwrap().display,
            if index == selected {
                Display::Flex
            } else {
                Display::None
            }
        );
        assert_ne!(world.get::<ChildOf>(page).unwrap().parent(), header);
    }
}

#[test]
fn fixed_scene_list_initializes_default_and_explicit_selection_without_notifications() {
    let mut app = app();
    for selected in [0, 2] {
        let root = app
            .world_mut()
            .spawn_scene(scene(selected, 3))
            .unwrap()
            .id();
        app.update();
        assert_selected(app.world(), root, selected, 3);
    }
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn click_and_both_setters_share_atomic_commit_and_silent_same_value() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(scene(0, 3)).unwrap().id();
    let other = app.world_mut().spawn_scene(scene(0, 3)).unwrap().id();
    app.update();
    let header = WidgetryTabs::header(app.world(), root, 2).unwrap();
    let text = app.world().get::<Children>(header).unwrap()[0];
    app.world_mut().trigger(primary_click(text));
    app.world_mut().flush();
    assert_selected(app.world(), root, 2, 3);
    assert_selected(app.world(), other, 0, 3);
    assert!(!WidgetryTabs::set_selected_in_world(app.world_mut(), root, 2).unwrap());
    WidgetryTabs::set_selected(&mut app.world_mut().commands(), root, 1);
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 2);
    app.world_mut().flush();
    assert_selected(app.world(), root, 1, 3);
    assert!(WidgetryTabs::set_selected_in_world(app.world_mut(), root, 0).unwrap());
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, 2), (root, 1), (root, 0)]
    );
}

#[test]
fn invalid_requests_preserve_selection_pages_and_report_execution_time_errors() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app.world_mut().spawn_scene(scene(0, 3)).unwrap().id();
    app.update();
    let logs = LogCapture::default();
    logs.run(|| {
        let error = WidgetryTabs::set_selected_in_world(app.world_mut(), root, 3).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("out of bounds"));
        assert!(
            WidgetryTabs::set_selected_in_world(app.world_mut(), Entity::PLACEHOLDER, 0).is_err()
        );
    });
    assert!(
        logs.records()
            .iter()
            .any(|entry| entry.level == bevy::log::Level::ERROR)
    );
    assert_selected(app.world(), root, 0, 3);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryTabs::set_selected(&mut Commands::new(&mut queue, app.world()), root, 2);
    let header = WidgetryTabs::header(app.world(), root, 1).unwrap();
    app.world_mut().entity_mut(header).remove::<WidgetryTab>();
    let errors = ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    app.world_mut().entity_mut(header).insert(WidgetryTab);
    assert_selected(app.world(), root, 0, 3);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn pages_keep_entities_values_scroll_and_only_hidden_page_focus_is_cleared() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(scene(0, 3)).unwrap().id();
    app.update();
    let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
    let content = app.world().get::<Children>(page).unwrap()[0];
    let editor = app.world().get::<Children>(content).unwrap()[0];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    WidgetryTabs::set_selected_in_world(app.world_mut(), root, 1).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    let outside = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(outside, FocusCause::Navigated);
    WidgetryTabs::set_selected_in_world(app.world_mut(), root, 0).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(outside));
    assert_eq!(WidgetryTabs::page(app.world(), root, 0).unwrap(), page);
    assert_eq!(
        app.world()
            .get::<bevy::text::EditableText>(editor)
            .unwrap()
            .value()
            .to_string(),
        "Value 0"
    );
    assert_eq!(
        app.world()
            .get::<bevy::ui::ScrollPosition>(content)
            .unwrap()
            .0,
        Vec2::new(0.0, 24.0)
    );
}

#[test]
fn invalid_initial_structure_reports_once_without_breaking_healthy_instances() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    for (selected, count) in [(0, 0), (3, 3)] {
        app.world_mut().spawn_scene(scene(selected, count)).unwrap();
    }
    let healthy = app.world_mut().spawn_scene(scene(0, 3)).unwrap().id();
    let errors = ErrorCapture::default();
    for _ in 0..3 {
        errors.run(|| app.update());
    }
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert_selected(app.world(), healthy, 0, 3);
}

#[test]
fn missing_pair_is_a_fallible_scene_error_and_rejected_request_does_not_initialize() {
    let mut app = app();
    let error =
        bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! { @WidgetryTab }).unwrap_err();
    assert!(error.to_string().contains("both header and content"));
    let root = app.world_mut().spawn_scene(scene(0, 3)).unwrap().id();
    let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
    let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
    assert!(WidgetryTabs::set_selected_in_world(app.world_mut(), root, 3).is_err());
    assert_eq!(app.world().get::<ChildOf>(page).unwrap().parent(), header);
    assert_eq!(
        app.world().get::<Node>(page).unwrap().display,
        Display::None
    );
}

#[test]
fn disabled_headers_block_users_without_disabling_their_paired_pages() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(scene(0, 3)).unwrap().id();
    app.update();
    let header = WidgetryTabs::header(app.world(), root, 1).unwrap();
    let page = WidgetryTabs::page(app.world(), root, 1).unwrap();
    app.world_mut()
        .entity_mut(header)
        .insert(bevy::ui::InteractionDisabled);
    app.world_mut().trigger(primary_click(header));
    app.world_mut().flush();
    assert_selected(app.world(), root, 0, 3);
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert!(WidgetryTabs::set_selected_in_world(app.world_mut(), root, 1).unwrap());
    assert_selected(app.world(), root, 1, 3);
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(page)
            .is_none()
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 1)]);
}

#[test]
fn inherited_root_disabled_keeps_all_pages_disabled_across_programmatic_switches() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(scene(0, 3)).unwrap().id();
    app.update();
    let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
    app.world_mut()
        .entity_mut(header)
        .insert(bevy::ui::InteractionDisabled);
    let ancestor = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(root).insert(ChildOf(ancestor));
    app.world_mut()
        .entity_mut(ancestor)
        .insert(bevy::ui::InteractionDisabled);
    let target = WidgetryTabs::header(app.world(), root, 2).unwrap();
    app.world_mut().trigger(primary_click(target));
    app.world_mut().flush();
    assert_selected(app.world(), root, 0, 3);
    assert!(app.world().resource::<Changes>().0.is_empty());
    WidgetryTabs::set_selected(&mut app.world_mut().commands(), root, 2);
    app.world_mut().flush();
    assert_selected(app.world(), root, 2, 3);
    for index in 0..3 {
        let page = WidgetryTabs::page(app.world(), root, index).unwrap();
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(page)
                .is_some()
        );
    }
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 2)]);
    app.world_mut()
        .entity_mut(ancestor)
        .remove::<bevy::ui::InteractionDisabled>();
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(header)
            .is_some()
    );
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(target)
            .is_none()
    );
    app.world_mut().trigger(primary_click(header));
    app.world_mut().flush();
    assert_selected(app.world(), root, 2, 3);
}
