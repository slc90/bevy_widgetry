use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged, WidgetryThemeMode};

use crate::WidgetryRadioGroup;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
type GroupStyleData = (
    Entity,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
);

fn apply(
    colors: &WidgetryTheme,
    focus: Option<Entity>,
    (entity, disabled, mut background, mut border): <GroupStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    background.0 = if disabled {
        colors.radio_group.container.disabled.background
    } else {
        colors.radio_group.container.normal.background
    };
    *border = BorderColor::all(if disabled {
        colors.radio_group.container.disabled.border
    } else if focus == Some(entity) {
        colors.radio_group.container.focused.border
    } else {
        colors.radio_group.container.normal.border
    });
}

pub(crate) fn update_changed(
    mode: Res<WidgetryThemeMode>,
    focus: Res<InputFocus>,
    mut groups: Query<
        GroupStyleData,
        (
            With<WidgetryRadioGroup>,
            Or<(Added<WidgetryRadioGroup>, Added<InteractionDisabled>)>,
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
