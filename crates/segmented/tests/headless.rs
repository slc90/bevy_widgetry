#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::expect_used)]

use bevy::input_focus::{
    InputFocus,
    tab_navigation::{TabGroup, TabIndex},
};
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_segmented::{
    WidgetrySegmented, WidgetrySegmentedOption, WidgetrySegmentedPlugin,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_keyboard_dispatch, press_key, primary_click, primary_press,
    scene_app,
};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize, bool)>);

fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetrySegmentedPlugin)
        .init_resource::<Changes>()
        .add_observer(
            |event: On<ValueChange<usize>>,
             roots: Query<&Children, With<WidgetrySegmented>>,
             checked: Query<(), With<Checked>>,
             mut changes: ResMut<Changes>| {
                let children = roots.get(event.source).unwrap();
                assert_eq!(
                    children
                        .iter()
                        .filter(|child| checked.contains(*child))
                        .count(),
                    1
                );
                assert!(checked.contains(children[event.value]));
                changes.0.push((event.source, event.value, event.is_final));
            },
        );
    app
}

fn selected(app: &App, root: Entity, index: usize) {
    for (position, child) in app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(
            app.world().get::<Checked>(child).is_some(),
            position == index
        );
    }
}

#[test]
fn default_selection_and_queued_programmatic_selection_are_exclusive() {
    let mut app = app();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetrySegmented Children [@WidgetrySegmentedOption-- @WidgetrySegmentedOption]
        })
        .unwrap()
        .id();
    app.update();
    selected(&app, root, 0);
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    selected(&app, root, 1);
}

#[test]
fn one_initial_checked_is_adopted_and_caller_name_is_preserved() {
    let mut app = app();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetrySegmented Name("CallerSegmented") Children [
                @WidgetrySegmentedOption-- @WidgetrySegmentedOption Checked
            ]
        })
        .unwrap()
        .id();
    app.update();
    selected(&app, root, 1);
    assert_eq!(
        app.world().get::<Name>(root).unwrap().as_str(),
        "CallerSegmented"
    );
}

fn three() -> impl Scene {
    bsn! { @WidgetrySegmented Children [
        @WidgetrySegmentedOption Children [Text("A")]--
        @WidgetrySegmentedOption Children [Text("B")]--
        @WidgetrySegmentedOption Children [Text("C")]
    ] }
}

#[test]
fn conflicting_initial_checked_reports_once_and_recovers_after_marker_removal() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetrySegmented Children [
            @WidgetrySegmentedOption Checked-- @WidgetrySegmentedOption Checked
        ] })
        .unwrap()
        .id();
    let healthy = app.world_mut().spawn_scene(three()).unwrap().id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    for _ in 0..3 {
        errors.run(|| logs.run(|| app.update()));
    }
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
    assert!(failures[0].to_string().contains("multiple initial Checked"));
    let children = app.world().get::<Children>(root).unwrap().to_vec();
    assert!(
        children
            .iter()
            .all(|child| app.world().get::<Checked>(*child).is_some())
    );
    selected(&app, healthy, 0);
    app.world_mut().entity_mut(children[0]).remove::<Checked>();
    errors.run(|| logs.run(|| app.update()));
    assert!(errors.take().is_empty());
    selected(&app, root, 1);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.fields.get("message").is_some_and(|m| m.contains("恢复")))
            .count(),
        1
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn clicks_from_content_commit_before_notification_and_same_value_is_silent() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(three()).unwrap().id();
    let other = app.world_mut().spawn_scene(three()).unwrap().id();
    app.update();
    let option = app.world().get::<Children>(root).unwrap()[2];
    let content = app.world().get::<Children>(option).unwrap()[0];
    for _ in 0..2 {
        app.world_mut().trigger(primary_click(content));
        app.world_mut().flush();
        selected(&app, root, 2);
        selected(&app, other, 0);
        assert_eq!(app.world().resource::<Changes>().0, vec![(root, 2, true)]);
    }
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 0);
    app.world_mut().flush();
    selected(&app, root, 0);
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
}

#[test]
fn disabled_input_is_blocked_while_programmatic_and_repair_selection_remain_legal() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(three()).unwrap().id();
    app.update();
    let children = app.world().get::<Children>(root).unwrap().to_vec();
    app.world_mut()
        .entity_mut(children[1])
        .insert(InteractionDisabled);
    app.update();
    app.world_mut().trigger(primary_click(children[1]));
    app.world_mut().flush();
    selected(&app, root, 0);
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    selected(&app, root, 1);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    app.world_mut().trigger(primary_click(children[2]));
    app.world_mut().flush();
    selected(&app, root, 1);
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 2);
    app.world_mut().flush();
    selected(&app, root, 2);
    app.world_mut().despawn(children[2]);
    app.update();
    selected(&app, root, 1);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn focused_horizontal_keys_skip_disabled_stop_at_edges_and_ignore_other_axes() {
    let mut app = app();
    app.init_resource::<ButtonInput<KeyCode>>();
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let root = app.world_mut().spawn_scene(three()).unwrap().id();
    app.update();
    press_key(&mut app, window, KeyCode::ArrowRight);
    selected(&app, root, 0);
    let children = app.world().get::<Children>(root).unwrap().to_vec();
    app.world_mut()
        .entity_mut(children[1])
        .insert(InteractionDisabled);
    app.world_mut().trigger(primary_press(children[0]));
    app.world_mut().flush();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    for (key, index) in [
        (KeyCode::ArrowLeft, 0),
        (KeyCode::ArrowRight, 2),
        (KeyCode::ArrowRight, 2),
        (KeyCode::ArrowUp, 2),
        (KeyCode::ArrowDown, 2),
        (KeyCode::Home, 2),
        (KeyCode::End, 2),
        (KeyCode::ArrowLeft, 0),
    ] {
        press_key(&mut app, window, key);
        selected(&app, root, index);
    }
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, 2, true), (root, 0, true)]
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    press_key(&mut app, window, KeyCode::ArrowRight);
    selected(&app, root, 0);
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    selected(&app, root, 1);
    assert_eq!(app.world().get::<TabIndex>(root).unwrap().0, -1);
}

