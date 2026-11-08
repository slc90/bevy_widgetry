#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry_ui_scale::{
    WidgetryUiScale, WidgetryUiScalePlugin, WidgetryUiScaleSystems, WindowScaleExcluded,
};

const DESIGN: UVec2 = UVec2::new(640, 400);

#[derive(Resource, Default)]
struct ChangedWindows(usize);

fn count_changed_windows(
    windows: Query<Entity, Changed<Window>>,
    mut count: ResMut<ChangedWindows>,
) {
    count.0 = windows.iter().count();
}

#[test]
fn static_and_same_scale_frames_do_not_notify_window_consumers() {
    let mut app = app();
    app.init_resource::<ChangedWindows>().add_systems(
        PostUpdate,
        count_changed_windows.after(WidgetryUiScaleSystems::Sync),
    );
    app.world_mut().spawn(window());
    app.update();
    assert_eq!(app.world().resource::<ChangedWindows>().0, 1);
    app.update();
    assert_eq!(app.world().resource::<ChangedWindows>().0, 0);
    assert!(
        !app.world_mut()
            .resource_mut::<WidgetryUiScale>()
            .set(1.0)
            .unwrap()
    );
    app.update();
    assert_eq!(app.world().resource::<ChangedWindows>().0, 0);
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(1.25)
        .unwrap();
    app.update();
    assert_eq!(app.world().resource::<ChangedWindows>().0, 1);
    app.update();
    assert_eq!(app.world().resource::<ChangedWindows>().0, 0);
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(WidgetryUiScalePlugin);
    app
}

fn window() -> Window {
    Window {
        resolution: DESIGN.into(),
        ..default()
    }
}

#[test]
fn normal_windows_share_current_scale_preserving_logical_size() {
    let mut app = app();
    let first = app.world_mut().spawn(window()).id();
    app.update();
    assert_eq!(
        app.world().get::<Window>(first).unwrap().scale_factor(),
        1.0
    );
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(1.25)
        .unwrap();
    let second = app.world_mut().spawn(window()).id();
    app.update();
    for entity in [first, second] {
        let window = app.world().get::<Window>(entity).unwrap();
        assert_eq!(window.physical_size(), UVec2::new(800, 500));
        assert_eq!(window.resolution.size(), DESIGN.as_vec2());
    }
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(2.0)
        .unwrap();
    app.update();
    assert_eq!(
        app.world().get::<Window>(second).unwrap().physical_size(),
        DESIGN * 2
    );
}

#[test]
fn excluded_window_is_never_scaled_even_during_bundle_creation() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(2.0)
        .unwrap();
    let entity = app.world_mut().spawn((window(), WindowScaleExcluded)).id();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().physical_size(),
        DESIGN
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<Window>(entity)
            .unwrap()
            .resolution
            .scale_factor_override(),
        None
    );
    app.world_mut()
        .entity_mut(entity)
        .remove::<WindowScaleExcluded>();
    app.update();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().physical_size(),
        DESIGN * 2
    );
    app.world_mut()
        .entity_mut(entity)
        .insert(WindowScaleExcluded);
    app.world_mut()
        .get_mut::<Window>(entity)
        .unwrap()
        .resolution
        .set_scale_factor_override(Some(1.0));
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(1.25)
        .unwrap();
    app.update();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().scale_factor(),
        1.0
    );
}

#[test]
fn user_resize_becomes_the_next_scale_baseline() {
    let mut app = app();
    let entity = app.world_mut().spawn(window()).id();
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(1.25)
        .unwrap();
    app.update();
    app.world_mut()
        .get_mut::<Window>(entity)
        .unwrap()
        .resolution
        .set_physical_resolution(1000, 625);
    app.update();
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(2.0)
        .unwrap();
    app.update();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().physical_size(),
        UVec2::new(1600, 1000)
    );
}

#[test]
fn windows_existing_before_plugin_keep_their_logical_design_size() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut initial = window();
    initial.resolution.set_scale_factor_override(Some(2.0));
    initial
        .resolution
        .set_physical_resolution(DESIGN.x * 2, DESIGN.y * 2);
    let entity = app.world_mut().spawn(initial).id();
    app.insert_resource(WidgetryUiScale::new(1.25).unwrap())
        .add_plugins(WidgetryUiScalePlugin);
    app.update();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().physical_size(),
        UVec2::new(800, 500)
    );
}

#[test]
fn reinserting_window_on_an_existing_entity_uses_the_new_design() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(2.0)
        .unwrap();
    let entity = app.world_mut().spawn(window()).id();
    app.update();
    app.world_mut().entity_mut(entity).remove::<Window>();
    app.world_mut().entity_mut(entity).insert(window());
    app.update();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().physical_size(),
        DESIGN * 2
    );
}

#[test]
fn runtime_constraints_are_applied_and_can_be_relaxed_again() {
    let mut app = app();
    let entity = app.world_mut().spawn(window()).id();
    app.update();
    app.world_mut()
        .get_mut::<Window>(entity)
        .unwrap()
        .resize_constraints
        .max_width = 500.0;
    app.update();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().physical_width(),
        500
    );
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(2.0)
        .unwrap();
    app.update();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().physical_width(),
        1000
    );
    app.world_mut()
        .get_mut::<Window>(entity)
        .unwrap()
        .resize_constraints
        .max_width = f32::INFINITY;
    app.update();
    assert_eq!(
        app.world().get::<Window>(entity).unwrap().physical_size(),
        DESIGN * 2
    );
}
