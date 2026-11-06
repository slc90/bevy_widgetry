use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged};

use crate::WidgetryRadioGroup;
use crate::error::{StyleFailureKey, StyleIssue, collect, observe_style};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState<StyleFailureKey>);
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
type GroupStyleData = (
    Entity,
    Option<&'static crate::colors::ColorState>,
    Has<InteractionDisabled>,
    Option<&'static mut bevy_widgetry_core::foreground::ResolvedForeground>,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
    &'static mut StyleDiagnostics,
);

fn apply(
    colors: &WidgetryTheme,
    focus: Option<Entity>,
    (entity, overrides, disabled, foreground, background, border, mut diagnostics): <GroupStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) -> Result<(), WidgetryEcsError> {
    let result = (|| {
        let missing = |part| StyleIssue::missing(Some(entity), None, entity, part);
        let overrides = overrides.ok_or_else(|| missing("ColorState"))?;
        let mut foreground = foreground.ok_or_else(|| missing("ResolvedForeground"))?;
        let mut background = background.ok_or_else(|| missing("BackgroundColor"))?;
        let mut border = border.ok_or_else(|| missing("BorderColor"))?;
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
        Ok(())
    })();
    observe_style(&mut diagnostics.0, result, || {
        ErrorFrame::new("radio_group.style")
            .field("group", entity)
            .field("disabled", disabled)
            .field("focused", focus == Some(entity))
    })
}

pub(crate) fn update_changed(
    mode: Res<WidgetryTheme>,
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
) -> Result {
    let mut batch = DiagnosticBatch::default();
    for item in &mut groups {
        collect(&mut batch, apply(&mode, focus.get(), item));
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

pub(crate) fn update_focus_and_removed(
    mode: Res<WidgetryTheme>,
    focus: Res<InputFocus>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mut groups: Query<GroupStyleData, With<WidgetryRadioGroup>>,
) -> Result {
    let mut batch = DiagnosticBatch::default();
    if focus.is_changed() {
        for item in &mut groups {
            collect(&mut batch, apply(&mode, focus.get(), item));
        }
        removed.clear();
    } else {
        for entity in removed.read() {
            if let Ok(item) = groups.get_mut(entity) {
                collect(&mut batch, apply(&mode, focus.get(), item));
            }
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme(
    _: On<WidgetryThemeChanged>,
    mode: Res<WidgetryTheme>,
    focus: Res<InputFocus>,
    mut groups: Query<GroupStyleData, With<WidgetryRadioGroup>>,
) -> Result {
    let mut batch = DiagnosticBatch::default();
    for item in &mut groups {
        collect(&mut batch, apply(&mode, focus.get(), item));
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}
