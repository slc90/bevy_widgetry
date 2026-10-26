#![cfg(feature = "power_inhibit")]
#![allow(clippy::disallowed_macros)]

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_widgetry::power_inhibit::{PowerError, PowerInhibit, WidgetryPowerInhibitPlugin};

#[test]
fn facade_feature_supports_the_resource_system_api() {
    let mut app = App::new();
    app.add_plugins(WidgetryPowerInhibitPlugin);
    app.world_mut()
        .run_system_once::<_, Result<(), PowerError>, _>(
            |mut power: ResMut<PowerInhibit>| -> Result<(), PowerError> {
                power.enable()?;
                power.disable()
            },
        )
        .unwrap()
        .unwrap();
}
