use bevy::app::App;
use bevy_widgetry_core::{ThemeChanged, ThemeMode};

pub fn switch_theme(app: &mut App, mode: ThemeMode) {
    *app.world_mut().resource_mut::<ThemeMode>() = mode;
    app.world_mut().trigger(ThemeChanged { mode });
}
