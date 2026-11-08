#![cfg(feature = "ui_scale")]
#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry::{
    WindowScaleExcluded,
    ui_scale::{WidgetryUiScale, WidgetryUiScalePlugin},
};

#[test]
fn facade_can_install_global_scaling_and_exclude_another_window() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(WidgetryUiScale::new(2.0).unwrap())
        .add_plugins(WidgetryUiScalePlugin);
    let normal = app.world_mut().spawn(Window::default()).id();
    let excluded = app
        .world_mut()
        .spawn((Window::default(), WindowScaleExcluded))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Window>(normal).unwrap().scale_factor(),
        2.0
    );
    assert_eq!(
        app.world().get::<Window>(excluded).unwrap().scale_factor(),
        1.0
    );
}
