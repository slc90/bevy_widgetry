use crate::checkbox::WidgetryCheckBox;
use crate::indicator::{CheckBoxIndicator, CheckBoxMark};
use crate::tri_state::{WidgetryCheckState, WidgetryTriStateCheckbox};
use crate::{CheckBoxError, CheckBoxFailure};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BorderColor, Checked, InteractionDisabled, Pressed};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged};
use std::sync::Arc;

#[derive(PartialEq, Eq)]
enum FailureKey {
    Missing(Entity, &'static str),
    Query(Entity, &'static str),
}
#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics {
    mark: FailureState<FailureKey>,
    colors: FailureState<FailureKey>,
}

type RootStyleData = (
    Entity,
    Option<&'static crate::colors::ColorState>,
    Option<&'static Hovered>,
    Has<Pressed>,
    Has<InteractionDisabled>,
    Has<Checked>,
    Option<&'static WidgetryCheckState>,
    Option<&'static Children>,
    Option<&'static mut ResolvedForeground>,
    &'static mut StyleDiagnostics,
);
type RootFilter = Or<(With<WidgetryCheckBox>, With<WidgetryTriStateCheckbox>)>;
type OwnedFilter = (
    RootFilter,
    Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryCheckBox>>,
);
type IndicatorData = (
    Option<&'static Children>,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
);
type MarkData = (
    Option<&'static mut CheckBoxMark>,
    Has<WidgetryIcon>,
    Option<&'static mut Visibility>,
);
type MarkFilter = Or<(With<CheckBoxMark>, With<WidgetryIcon>)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckBoxVisualState {
    Unchecked,
    Checked,
    Indeterminate,
}
struct CheckBoxStyle {
    background: Color,
    border: Color,
    foreground: Color,
    mark: Color,
}
fn visual_state(checked: bool, tri_state: Option<&WidgetryCheckState>) -> CheckBoxVisualState {
    match tri_state.copied() {
        Some(WidgetryCheckState::Indeterminate) => CheckBoxVisualState::Indeterminate,
        Some(WidgetryCheckState::Checked) => CheckBoxVisualState::Checked,
        Some(WidgetryCheckState::Unchecked) => CheckBoxVisualState::Unchecked,
        None if checked => CheckBoxVisualState::Checked,
        None => CheckBoxVisualState::Unchecked,
    }
}
struct StructureIssue {
    root: Entity,
    entity: Entity,
    part: &'static str,
    source: Option<bevy::ecs::query::QueryEntityError>,
}
impl StructureIssue {
    fn into_kind(self) -> CheckBoxError {
        match self.source {
            Some(source) => CheckBoxError::Query {
                root: self.root,
                entity: self.entity,
                part: self.part,
                source,
            },
            None => CheckBoxError::MissingPart {
                root: self.root,
                entity: self.entity,
                part: self.part,
            },
        }
    }
}
fn missing(root: Entity, entity: Entity, part: &'static str) -> StructureIssue {
    StructureIssue {
        root,
        entity,
        part,
        source: None,
    }
}
fn query_error(
    root: Entity,
    entity: Entity,
    part: &'static str,
    source: bevy::ecs::query::QueryEntityError,
) -> StructureIssue {
    StructureIssue {
        root,
        entity,
        part,
        source: Some(source),
    }
}
fn observe(
    state: &mut FailureState<FailureKey>,
    result: Result<(), StructureIssue>,
    frame: ErrorFrame,
) -> Result<(), WidgetryEcsError> {
    match result {
        Ok(()) => {
            state.recover(|| frame);
            Ok(())
        }
        Err(issue) => {
            let key = if issue.source.is_some() {
                FailureKey::Query(issue.entity, issue.part)
            } else {
                FailureKey::Missing(issue.entity, issue.part)
            };
            let frame = frame
                .field("entity", issue.entity)
                .field("part", issue.part);
            Err(state.failure(key, || {
                Arc::new(CheckBoxFailure::new(issue.into_kind(), frame))
            }))
        }
    }
}
fn push(batch: &mut DiagnosticBatch, result: Result<(), WidgetryEcsError>) {
    if let Err(error) = result {
        for diagnostic in error.shared_diagnostics() {
            batch.push_shared(diagnostic);
        }
    }
}

pub(crate) fn update_mark_geometry(
    changed: Query<
        Entity,
        Or<(
            Added<WidgetryCheckBox>,
            Added<WidgetryTriStateCheckbox>,
            Added<Checked>,
            Changed<WidgetryCheckState>,
        )>,
    >,
    mut removed: RemovedComponents<Checked>,
    mut roots: Query<
        (
            Entity,
            Option<&Children>,
            Has<Checked>,
            Option<&WidgetryCheckState>,
            &mut StyleDiagnostics,
        ),
        RootFilter,
    >,
    indicators: Query<Option<&Children>, With<CheckBoxIndicator>>,
    mut marks: Query<MarkData, MarkFilter>,
    mut commands: Commands,
) -> Result {
    let removed: bevy::ecs::entity::EntityHashSet = removed.read().collect();
    let mut batch = DiagnosticBatch::default();
    for (root, children, checked, tri_state, mut diagnostics) in &mut roots {
        if !changed.contains(root)
            && !removed.contains(&root)
            && diagnostics.mark.active().is_none()
        {
            continue;
        }
        let state = visual_state(checked, tri_state);
        let result = (|| {
            let children = children.ok_or_else(|| missing(root, root, "Children"))?;
            let indicator = children
                .iter()
                .find(|child| indicators.contains(*child))
                .ok_or_else(|| missing(root, root, "indicator"))?;
            let children = indicators
                .get(indicator)
                .map_err(|source| query_error(root, indicator, "indicator", source))?
                .ok_or_else(|| missing(root, indicator, "indicator.Children"))?;
            let mark_entity = children
                .iter()
                .find(|child| marks.contains(*child))
                .ok_or_else(|| missing(root, indicator, "mark"))?;
            let (mark, icon, visibility) = marks
                .get_mut(mark_entity)
                .map_err(|source| query_error(root, mark_entity, "mark", source))?;
            let mut mark = mark.ok_or_else(|| missing(root, mark_entity, "mark.CheckBoxMark"))?;
            if !icon {
                return Err(missing(root, mark_entity, "mark.WidgetryIcon"));
            }
            let mut visibility =
                visibility.ok_or_else(|| missing(root, mark_entity, "mark.Visibility"))?;
            let desired = match state {
                CheckBoxVisualState::Unchecked => None,
                CheckBoxVisualState::Checked => Some(BuiltinIcon::CheckboxCheck),
                CheckBoxVisualState::Indeterminate => Some(BuiltinIcon::CheckboxIndeterminate),
            };
            visibility.set_if_neq(if desired.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            if let Some(icon) = desired
                && mark.icon != Some(icon)
            {
                WidgetryIcon::set_svg(&mut commands, mark_entity, icon.path());
                mark.icon = Some(icon);
            }
            Ok(())
        })();
        push(
            &mut batch,
            observe(
                &mut diagnostics.mark,
                result,
                ErrorFrame::new("check_box.mark")
                    .field("root", root)
                    .field("state", format_args!("{state:?}")),
            ),
        );
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

fn resolve_style(
    colors: &bevy_widgetry_theme::WidgetryCheckBoxColors,
    state: CheckBoxVisualState,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> CheckBoxStyle {
    let colors = match state {
        CheckBoxVisualState::Unchecked => colors.unchecked,
        CheckBoxVisualState::Checked => colors.checked,
        CheckBoxVisualState::Indeterminate => colors.indeterminate,
    };
    let colors = if disabled {
        colors.disabled
    } else if pressed {
        colors.pressed
    } else if hovered {
        colors.hovered
    } else {
        colors.normal
    };
    CheckBoxStyle {
        background: colors.background,
        border: colors.border,
        foreground: colors.foreground,
        mark: colors.mark,
    }
}

fn apply_style(
    colors: &bevy_widgetry_theme::WidgetryCheckBoxColors,
    (
        root,
        overrides,
        hovered,
        pressed,
        disabled,
        checked,
        tri_state,
        children,
        foreground,
        mut diagnostics,
    ): <RootStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    indicators: &mut Query<IndicatorData, With<CheckBoxIndicator>>,
    marks: &mut Query<MarkData, MarkFilter>,
    commands: &mut Commands,
) -> Result<(), WidgetryEcsError> {
    let state = visual_state(checked, tri_state);
    let result = (|| {
        let overrides = overrides.ok_or_else(|| missing(root, root, "ColorState"))?;
        let hovered = hovered.ok_or_else(|| missing(root, root, "Hovered"))?;
        let children = children.ok_or_else(|| missing(root, root, "Children"))?;
        let mut foreground = foreground.ok_or_else(|| missing(root, root, "ResolvedForeground"))?;
        let colors = overrides.0.resolve(colors);
        let style = resolve_style(&colors, state, hovered.0, pressed, disabled);
        foreground.set_if_neq(ResolvedForeground(style.foreground));
        let indicator = children
            .iter()
            .find(|child| indicators.contains(*child))
            .ok_or_else(|| missing(root, root, "indicator"))?;
        let (children, background, border) = indicators
            .get_mut(indicator)
            .map_err(|source| query_error(root, indicator, "indicator", source))?;
        let children = children.ok_or_else(|| missing(root, indicator, "indicator.Children"))?;
        let mut background =
            background.ok_or_else(|| missing(root, indicator, "indicator.BackgroundColor"))?;
        let mut border = border.ok_or_else(|| missing(root, indicator, "indicator.BorderColor"))?;
        let mark_entity = children
            .iter()
            .find(|child| marks.contains(*child))
            .ok_or_else(|| missing(root, indicator, "mark"))?;
        let (mark, icon, _) = marks
            .get_mut(mark_entity)
            .map_err(|source| query_error(root, mark_entity, "mark", source))?;
        let mut mark = mark.ok_or_else(|| missing(root, mark_entity, "mark.CheckBoxMark"))?;
        if !icon {
            return Err(missing(root, mark_entity, "mark.WidgetryIcon"));
        }
        background.set_if_neq(BackgroundColor(style.background));
        border.set_if_neq(BorderColor::all(style.border));
        if state != CheckBoxVisualState::Unchecked && mark.color != Some(style.mark) {
            commands
                .entity(mark_entity)
                .insert(ResolvedForeground(style.mark));
            mark.color = Some(style.mark);
        }
        Ok(())
    })();
    observe(
        &mut diagnostics.colors,
        result,
        ErrorFrame::new("check_box.colors")
            .field("root", root)
            .field("state", format_args!("{state:?}"))
            .field("hovered", hovered.is_some_and(|hovered| hovered.0))
            .field("pressed", pressed)
            .field("disabled", disabled),
    )
}

pub(crate) fn update_changed(
    mode: Res<WidgetryTheme>,
    mut roots: Query<RootStyleData, OwnedFilter>,
    changed: Query<
        (),
        Or<(
            Added<WidgetryCheckBox>,
            Added<WidgetryTriStateCheckbox>,
            Changed<Hovered>,
            Changed<crate::colors::ColorState>,
            Added<Pressed>,
            Added<Checked>,
            Added<InteractionDisabled>,
            Changed<WidgetryCheckState>,
        )>,
    >,
    mut indicators: Query<IndicatorData, With<CheckBoxIndicator>>,
    mut marks: Query<MarkData, MarkFilter>,
    mut commands: Commands,
) -> Result {
    let mut batch = DiagnosticBatch::default();
    for item in &mut roots {
        if changed.contains(item.0) || item.9.colors.active().is_some() {
            push(
                &mut batch,
                apply_style(
                    &mode.check_box,
                    item,
                    &mut indicators,
                    &mut marks,
                    &mut commands,
                ),
            );
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

pub(crate) fn update_removed(
    mode: Res<WidgetryTheme>,
    mut pressed: RemovedComponents<Pressed>,
    mut checked: RemovedComponents<Checked>,
    mut disabled: RemovedComponents<InteractionDisabled>,
    mut roots: Query<RootStyleData, OwnedFilter>,
    mut indicators: Query<IndicatorData, With<CheckBoxIndicator>>,
    mut marks: Query<MarkData, MarkFilter>,
    mut commands: Commands,
) -> Result {
    let mut batch = DiagnosticBatch::default();
    for entity in pressed.read().chain(checked.read()).chain(disabled.read()) {
        if let Ok(item) = roots.get_mut(entity) {
            push(
                &mut batch,
                apply_style(
                    &mode.check_box,
                    item,
                    &mut indicators,
                    &mut marks,
                    &mut commands,
                ),
            );
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result {
        let theme = world.resource::<WidgetryTheme>().check_box;
        let roots = world
            .query_filtered::<Entity, OwnedFilter>()
            .iter(world)
            .collect::<Vec<_>>();
        let mut batch = DiagnosticBatch::default();
        for root in roots {
            push(&mut batch, apply_owned_checkbox_colors(world, root, &theme));
        }
        batch.into_bevy_error().map_or(Ok(()), Err)
    });
}

pub fn apply_owned_checkbox_colors(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryCheckBoxColors,
) -> Result<(), WidgetryEcsError> {
    if world.get::<WidgetryCheckBox>(root).is_none()
        && world.get::<WidgetryTriStateCheckbox>(root).is_none()
    {
        return Err(WidgetryEcsError::new(crate::error::failure(
            CheckBoxError::InvalidTarget { root },
            "check_box.apply_owned_colors",
        )));
    }
    let mut state = bevy::ecs::system::SystemState::<(
        Query<RootStyleData>,
        Query<IndicatorData, With<CheckBoxIndicator>>,
        Query<MarkData, MarkFilter>,
        Commands,
    )>::new(world);
    let (mut roots, mut indicators, mut marks, mut commands) =
        state.get_mut(world).map_err(|source| {
            WidgetryEcsError::new(crate::error::failure(
                CheckBoxError::SystemQuery { root, source },
                "check_box.apply_owned_colors",
            ))
        })?;
    let result = roots
        .get_mut(root)
        .map_err(|source| {
            WidgetryEcsError::new(crate::error::failure(
                query_error(root, root, "StyleDiagnostics", source).into_kind(),
                "check_box.apply_owned_colors",
            ))
        })
        .and_then(|item| apply_style(colors, item, &mut indicators, &mut marks, &mut commands));
    state.apply(world);
    result
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{WidgetryCheckBoxPlugin, WidgetryTriStateCheckbox};
    use bevy_widgetry_test_utils::{LogCapture, scene_app};

    #[test]
    fn every_broken_owner_reaches_the_host_while_a_healthy_root_keeps_updating() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture};
        let mut app = scene_app();
        app.add_plugins(WidgetryCheckBoxPlugin);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let roots: Vec<_> = (0..3)
            .map(|_| {
                app.world_mut()
                    .spawn_scene(bsn! { @WidgetryCheckBox })
                    .unwrap()
                    .id()
            })
            .collect();
        app.update();
        let indicators: Vec<_> = roots
            .iter()
            .map(|root| app.world().get::<Children>(*root).unwrap()[0])
            .collect();
        for indicator in &indicators[..2] {
            app.world_mut()
                .entity_mut(*indicator)
                .remove::<BackgroundColor>();
        }
        for root in &roots {
            app.world_mut().entity_mut(*root).insert(Hovered(true));
        }
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
                let records = errors.take_records();
                assert_eq!(records.len(), 2);
                let ids: Vec<Vec<_>> = records
                    .iter()
                    .map(|record| {
                        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
                        let diagnostics: Vec<_> = record.managed().unwrap().diagnostics().collect();
                        assert_eq!(diagnostics.len(), 2);
                        diagnostics
                            .iter()
                            .map(|diagnostic| {
                                assert_eq!(
                                    logs.diagnostic_records(
                                        diagnostic.origin().id(),
                                        DiagnosticPhase::Source
                                    )
                                    .len(),
                                    1
                                );
                                diagnostic.origin().id().clone()
                            })
                            .collect()
                    })
                    .collect();
                assert_eq!(ids[0], ids[1]);
                assert_ne!(ids[0][0], ids[0][1]);
                assert_eq!(
                    app.world().get::<BackgroundColor>(indicators[2]).unwrap().0,
                    bevy_widgetry_theme::WIDGETRY_DARK_THEME
                        .check_box
                        .unchecked
                        .hovered
                        .background
                );
                for indicator in &indicators[..2] {
                    app.world_mut()
                        .entity_mut(*indicator)
                        .insert(BackgroundColor(Color::NONE));
                }
                app.update();
                assert!(errors.take_records().is_empty());
                assert_eq!(
                    logs.records()
                        .iter()
                        .filter(|record| record.matches_field("phase", "recovered"))
                        .count(),
                    2
                );
                app.world_mut()
                    .entity_mut(indicators[0])
                    .remove::<BackgroundColor>();
                app.world_mut().entity_mut(roots[0]).insert(Hovered(false));
                app.update();
                let records = errors.take_records();
                let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
                assert!(!ids[0].contains(diagnostic.origin().id()));
            })
        });
    }

    #[test]
    fn damaged_mark_geometry_keeps_all_owners_and_recovers_current_state() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture};
        let mut app = scene_app();
        app.add_plugins(WidgetryCheckBoxPlugin);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let roots: Vec<_> = (0..3)
            .map(|_| {
                app.world_mut()
                    .spawn_scene(bsn! { @WidgetryCheckBox })
                    .unwrap()
                    .id()
            })
            .collect();
        app.update();
        let marks: Vec<_> = roots
            .iter()
            .map(|root| {
                let indicator = app.world().get::<Children>(*root).unwrap()[0];
                app.world().get::<Children>(indicator).unwrap()[0]
            })
            .collect();
        for mark in &marks[..2] {
            app.world_mut().entity_mut(*mark).remove::<Visibility>();
        }
        for root in &roots {
            app.world_mut().entity_mut(*root).insert(Checked);
        }
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| errors.run(|| {
            app.update();
            app.update();
            let records = errors.take_records();
            assert_eq!(records.len(), 2);
            let ids: Vec<Vec<_>> = records.iter().map(|record| record.managed().unwrap().diagnostics().map(|diagnostic| {
                assert!(matches!(diagnostic.source().unwrap().downcast_ref::<CheckBoxError>(), Some(CheckBoxError::MissingPart { entity, part: "mark.Visibility", .. }) if marks[..2].contains(entity)));
                assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
                diagnostic.origin().id().clone()
            }).collect()).collect();
            assert_eq!(ids[0].len(), 2);
            assert_eq!(ids[0], ids[1]);
            assert_eq!(*app.world().get::<Visibility>(marks[2]).unwrap(), Visibility::Inherited);
            for mark in &marks[..2] { app.world_mut().entity_mut(*mark).insert(Visibility::Hidden); }
            app.update();
            assert!(errors.take_records().is_empty());
            assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
            for mark in marks { assert_eq!(*app.world().get::<Visibility>(mark).unwrap(), Visibility::Inherited); }
        }));
    }

