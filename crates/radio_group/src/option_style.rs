use bevy_widgetry_theme::{WidgetryRadioGroupColors, WidgetryThemeChanged, WidgetryThemeMode};

use crate::{
    WidgetryRadioOption,
    option::{RadioDot, RadioIndicator},
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState);

type OptionStyleData = (
    Entity,
    &'static ChildOf,
    &'static crate::colors::OptionColorState,
    &'static Children,
    &'static Hovered,
    Has<Checked>,
    Has<InteractionDisabled>,
    &'static mut ResolvedForeground,
    &'static mut StyleDiagnostics,
);

fn apply(
    colors: &WidgetryRadioGroupColors,
    (option, parent, local, children, hovered, checked, disabled, mut foreground, mut diagnostics): <OptionStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    groups: &Query<&crate::colors::ColorState, With<crate::WidgetryRadioGroup>>,
    indicators: &mut Query<
        (&Children, &mut BorderColor, &mut BackgroundColor),
        (With<RadioIndicator>, Without<RadioDot>),
    >,
    dots: &mut Query<&mut BackgroundColor, (With<RadioDot>, Without<RadioIndicator>)>,
) -> Result<(), BevyError> {
    let overrides = groups.get(parent.parent()).map_err(|_| {
        widgetry_error!(?option, "RadioOption 缺失 group 颜色配置");
        BevyError::error("RadioOption 缺失 group 颜色配置")
    })?;
    let colors = overrides.0.resolve(colors);
    let option_colors = local.0.resolve(&colors.option);
    let option_colors = if checked {
        option_colors.checked
    } else {
        option_colors.unchecked
    };
    let colors = if disabled {
        option_colors.disabled
    } else if hovered.0 {
        option_colors.hovered
    } else {
        option_colors.normal
    };
    let result = (|| -> Result<(), BevyError> {
        let Some(indicator) = children.iter().find(|&child| indicators.contains(child)) else {
            return Err(BevyError::error("RadioOption missing indicator"));
        };
        let Ok((children, mut border, mut background)) = indicators.get_mut(indicator) else {
            return Err(BevyError::error("RadioOption indicator missing style"));
        };
        let Some(dot) = children.iter().find(|&child| dots.contains(child)) else {
            return Err(BevyError::error("RadioOption indicator missing dot"));
        };
        let Ok(mut dot_background) = dots.get_mut(dot) else {
            return Err(BevyError::error("RadioOption dot missing background"));
        };
        background.set_if_neq(BackgroundColor(colors.background));
        border.set_if_neq(BorderColor::all(colors.border));
        dot_background.set_if_neq(BackgroundColor(colors.dot));
        foreground.set_if_neq(ResolvedForeground(colors.foreground));
        Ok(())
    })();
    diagnostics.0.observe(
        result,
        |error| widgetry_error!(?option, %error, "RadioOption style 内部结构失效"),
        || widgetry_info!(?option, "RadioOption style 恢复正常"),
    )
}

pub(crate) fn update_changed(
    mode: Res<WidgetryThemeMode>,
    mut options: Query<OptionStyleData, (With<WidgetryRadioOption>,)>,
    groups: Query<&crate::colors::ColorState, With<crate::WidgetryRadioGroup>>,
    mut indicators: Query<
        (&Children, &mut BorderColor, &mut BackgroundColor),
        (With<RadioIndicator>, Without<RadioDot>),
    >,
    mut dots: Query<&mut BackgroundColor, (With<RadioDot>, Without<RadioIndicator>)>,
) -> Result<(), BevyError> {
    let mut failure = None;
    for item in &mut options {
        if let Err(error) = apply(
            &mode.colors().radio_group,
            item,
            &groups,
            &mut indicators,
            &mut dots,
        ) && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn update_removed(
    mode: Res<WidgetryThemeMode>,
    mut checked: RemovedComponents<Checked>,
    mut disabled: RemovedComponents<InteractionDisabled>,
    mut options: Query<OptionStyleData, With<WidgetryRadioOption>>,
    groups: Query<&crate::colors::ColorState, With<crate::WidgetryRadioGroup>>,
    mut indicators: Query<
        (&Children, &mut BorderColor, &mut BackgroundColor),
        (With<RadioIndicator>, Without<RadioDot>),
    >,
    mut dots: Query<&mut BackgroundColor, (With<RadioDot>, Without<RadioIndicator>)>,
) -> Result<(), BevyError> {
    let mut failure = None;
    for entity in checked.read().chain(disabled.read()) {
        if let Ok(item) = options.get_mut(entity)
            && let Err(error) = apply(
                &mode.colors().radio_group,
                item,
                &groups,
                &mut indicators,
                &mut dots,
            )
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme(
    event: On<WidgetryThemeChanged>,
    mut options: Query<OptionStyleData, With<WidgetryRadioOption>>,
    groups: Query<&crate::colors::ColorState, With<crate::WidgetryRadioGroup>>,
    mut indicators: Query<
        (&Children, &mut BorderColor, &mut BackgroundColor),
        (With<RadioIndicator>, Without<RadioDot>),
    >,
    mut dots: Query<&mut BackgroundColor, (With<RadioDot>, Without<RadioIndicator>)>,
) -> Result<(), BevyError> {
    let mut failure = None;
    for item in &mut options {
        if let Err(error) = apply(
            &event.mode.colors().radio_group,
            item,
            &groups,
            &mut indicators,
            &mut dots,
        ) && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}
