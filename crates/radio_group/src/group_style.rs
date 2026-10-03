use crate::WidgetryRadioGroup;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::{ColorTheme, ThemeChanged, ThemeMode};

type GroupStyleData = (
    Entity,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
);

fn apply(
    colors: &ColorTheme,
    focus: Option<Entity>,
    (entity, disabled, mut background, mut border): <GroupStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    background.0 = if disabled {
        colors.control_background_disabled
    } else {
        colors.control_background
    };
    *border = BorderColor::all(if disabled {
        colors.control_border_disabled
    } else if focus == Some(entity) {
        colors.control_border_active
    } else {
        colors.control_border
    });
}

pub(crate) fn update_changed(
    mode: Res<ThemeMode>,
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
    mode: Res<ThemeMode>,
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
    event: On<ThemeChanged>,
    focus: Res<InputFocus>,
    mut groups: Query<GroupStyleData, With<WidgetryRadioGroup>>,
) {
    for item in &mut groups {
        apply(event.mode.colors(), focus.get(), item);
    }
}