    #[test]
    fn style_priority() {
        let colors = bevy_widgetry_theme::WIDGETRY_DARK_THEME;
        for (state, hover, press, disabled, background, border) in [
            (
                CheckBoxVisualState::Checked,
                true,
                true,
                true,
                colors.check_box.unchecked.disabled.background,
                colors.check_box.unchecked.disabled.border,
            ),
            (
                CheckBoxVisualState::Indeterminate,
                true,
                true,
                false,
                colors.check_box.indeterminate.pressed.background,
                colors.check_box.indeterminate.pressed.border,
            ),
            (
                CheckBoxVisualState::Checked,
                true,
                false,
                false,
                colors.check_box.checked.hovered.background,
                colors.check_box.checked.hovered.border,
            ),
            (
                CheckBoxVisualState::Indeterminate,
                false,
                false,
                false,
                colors.check_box.checked.normal.background,
                colors.check_box.checked.normal.border,
            ),
            (
                CheckBoxVisualState::Unchecked,
                false,
                false,
                false,
                colors.check_box.unchecked.normal.background,
                colors.check_box.unchecked.normal.border,
            ),
        ] {
            let style = resolve_style(&colors.check_box, state, hover, press, disabled);
            assert_eq!(style.background, background);
            assert_eq!(style.border, border);
            assert_eq!(
                style.foreground,
                if disabled {
                    colors.check_box.unchecked.disabled.foreground
                } else {
                    colors.check_box.unchecked.normal.foreground
                }
            );
            assert_eq!(
                style.mark,
                if state == CheckBoxVisualState::Unchecked {
                    Color::NONE
                } else if disabled {
                    colors.check_box.checked.disabled.mark
                } else {
                    colors.check_box.checked.normal.mark
                }
            );
        }
    }

