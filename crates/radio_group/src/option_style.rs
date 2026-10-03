use crate::{
    WidgetryRadioOption,
    option::{RadioDot, RadioIndicator},
};
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::{ColorTheme, ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState);

type OptionStyleData = (
    Entity,
    &'static Children,
    &'static Hovered,
    Has<Checked>,
    Has<InteractionDisabled>,
    &'static mut Propagate<ForegroundColor>,
    &'static mut StyleDiagnostics,
);

fn apply(
    colors: &ColorTheme,
    (option, children, hovered, checked, disabled, mut foreground, mut diagnostics): <OptionStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    indicators: &mut Query<(&Children, &mut BorderColor), With<RadioIndicator>>,
    dots: &mut Query<&mut BackgroundColor, With<RadioDot>>,
) -> Result<(), BevyError> {
    let result = (|| -> Result<(), BevyError> {
        let Some(indicator) = children.iter().find(|&child| indicators.contains(child)) else {
            return Err(BevyError::error("RadioOption missing indicator"));
        };
        let Ok((children, mut border)) = indicators.get_mut(indicator) else {
            return Err(BevyError::error("RadioOption indicator missing style"));
        };
        let Some(dot) = children.iter().find(|&child| dots.contains(child)) else {
            return Err(BevyError::error("RadioOption indicator missing dot"));
        };
        let Ok(mut dot_background) = dots.get_mut(dot) else {
            return Err(BevyError::error("RadioOption dot missing background"));
        };
        *border = BorderColor::all(if disabled {
            colors.control_border_disabled
        } else if hovered.0 {
            colors.control_border_hovered
        } else if checked {
            colors.control_border_active
        } else {
            colors.control_border
        });
        dot_background.0 = if !checked {
            Color::NONE
        } else if disabled {
            colors.foreground_disabled
        } else {
            colors.control_border_active
        };
        foreground.0 = ForegroundColor(if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        });
        Ok(())
    })();
    diagnostics.0.observe(
        result,
        |error| widgetry_error!(?option, %error, "RadioOption style 内部结构失效"),
        || widgetry_info!(?option, "RadioOption style 恢复正常"),
    )
}

pub(crate) fn update_changed(
    mode: Res<ThemeMode>,
    mut options: Query<
        OptionStyleData,
        (
            With<WidgetryRadioOption>,
            Or<(
                Added<WidgetryRadioOption>,
                Changed<Hovered>,
                Added<Checked>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    mut indicators: Query<(&Children, &mut BorderColor), With<RadioIndicator>>,
    mut dots: Query<&mut BackgroundColor, With<RadioDot>>,
) -> Result<(), BevyError> {
    let mut failure = None;
    for item in &mut options {
        if let Err(error) = apply(mode.colors(), item, &mut indicators, &mut dots)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn update_removed(
    mode: Res<ThemeMode>,
    mut checked: RemovedComponents<Checked>,
    mut disabled: RemovedComponents<InteractionDisabled>,
    mut options: Query<OptionStyleData, With<WidgetryRadioOption>>,
    mut indicators: Query<(&Children, &mut BorderColor), With<RadioIndicator>>,
    mut dots: Query<&mut BackgroundColor, With<RadioDot>>,
) -> Result<(), BevyError> {
    let mut failure = None;
    for entity in checked.read().chain(disabled.read()) {
        if let Ok(item) = options.get_mut(entity)
            && let Err(error) = apply(mode.colors(), item, &mut indicators, &mut dots)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme(
    event: On<ThemeChanged>,
    mut options: Query<OptionStyleData, With<WidgetryRadioOption>>,
    mut indicators: Query<(&Children, &mut BorderColor), With<RadioIndicator>>,
    mut dots: Query<&mut BackgroundColor, With<RadioDot>>,
) -> Result<(), BevyError> {
    let mut failure = None;
    for item in &mut options {
        if let Err(error) = apply(event.mode.colors(), item, &mut indicators, &mut dots)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}
