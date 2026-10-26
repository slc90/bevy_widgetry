#![allow(clippy::disallowed_macros)]

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_widgetry_power_inhibit::{PowerError, PowerInhibit, WidgetryPowerInhibitPlugin};

fn start(mut power: ResMut<PowerInhibit>) -> Result<(), PowerError> {
    power.enable()
}

fn finish(mut power: ResMut<PowerInhibit>) -> Result<(), PowerError> {
    power.disable()
}

#[test]
fn resource_api_works_without_window_plugins_or_updates() {
    let mut app = App::new();
    app.add_plugins(WidgetryPowerInhibitPlugin);
    for system in [start, start, finish, finish, start, finish] {
        app.world_mut()
            .run_system_once::<_, Result<(), PowerError>, _>(system)
            .unwrap()
            .unwrap();
    }
    app.world_mut()
        .run_system_once::<_, Result<(), PowerError>, _>(start)
        .unwrap()
        .unwrap();
    app.world_mut().remove_resource::<PowerInhibit>();
    app.init_resource::<PowerInhibit>();
    app.world_mut()
        .run_system_once::<_, Result<(), PowerError>, _>(start)
        .unwrap()
        .unwrap();
    drop(app);
}