    #[test]
    fn mark_svg_switches_without_replacing_entity() {
        let mut app = scene_app();
        app.add_plugins(WidgetryCheckBoxPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
            .expect("test entity exists")
            .id();
        app.update();
        let indicator = app
            .world()
            .get::<Children>(root)
            .expect("test entity exists")[0];
        let mark = app
            .world()
            .get::<Children>(indicator)
            .expect("test entity exists")[0];
        for (state, icon) in [
            (WidgetryCheckState::Checked, BuiltinIcon::CheckboxCheck),
            (
                WidgetryCheckState::Indeterminate,
                BuiltinIcon::CheckboxIndeterminate,
            ),
        ] {
            WidgetryTriStateCheckbox::set_state(&mut app.world_mut().commands(), root, state);
            app.update();
            assert_eq!(
                app.world()
                    .get::<CheckBoxMark>(mark)
                    .expect("test entity exists")
                    .icon,
                Some(icon)
            );
            assert_eq!(
                app.world()
                    .get::<Children>(indicator)
                    .expect("test entity exists")[0],
                mark
            );
        }
        WidgetryTriStateCheckbox::set_state(
            &mut app.world_mut().commands(),
            root,
            WidgetryCheckState::Unchecked,
        );
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(mark),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world()
                .get::<Children>(indicator)
                .expect("test entity exists")[0],
            mark
        );
    }

