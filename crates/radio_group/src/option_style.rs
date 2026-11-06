use bevy_widgetry_theme::{WidgetryRadioGroupColors, WidgetryTheme, WidgetryThemeChanged};

use crate::error::{StyleFailureKey, StyleIssue, collect, observe_style};
use crate::{
    WidgetryRadioOption,
    option::{RadioDot, RadioIndicator},
};
use bevy::ecs::entity::EntityHashSet;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};

#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState<StyleFailureKey>);

#[derive(Resource, Default)]
pub(crate) struct StyleChanges(EntityHashSet);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<StyleChanges>()
        .add_observer(removed_style_input);
}

fn queue_option(
    entity: Entity,
    parents: &Query<&ChildOf>,
    options: &Query<(), With<WidgetryRadioOption>>,
    changes: &mut StyleChanges,
) {
    for option in std::iter::once(entity).chain(parents.iter_ancestors(entity)) {
        if options.contains(option) {
            changes.0.insert(option);
            break;
        }
    }
}

fn removed_style_input(
    event: On<
        Remove<(
            Checked,
            Hovered,
            InteractionDisabled,
            crate::colors::ColorState,
            crate::colors::OptionColorState,
            Children,
            ChildOf,
            RadioIndicator,
            RadioDot,
            BackgroundColor,
            BorderColor,
            ResolvedForeground,
            crate::WidgetryRadioGroup,
        )>,
    >,
    parents: Query<&ChildOf>,
    options: Query<(), With<WidgetryRadioOption>>,
    groups: Query<&Children, With<crate::WidgetryRadioGroup>>,
    mut changes: ResMut<StyleChanges>,
) {
    queue_option(event.entity, &parents, &options, &mut changes);
    if let Ok(children) = groups.get(event.entity) {
        changes
            .0
            .extend(children.iter().filter(|child| options.contains(*child)));
    }
}

pub(crate) fn collect_changes(
    inputs: Query<
        Entity,
        (
            With<WidgetryRadioOption>,
            Or<(
                Added<WidgetryRadioOption>,
                Changed<ChildOf>,
                Changed<Children>,
                Changed<Hovered>,
                Added<Checked>,
                Added<InteractionDisabled>,
                Changed<crate::colors::OptionColorState>,
                Added<ResolvedForeground>,
            )>,
        ),
    >,
    groups: Query<
        &Children,
        (
            With<crate::WidgetryRadioGroup>,
            Or<(
                Added<crate::WidgetryRadioGroup>,
                Changed<crate::colors::ColorState>,
            )>,
        ),
    >,
    parts: Query<
        Entity,
        (
            Or<(With<RadioIndicator>, With<RadioDot>)>,
            Or<(
                Added<RadioIndicator>,
                Added<RadioDot>,
                Changed<ChildOf>,
                Changed<Children>,
                Added<BackgroundColor>,
                Added<BorderColor>,
            )>,
        ),
    >,
    parents: Query<&ChildOf>,
    options: Query<(), With<WidgetryRadioOption>>,
    mut changes: ResMut<StyleChanges>,
) {
    changes.0.extend(inputs.iter());
    for children in &groups {
        changes
            .0
            .extend(children.iter().filter(|child| options.contains(*child)));
    }
    for entity in &parts {
        queue_option(entity, &parents, &options, &mut changes);
    }
}

#[cfg(test)]
thread_local! {
    static STYLE_WORK: std::cell::RefCell<Vec<Entity>> = const { std::cell::RefCell::new(Vec::new()) };
}

type OptionStyleData = (
    Entity,
    &'static ChildOf,
    Option<&'static crate::colors::OptionColorState>,
    Option<&'static Children>,
    Option<&'static Hovered>,
    Has<Checked>,
    Has<InteractionDisabled>,
    Option<&'static mut ResolvedForeground>,
    &'static mut StyleDiagnostics,
);

