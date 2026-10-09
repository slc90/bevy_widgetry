use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged, WidgetryThemeMode};

use crate::WidgetryRadioGroup;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
type GroupStyleData = (
    Entity,
    &'static crate::colors::ColorState,
    Has<InteractionDisabled>,
    &'static mut bevy_widgetry_core::foreground::ResolvedForeground,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
);

fn apply(
    colors: &WidgetryTheme,
    focus: Option<Entity>,
    (entity, overrides, disabled, mut foreground, mut background, mut border): <GroupStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let colors = overrides.0.resolve(&colors.radio_group).container;
    let state = if disabled {
        colors.disabled
    } else if focus == Some(entity) {
        colors.focused
    } else {
        colors.normal
    };
    background.set_if_neq(BackgroundColor(state.background));
    border.set_if_neq(BorderColor::all(state.border));
    foreground.set_if_neq(bevy_widgetry_core::foreground::ResolvedForeground(
        state.foreground,
    ));
}

pub(crate) fn update_changed(
    mode: Res<WidgetryThemeMode>,
    focus: Res<InputFocus>,
    mut groups: Query<
        GroupStyleData,
        (
            With<WidgetryRadioGroup>,
            Or<(
                Added<WidgetryRadioGroup>,
                Added<InteractionDisabled>,
                Changed<crate::colors::ColorState>,
            )>,
        ),
    >,
) {
    for item in &mut groups {
        apply(mode.colors(), focus.get(), item);
    }
}

pub(crate) fn update_focus_and_removed(
    mode: Res<WidgetryThemeMode>,
    focus: Res<InputFocus>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mut groups: Query<GroupStyleData, With<WidgetryRadioGroup>>,
) {
    if focus.is_changed() {
        for item in &mut groups {
            apply(mode.colors(), focus.get(), item);
        }
        removed.clear();
    } else {
        for entity in removed.read() {
            if let Ok(item) = groups.get_mut(entity) {
                apply(mode.colors(), focus.get(), item);
            }
        }
    }
}

pub(crate) fn refresh_theme(
    event: On<WidgetryThemeChanged>,
    focus: Res<InputFocus>,
    mut groups: Query<GroupStyleData, With<WidgetryRadioGroup>>,
) {
    for item in &mut groups {
        apply(event.mode.colors(), focus.get(), item);
    }
}
