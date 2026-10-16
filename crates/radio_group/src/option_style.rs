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
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
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
    let group = parent.parent();
    let result = (|| -> Result<(), BevyError> {
        let overrides = groups
            .get(group)
            .map_err(|_| BevyError::error("RadioOption 缺失 group 颜色配置"))?;
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
        |error| widgetry_error!(?option, ?group, %error, "RadioOption style 内部结构失效"),
        || widgetry_info!(?option, ?group, "RadioOption style 恢复正常"),
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

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::{ScheduleLabel, SingleThreadedExecutor};
    use bevy::ui_widgets::ValueChange;
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app, switch_theme};

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct StyleProbe;

    #[derive(Resource, Default)]
    struct Notifications(usize);

    fn option_logs(logs: &LogCapture, level: bevy::log::Level) -> usize {
        logs.records()
            .iter()
            .filter(|record| record.level == level && record.fields.contains_key("option"))
            .count()
    }

    fn probe(app: &mut App, errors: &ErrorCapture, expected: usize) {
        errors.run(|| app.world_mut().run_schedule(StyleProbe));
        let failures = errors.take();
        assert_eq!(failures.len(), expected);
        assert!(
            failures
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
    }

    #[test]
    fn parent_palette_failures_share_option_diagnostics_across_style_entries() {
        let logs = LogCapture::default();
        logs.run(|| {
            let mut app = scene_app();
            app.add_plugins(crate::WidgetryRadioGroupPlugin)
                .init_resource::<Notifications>()
                .add_observer(
                    |_: On<ValueChange<usize>>, mut notifications: ResMut<Notifications>| {
                        notifications.0 += 1;
                    },
                )
                .add_systems(StyleProbe, (update_changed, update_removed).chain());
            app.edit_schedule(StyleProbe, |schedule| {
                schedule.set_executor(SingleThreadedExecutor::new());
            });
            let root = app
                .world_mut()
                .spawn_scene(bsn! {
                    @crate::WidgetryRadioGroup Children [
                        @WidgetryRadioOption-- @WidgetryRadioOption
                    ]
                })
                .unwrap()
                .id();
            let healthy = app
                .world_mut()
                .spawn_scene(bsn! {
                    @crate::WidgetryRadioGroup Children [@WidgetryRadioOption]
                })
                .unwrap()
                .id();
            app.update();
            let options = app.world().get::<Children>(root).unwrap().to_vec();
            let healthy_option = app.world().get::<Children>(healthy).unwrap()[0];
            app.set_error_handler(ErrorCapture::handler());
            let errors = ErrorCapture::default();
            probe(&mut app, &errors, 0);
            assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 0);

            app.world_mut()
                .entity_mut(root)
                .remove::<crate::colors::ColorState>();
            for _ in 0..3 {
                probe(&mut app, &errors, 1);
            }
            assert_eq!(option_logs(&logs, bevy::log::Level::ERROR), 2);
            errors.run(|| switch_theme(&mut app, WidgetryThemeMode::Light));
            let failures = errors.take();
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
            assert_eq!(option_logs(&logs, bevy::log::Level::ERROR), 2);
            assert_eq!(
                app.world()
                    .get::<ResolvedForeground>(healthy_option)
                    .unwrap()
                    .0,
                WidgetryThemeMode::Light
                    .colors()
                    .radio_group
                    .option
                    .checked
                    .normal
                    .foreground
            );

            app.world_mut()
                .entity_mut(options[0])
                .insert(InteractionDisabled);
            app.world_mut()
                .entity_mut(options[0])
                .remove::<InteractionDisabled>();
            probe(&mut app, &errors, 2);
            assert_eq!(option_logs(&logs, bevy::log::Level::ERROR), 2);

            let indicators = options
                .iter()
                .map(|option| app.world().get::<Children>(*option).unwrap()[0])
                .collect::<Vec<_>>();
            for indicator in &indicators {
                app.world_mut()
                    .entity_mut(*indicator)
                    .remove::<RadioIndicator>();
            }
            app.world_mut()
                .entity_mut(root)
                .insert(crate::colors::ColorState::default());
            for _ in 0..2 {
                probe(&mut app, &errors, 1);
            }
            assert_eq!(option_logs(&logs, bevy::log::Level::ERROR), 4);
            assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 0);
            app.world_mut()
                .entity_mut(indicators[0])
                .insert(RadioIndicator);
            probe(&mut app, &errors, 1);
            assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 1);
            app.world_mut()
                .entity_mut(indicators[1])
                .insert(RadioIndicator);
            probe(&mut app, &errors, 0);
            probe(&mut app, &errors, 0);
            assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 2);

            app.world_mut()
                .entity_mut(root)
                .remove::<crate::colors::ColorState>();
            for _ in 0..2 {
                probe(&mut app, &errors, 1);
            }
            assert_eq!(option_logs(&logs, bevy::log::Level::ERROR), 6);
            app.world_mut()
                .entity_mut(root)
                .insert(crate::colors::ColorState::default());
            probe(&mut app, &errors, 0);
            assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 4);
            for (index, option) in options.iter().enumerate() {
                assert_eq!(app.world().get::<Checked>(*option).is_some(), index == 0);
            }
            assert!(app.world().get::<Checked>(healthy_option).is_some());
            assert_eq!(app.world().resource::<Notifications>().0, 0);
            for record in logs.records().iter().filter(|record| {
                record.level == bevy::log::Level::ERROR && record.fields.contains_key("option")
            }) {
                assert_eq!(record.fields.get("group"), Some(&format!("{root:?}")));
            }
        });
    }
}