    #[test]
    fn broken_internal_structure_is_logged() {
        let capture = LogCapture::default();
        let mut app = scene_app();
        app.add_plugins(WidgetryCheckBoxPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
            .unwrap()
            .id();
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        app.update();
        let indicator = app.world().get::<Children>(root).unwrap()[0];
        let mark = app.world().get::<Children>(indicator).unwrap()[0];

        app.world_mut()
            .entity_mut(indicator)
            .remove::<CheckBoxIndicator>();
        app.world_mut().entity_mut(root).insert(Hovered(true));
        capture.run(|| app.update());
        capture.run(|| app.update());
        app.world_mut()
            .entity_mut(indicator)
            .insert(CheckBoxIndicator);
        app.world_mut().entity_mut(mark).remove::<CheckBoxMark>();
        app.world_mut().entity_mut(root).insert(Hovered(false));
        capture.run(|| app.update());

        let records = capture.records();
        let errors: Vec<_> = records
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .collect();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].fields.get("error").unwrap().contains("indicator"));
        assert!(
            errors[0]
                .fields
                .get("context")
                .unwrap()
                .contains(&root.to_string())
        );
        assert!(errors[1].fields.get("error").unwrap().contains("mark"));
        assert!(
            errors[1]
                .fields
                .get("context")
                .unwrap()
                .contains(&root.to_string())
        );
    }
}
