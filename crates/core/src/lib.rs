//! Shared infrastructure for Widgetry controls.

use bevy::{
    app::{App, HierarchyPropagatePlugin, Plugin, PostUpdate, PropagateSet},
    color::Color,
    ecs::{component::Component, query::Changed, schedule::IntoScheduleConfigs, system::Query},
    text::TextColor,
    ui::UiSystems,
};

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ForegroundColor(pub Color);

impl Default for ForegroundColor {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

fn apply_foreground_color_to_text(
    mut query: Query<(&ForegroundColor, &mut TextColor), Changed<ForegroundColor>>,
) {
    for (foreground, mut text_color) in &mut query {
        text_color.0 = foreground.0;
    }
}

/// Propagates foreground colors through the hierarchy and applies them to text.
pub struct ForegroundColorPlugin;

impl Plugin for ForegroundColorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HierarchyPropagatePlugin::<ForegroundColor>::new(PostUpdate));
        app.configure_sets(
            PostUpdate,
            PropagateSet::<ForegroundColor>::default().in_set(UiSystems::Propagate),
        );
        app.add_systems(
            PostUpdate,
            apply_foreground_color_to_text
                .in_set(UiSystems::Propagate)
                .after(PropagateSet::<ForegroundColor>::default()),
        );
    }
}