type IndicatorData = (
    Option<&'static Children>,
    Option<&'static mut BorderColor>,
    Option<&'static mut BackgroundColor>,
);
type IndicatorFilter = (With<RadioIndicator>, Without<RadioDot>);
type DotFilter = (With<RadioDot>, Without<RadioIndicator>);

fn apply(
    colors: &WidgetryRadioGroupColors,
    (option, parent, local, children, hovered, checked, disabled, foreground, mut diagnostics): <OptionStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    groups: &Query<&crate::colors::ColorState, With<crate::WidgetryRadioGroup>>,
    valid_groups: &Query<(), With<crate::WidgetryRadioGroup>>,
    indicators: &mut Query<IndicatorData, IndicatorFilter>,
    dots: &mut Query<Option<&mut BackgroundColor>, DotFilter>,
) -> Result<(), WidgetryEcsError> {
    #[cfg(test)]
    STYLE_WORK.with_borrow_mut(|work| work.push(option));
    let group = parent.parent();
    let result = (|| {
        let missing = |entity, part| StyleIssue::missing(Some(group), Some(option), entity, part);
        let query_error = |entity, part, source| {
            StyleIssue::query(Some(group), Some(option), entity, part, source)
        };
        let overrides = groups.get(group).map_err(|source| {
            query_error(
                group,
                if valid_groups.contains(group) {
                    "group.ColorState"
                } else {
                    "group.WidgetryRadioGroup"
                },
                source,
            )
        })?;
        let local = local.ok_or_else(|| missing(option, "OptionColorState"))?;
        let mut foreground = foreground.ok_or_else(|| missing(option, "ResolvedForeground"))?;
        let colors = overrides.0.resolve(colors);
        let option_colors = local.0.resolve(&colors.option);
        let option_colors = if checked {
            option_colors.checked
        } else {
            option_colors.unchecked
        };
        let colors = if disabled {
            option_colors.disabled
        } else if hovered.is_some_and(|hovered| hovered.0) {
            option_colors.hovered
        } else {
            option_colors.normal
        };
        let indicator = children
            .and_then(|children| children.iter().find(|&child| indicators.contains(child)))
            .ok_or_else(|| missing(option, "indicator"))?;
        let (children, border, background) = indicators
            .get_mut(indicator)
            .map_err(|source| query_error(indicator, "indicator", source))?;
        let children = children.ok_or_else(|| missing(indicator, "indicator.Children"))?;
        let mut border = border.ok_or_else(|| missing(indicator, "indicator.BorderColor"))?;
        let mut background =
            background.ok_or_else(|| missing(indicator, "indicator.BackgroundColor"))?;
        let dot = children
            .iter()
            .find(|&child| dots.contains(child))
            .ok_or_else(|| missing(indicator, "dot"))?;
        let mut dot_background = dots
            .get_mut(dot)
            .map_err(|source| query_error(dot, "dot", source))?
            .ok_or_else(|| missing(dot, "dot.BackgroundColor"))?;
        background.set_if_neq(BackgroundColor(colors.background));
        border.set_if_neq(BorderColor::all(colors.border));
        dot_background.set_if_neq(BackgroundColor(colors.dot));
        foreground.set_if_neq(ResolvedForeground(colors.foreground));
        Ok(())
    })();
    observe_style(&mut diagnostics.0, result, || {
        ErrorFrame::new("radio_option.style")
            .field("option", option)
            .field("group", group)
            .field("checked", checked)
            .field("disabled", disabled)
            .field("hovered", hovered.is_some_and(|hovered| hovered.0))
    })
}