#[test]
fn tab_passes_between_siblings_without_entering_segmented() {
    let mut app = app();
    app.init_resource::<ButtonInput<KeyCode>>();
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let container = app
        .world_mut()
        .spawn_scene(bsn! { Node TabGroup::default() Children [
            Node TabIndex(0)-- @three()-- Node TabIndex(0)
        ] })
        .unwrap()
        .id();
    app.update();
    let children = app.world().get::<Children>(container).unwrap().to_vec();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(children[0], bevy::input_focus::FocusCause::Navigated);
    press_key(&mut app, window, KeyCode::Tab);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(children[2])
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    press_key(&mut app, window, KeyCode::Tab);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(children[0])
    );
}

#[test]
fn dynamic_removal_addition_and_full_replacement_are_silent() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(three()).unwrap().id();
    app.update();
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    let children = app.world().get::<Children>(root).unwrap().to_vec();
    app.world_mut().despawn(children[1]);
    app.update();
    selected(&app, root, 1);
    let temporary = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetrySegmentedOption Checked ChildOf(root) })
        .unwrap()
        .id();
    app.update();
    selected(&app, root, 1);
    app.world_mut().despawn(children[0]);
    app.world_mut().despawn(children[2]);
    app.world_mut().despawn(temporary);
    app.world_mut()
        .entity_mut(root)
        .apply_scene(bsn! { Children [
            @WidgetrySegmentedOption-- @WidgetrySegmentedOption Checked
        ] })
        .unwrap();
    app.update();
    selected(&app, root, 0);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn invalid_queued_requests_do_not_partially_change_state_or_notify() {
    for case in 0..5 {
        let mut app = app();
        app.set_error_handler(ErrorCapture::handler());
        let root = app.world_mut().spawn_scene(three()).unwrap().id();
        let healthy = app.world_mut().spawn_scene(three()).unwrap().id();
        app.update();
        let children = app.world().get::<Children>(root).unwrap().to_vec();
        match case {
            0 => WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 3),
            1 => {
                app.world_mut()
                    .commands()
                    .entity(root)
                    .remove::<WidgetrySegmented>();
                WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
            }
            2 => {
                app.world_mut().commands().entity(root).despawn();
                WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
            }
            3 => {
                app.world_mut()
                    .commands()
                    .entity(children[2])
                    .remove::<WidgetrySegmentedOption>();
                WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
            }
            _ => {
                for child in children {
                    app.world_mut().despawn(child);
                }
                WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 0);
            }
        }
        WidgetrySegmented::set_selected(&mut app.world_mut().commands(), healthy, 2);
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        errors.run(|| logs.run(|| app.world_mut().flush()));
        let failures = errors.take();
        assert_eq!(failures.len(), 1, "case {case}");
        assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(
            logs.records()
                .iter()
                .filter(|r| r.level == bevy::log::Level::ERROR)
                .count(),
            1
        );
        if matches!(case, 0 | 1 | 3) {
            selected(&app, root, 0);
        }
        selected(&app, healthy, 2);
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn queued_selection_uses_current_children_and_same_value_preserves_checked_tick() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(three()).unwrap().id();
    app.update();
    let first = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut().commands().entity(first).despawn();
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    selected(&app, root, 1);
    let chosen = app.world().get::<Children>(root).unwrap()[1];
    let tick = app
        .world()
        .entity(chosen)
        .get_ref::<Checked>()
        .unwrap()
        .last_changed();
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .entity(chosen)
            .get_ref::<Checked>()
            .unwrap()
            .last_changed(),
        tick
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn nested_segmented_does_not_change_outer_native_radio_group() {
    use bevy_widgetry_radio_group::{
        WidgetryRadioGroup, WidgetryRadioGroupPlugin, WidgetryRadioOption,
    };
    let mut app = app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    let outer = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryRadioGroup Children [
            @WidgetryRadioOption Children [@three()]-- @WidgetryRadioOption
        ] })
        .unwrap()
        .id();
    app.update();
    let outer_options = app.world().get::<Children>(outer).unwrap().to_vec();
    let inner = app.world().get::<Children>(outer_options[0]).unwrap()[1];
    let target = app.world().get::<Children>(inner).unwrap()[2];
    app.world_mut().trigger(primary_click(target));
    app.world_mut().flush();
    selected(&app, inner, 2);
    selected(&app, outer, 0);
    assert_eq!(app.world().resource::<Changes>().0, vec![(inner, 2, true)]);
}
