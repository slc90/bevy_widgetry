use crate::{
    WidgetryRadioOption,
    option::{RadioDot, RadioIndicator},
};
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy_widgetry_core::{ColorTheme, ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry_log::widgetry_error;

/// Option 的 state、内建结构入口和 foreground 输出。
type OptionStyleData = (
    Entity,
    &'static Children,
    &'static Hovered,
    Has<Checked>,
    Has<InteractionDisabled>,
    &'static mut Propagate<ForegroundColor>,
);

/// 只在需要刷新时查找 private marker，避免改写用户内容中的 border 或 background。
fn apply(
    colors: &ColorTheme,
    (option, children, hovered, checked, disabled, mut foreground): <OptionStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    indicators: &mut Query<(&Children, &mut BorderColor), With<RadioIndicator>>,
    dots: &mut Query<&mut BackgroundColor, With<RadioDot>>,
) {
    let Some(indicator) = children.iter().find(|&child| indicators.contains(child)) else {
        widgetry_error!(?option, "RadioOption 缺少内建 indicator");
        return;
    };
    let Ok((children, mut border)) = indicators.get_mut(indicator) else {
        widgetry_error!(?option, ?indicator, "RadioOption indicator 缺少 style 结构");
        return;
    };
    let Some(dot) = children.iter().find(|&child| dots.contains(child)) else {
        widgetry_error!(?option, ?indicator, "RadioOption indicator 缺少内建 dot");
        return;
    };
    let Ok(mut dot_background) = dots.get_mut(dot) else {
        widgetry_error!(?option, ?dot, "RadioOption dot 缺少 background");
        return;
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
}

/// 新增 Option、hover、checked 或 disabled 时重新解析完整配色。
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
) {
    for item in &mut options {
        apply(mode.colors(), item, &mut indicators, &mut dots);
    }
}

/// Checked 或 disabled 移除后清除旧配色，保留剩余 hover state。
pub(crate) fn update_removed(
    mode: Res<ThemeMode>,
    mut checked: RemovedComponents<Checked>,
    mut disabled: RemovedComponents<InteractionDisabled>,
    mut options: Query<OptionStyleData, With<WidgetryRadioOption>>,
    mut indicators: Query<(&Children, &mut BorderColor), With<RadioIndicator>>,
    mut dots: Query<&mut BackgroundColor, With<RadioDot>>,
) {
    for entity in checked.read().chain(disabled.read()) {
        if let Ok(item) = options.get_mut(entity) {
            apply(mode.colors(), item, &mut indicators, &mut dots);
        }
    }
}

/// ThemeChanged 立即刷新内建 indicator 和 foreground，不等待下一次交互。
pub(crate) fn refresh_theme(
    event: On<ThemeChanged>,
    mut options: Query<OptionStyleData, With<WidgetryRadioOption>>,
    mut indicators: Query<(&Children, &mut BorderColor), With<RadioIndicator>>,
    mut dots: Query<&mut BackgroundColor, With<RadioDot>>,
) {
    for item in &mut options {
        apply(event.mode.colors(), item, &mut indicators, &mut dots);
    }
}