pub(crate) fn update_changed(
    mode: Res<WidgetryTheme>,
    mut changes: ResMut<StyleChanges>,
    mut options: Query<OptionStyleData, With<WidgetryRadioOption>>,
    groups: Query<&crate::colors::ColorState, With<crate::WidgetryRadioGroup>>,
    valid_groups: Query<(), With<crate::WidgetryRadioGroup>>,
    mut indicators: Query<IndicatorData, IndicatorFilter>,
    mut dots: Query<Option<&mut BackgroundColor>, DotFilter>,
) -> Result {
    let mut batch = DiagnosticBatch::default();
    for option in changes.0.drain() {
        if let Ok(item) = options.get_mut(option) {
            collect(
                &mut batch,
                apply(
                    &mode.radio_group,
                    item,
                    &groups,
                    &valid_groups,
                    &mut indicators,
                    &mut dots,
                ),
            );
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme(
    _: On<WidgetryThemeChanged>,
    mode: Res<WidgetryTheme>,
    mut options: Query<OptionStyleData, With<WidgetryRadioOption>>,
    groups: Query<&crate::colors::ColorState, With<crate::WidgetryRadioGroup>>,
    valid_groups: Query<(), With<crate::WidgetryRadioGroup>>,
    mut indicators: Query<IndicatorData, IndicatorFilter>,
    mut dots: Query<Option<&mut BackgroundColor>, DotFilter>,
) -> Result {
    let mut batch = DiagnosticBatch::default();
    for item in &mut options {
        collect(
            &mut batch,
            apply(
                &mode.radio_group,
                item,
                &groups,
                &valid_groups,
                &mut indicators,
                &mut dots,
            ),
        );
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
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
            .filter(|record| {
                record.level == level
                    && record
                        .fields
                        .get("context")
                        .is_some_and(|context| context.contains("(\"option\","))
            })
            .count()
    }

    fn probe(app: &mut App, errors: &ErrorCapture, expected: usize) {
        let options = app
            .world_mut()
            .query_filtered::<Entity, With<WidgetryRadioOption>>()
            .iter(app.world())
            .collect::<Vec<_>>();
        for option in options {
            app.world_mut().entity_mut(option).insert(Hovered(false));
        }
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
    fn stable_options_skip_palette_work_and_local_input_only_refreshes_its_option() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryRadioGroupPlugin);
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let mut roots = Vec::new();
        for _ in 0..2 {
            roots.push(
                app.world_mut()
                    .spawn_scene(bsn! {
                        @crate::WidgetryRadioGroup Children [
                            @WidgetryRadioOption-- @WidgetryRadioOption-- @WidgetryRadioOption
                        ]
                    })
                    .unwrap()
                    .id(),
            );
        }
        app.update();
        app.update();
        STYLE_WORK.with_borrow_mut(Vec::clear);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(STYLE_WORK.with_borrow(Vec::len), 0);
        let option = app.world().get::<Children>(roots[0]).unwrap()[1];
        app.world_mut().entity_mut(option).insert(Hovered(true));
        app.update();
        assert_eq!(STYLE_WORK.with_borrow_mut(std::mem::take), vec![option]);
        app.world_mut().entity_mut(option).remove::<Hovered>();
        app.update();
        assert_eq!(STYLE_WORK.with_borrow_mut(std::mem::take), vec![option]);
        let colors = bevy_widgetry_theme::WIDGETRY_DARK_THEME
            .radio_group
            .option
            .unchecked
            .normal;
        assert_eq!(
            app.world().get::<ResolvedForeground>(option).unwrap().0,
            colors.foreground
        );
    }

    #[test]
    fn missing_parent_palette_keeps_the_actual_query_source_for_every_option() {
        use crate::RadioGroupError;
        use bevy_widgetry_test_utils::DiagnosticPhase;
        use std::error::Error;
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryRadioGroupPlugin)
            .add_systems(StyleProbe, (collect_changes, update_changed).chain());
        app.edit_schedule(StyleProbe, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let root = app.world_mut().spawn_scene(bsn! { @crate::WidgetryRadioGroup Children [@WidgetryRadioOption-- @WidgetryRadioOption] }).unwrap().id();
        app.update();
        let options = app.world().get::<Children>(root).unwrap().to_vec();
        app.world_mut()
            .entity_mut(root)
            .remove::<crate::colors::ColorState>();
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| errors.run(|| {
            app.world_mut().run_schedule(StyleProbe);
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            let diagnostics: Vec<_> = records[0].managed().unwrap().diagnostics().collect();
            assert_eq!(diagnostics.len(), 2);
            let ids: Vec<_> = diagnostics.iter().map(|diagnostic| diagnostic.origin().id().clone()).collect();
            for diagnostic in diagnostics {
                let kind = diagnostic.source().unwrap().downcast_ref::<RadioGroupError>().unwrap();
                assert!(matches!(kind, RadioGroupError::Query { group: Some(group), option: Some(option), entity, part: "group.ColorState", .. } if *group == root && *entity == root && options.contains(option)));
                assert!(kind.source().unwrap().is::<bevy::ecs::query::QueryEntityError>());
                assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
                assert!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled).is_empty());
            }
            app.world_mut().entity_mut(root).insert(crate::colors::ColorState::default());
            app.world_mut().entity_mut(root).remove::<crate::WidgetryRadioGroup>();
            app.world_mut().run_schedule(StyleProbe);
            let changed = errors.take_records();
            assert_eq!(changed.len(), 1);
            let diagnostics: Vec<_> = changed[0].managed().unwrap().diagnostics().collect();
            assert_eq!(diagnostics.len(), 2);
            for diagnostic in diagnostics {
                assert!(matches!(diagnostic.source().unwrap().downcast_ref::<RadioGroupError>(), Some(RadioGroupError::Query { part: "group.WidgetryRadioGroup", .. })));
                assert!(!ids.contains(diagnostic.origin().id()));
                assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
            }
            app.world_mut().entity_mut(root).insert(crate::WidgetryRadioGroup);
            app.world_mut().run_schedule(StyleProbe);
            assert!(errors.take_records().is_empty());
            assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
        }));
    }

    #[test]
    fn palette_and_output_repairs_recheck_only_affected_options_without_polling() {
        let logs = LogCapture::default();
        logs.run(|| {
            let mut app = scene_app();
            app.add_plugins(crate::WidgetryRadioGroupPlugin);
            app.edit_schedule(PostUpdate, |schedule| {
                schedule.set_executor(SingleThreadedExecutor::new());
            });
            let mut roots = Vec::new();
            for _ in 0..2 {
                roots.push(
                    app.world_mut()
                        .spawn_scene(bsn! {
                            @crate::WidgetryRadioGroup Children [
                                @WidgetryRadioOption-- @WidgetryRadioOption
                            ]
                        })
                        .unwrap()
                        .id(),
                );
            }
            app.update();
            app.update();
            app.set_error_handler(ErrorCapture::handler());
            let errors = ErrorCapture::default();
            let options = app.world().get::<Children>(roots[0]).unwrap().to_vec();
            let indicator = app.world().get::<Children>(options[0]).unwrap()[0];
            let dot = app.world().get::<Children>(indicator).unwrap()[0];
            STYLE_WORK.with_borrow_mut(Vec::clear);
            app.world_mut()
                .entity_mut(roots[0])
                .remove::<crate::colors::ColorState>();
            errors.run(|| app.update());
            assert_eq!(errors.take().len(), 1);
            let work = STYLE_WORK.with_borrow_mut(std::mem::take);
            assert_eq!(work.len(), 2);
            assert!(options.iter().all(|option| work.contains(option)));
            assert_eq!(option_logs(&logs, bevy::log::Level::ERROR), 2);
            errors.run(|| app.update());
            assert!(errors.take().is_empty());
            assert_eq!(STYLE_WORK.with_borrow(Vec::len), 0);
            app.world_mut()
                .entity_mut(roots[0])
                .insert(crate::colors::ColorState::default());
            errors.run(|| app.update());
            assert!(errors.take().is_empty());
            assert_eq!(STYLE_WORK.with_borrow_mut(std::mem::take).len(), 2);
            assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 2);
            let background = *app.world().get::<BackgroundColor>(dot).unwrap();
            for round in 0..2 {
                app.world_mut().entity_mut(dot).remove::<BackgroundColor>();
                errors.run(|| app.update());
                let failures = errors.take();
                assert_eq!(failures.len(), 1);
                assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
                assert_eq!(STYLE_WORK.with_borrow_mut(std::mem::take), vec![options[0]]);
                assert_eq!(option_logs(&logs, bevy::log::Level::ERROR), 3 + round);
                errors.run(|| app.update());
                assert!(errors.take().is_empty());
                assert_eq!(STYLE_WORK.with_borrow(Vec::len), 0);
                app.world_mut().entity_mut(dot).insert(background);
                errors.run(|| app.update());
                assert!(errors.take().is_empty());
                assert_eq!(STYLE_WORK.with_borrow_mut(std::mem::take), vec![options[0]]);
                assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 3 + round);
            }
            app.world_mut()
                .entity_mut(indicator)
                .remove::<RadioIndicator>();
            errors.run(|| app.update());
            assert_eq!(errors.take().len(), 1);
            STYLE_WORK.with_borrow_mut(Vec::clear);
            app.world_mut().entity_mut(indicator).insert(RadioIndicator);
            errors.run(|| app.update());
            assert!(errors.take().is_empty());
            assert_eq!(STYLE_WORK.with_borrow_mut(std::mem::take), vec![options[0]]);
            assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 5);
            app.world_mut().despawn(indicator);
            errors.run(|| app.update());
            assert_eq!(errors.take().len(), 1);
            errors.run(|| app.update());
            assert!(errors.take().is_empty());
            let replacement = app
                .world_mut()
                .spawn_scene(bsn! {
                    template(|_| Ok(RadioIndicator))
                    Node BorderColor BackgroundColor Children [
                        template(|_| Ok(RadioDot)) Node BackgroundColor
                    ]
                })
                .unwrap()
                .id();
            errors.run(|| app.update());
            assert!(errors.take().is_empty());
            STYLE_WORK.with_borrow_mut(Vec::clear);
            app.world_mut()
                .entity_mut(options[0])
                .add_child(replacement);
            errors.run(|| app.update());
            assert!(errors.take().is_empty());
            assert_eq!(STYLE_WORK.with_borrow_mut(std::mem::take), vec![options[0]]);
            let dot = app.world().get::<Children>(replacement).unwrap()[0];
            assert_eq!(
                app.world().get::<BackgroundColor>(dot).unwrap().0,
                bevy_widgetry_theme::WIDGETRY_DARK_THEME
                    .radio_group
                    .option
                    .checked
                    .normal
                    .dot
            );
            assert_eq!(option_logs(&logs, bevy::log::Level::INFO), 6);
        });
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
                .add_systems(StyleProbe, (collect_changes, update_changed).chain());
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
            errors.run(|| switch_theme(&mut app, bevy_widgetry_theme::WIDGETRY_LIGHT_THEME));
            let failures = errors.take_records();
            assert_eq!(failures.len(), 2);
            assert!(
                failures
                    .iter()
                    .all(|record| record.error.severity() == bevy::ecs::error::Severity::Error)
            );
            assert_eq!(
                failures
                    .iter()
                    .map(|record| record.managed().unwrap().diagnostics().count())
                    .sum::<usize>(),
                3
            );
            assert_eq!(option_logs(&logs, bevy::log::Level::ERROR), 2);
            assert_eq!(
                app.world()
                    .get::<ResolvedForeground>(healthy_option)
                    .unwrap()
                    .0,
                bevy_widgetry_theme::WIDGETRY_LIGHT_THEME
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
            probe(&mut app, &errors, 1);
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
                record.level == bevy::log::Level::ERROR
                    && record
                        .fields
                        .get("context")
                        .is_some_and(|context| context.contains("(\"option\","))
            }) {
                assert!(
                    record
                        .fields
                        .get("context")
                        .unwrap()
                        .contains(&root.to_string())
                );
            }
        });
    }
}
