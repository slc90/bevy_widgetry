use crate::{
    SegmentedError,
    error::{collect, failure},
};
use crate::{
    WidgetrySegmented, WidgetrySegmentedOption,
    colors::{ColorState, OptionColorState},
};
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};
use std::sync::Arc;

#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState<StyleIssue>);
#[derive(Clone, PartialEq, Eq)]
enum StyleIssue {
    Missing {
        group: Option<Entity>,
        option: Option<Entity>,
        entity: Entity,
        part: &'static str,
    },
    Parent {
        option: Entity,
        parent: Option<Entity>,
    },
}
impl StyleIssue {
    fn into_kind(self) -> SegmentedError {
        match self {
            Self::Missing {
                group,
                option,
                entity,
                part,
            } => SegmentedError::MissingPart {
                group,
                option,
                entity,
                part,
            },
            Self::Parent { option, parent } => SegmentedError::InvalidParent { option, parent },
        }
    }
}
fn missing(
    group: Entity,
    option: Option<Entity>,
    entity: Entity,
    part: &'static str,
) -> StyleIssue {
    StyleIssue::Missing {
        group: Some(group),
        option,
        entity,
        part,
    }
}

#[derive(Resource, Default)]
struct StyleChanges(EntityHashSet);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<StyleChanges>()
        .add_observer(removed_input)
        .add_observer(refresh_theme)
        .add_systems(
            PostUpdate,
            (collect_changes, update)
                .chain()
                .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
    let world = app.world_mut();
    let entities = world
        .query_filtered::<Entity, Or<(With<WidgetrySegmented>, With<WidgetrySegmentedOption>)>>()
        .iter(world)
        .collect::<Vec<_>>();
    world.resource_mut::<StyleChanges>().0.extend(entities);
}

fn removed_input(
    event: On<
        Remove<(
            Checked,
            Hovered,
            InteractionDisabled,
            ColorState,
            OptionColorState,
            ChildOf,
            BackgroundColor,
            BorderColor,
            ResolvedForeground,
            WidgetrySegmented,
        )>,
    >,
    groups: Query<&Children, With<WidgetrySegmented>>,
    mut changes: ResMut<StyleChanges>,
) {
    changes.0.insert(event.entity);
    if let Ok(children) = groups.get(event.entity) {
        changes.0.extend(children.iter());
    }
}

fn refresh_theme(
    _: On<WidgetryThemeChanged>,
    entities: Query<Entity, Or<(With<WidgetrySegmented>, With<WidgetrySegmentedOption>)>>,
    mut changes: ResMut<StyleChanges>,
) {
    changes.0.extend(entities.iter());
}

fn collect_changes(
    inputs: Query<
        Entity,
        (
            Or<(With<WidgetrySegmented>, With<WidgetrySegmentedOption>)>,
            Or<(
                Added<WidgetrySegmented>,
                Added<WidgetrySegmentedOption>,
                Changed<ColorState>,
                Changed<OptionColorState>,
                Changed<Hovered>,
                Changed<ChildOf>,
                Added<Checked>,
                Added<InteractionDisabled>,
                Added<BackgroundColor>,
                Added<BorderColor>,
                Added<ResolvedForeground>,
            )>,
        ),
    >,
    groups: Query<
        &Children,
        (
            With<WidgetrySegmented>,
            Or<(Added<WidgetrySegmented>, Changed<ColorState>)>,
        ),
    >,
    focus: Res<InputFocus>,
    mut previous_focus: Local<Option<Entity>>,
    mut changes: ResMut<StyleChanges>,
) {
    changes.0.extend(inputs.iter());
    for children in &groups {
        changes.0.extend(children.iter());
    }
    if focus.is_changed() {
        changes.0.extend(*previous_focus);
        changes.0.extend(focus.get());
        *previous_focus = focus.get();
    }
}

fn update(world: &mut World) -> Result<(), BevyError> {
    let dirty = std::mem::take(&mut world.resource_mut::<StyleChanges>().0);
    let mut batch = DiagnosticBatch::default();
    for entity in dirty {
        if world.get::<WidgetrySegmented>(entity).is_none()
            && world.get::<WidgetrySegmentedOption>(entity).is_none()
        {
            continue;
        }
        #[cfg(test)]
        STYLE_WORK.with_borrow_mut(|work| work.push(entity));
        let frame = || ErrorFrame::new("segmented.style").field("entity", entity);
        match apply(world, entity) {
            Ok(()) => {
                if let Some(mut diagnostics) = world.get_mut::<StyleDiagnostics>(entity) {
                    diagnostics.0.recover(frame);
                }
            }
            Err(issue) => {
                let key = issue.clone();
                let create = || -> Arc<dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
                    Arc::new(failure(issue.into_kind(), frame()))
                };
                let error = if let Some(mut diagnostics) = world.get_mut::<StyleDiagnostics>(entity)
                {
                    diagnostics.0.failure(key, create)
                } else {
                    WidgetryEcsError::from_shared(create())
                };
                collect(&mut batch, Err(error));
            }
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

fn apply(world: &mut World, entity: Entity) -> Result<(), StyleIssue> {
    let theme = world.resource::<WidgetryThemeMode>().colors().segmented;
    let disabled = world.get::<InteractionDisabled>(entity).is_some();
    if world.get::<WidgetrySegmented>(entity).is_some() {
        let colors = world
            .get::<ColorState>(entity)
            .ok_or_else(|| missing(entity, None, entity, "ColorState"))?
            .0
            .resolve(&theme)
            .container;
        let colors = if disabled {
            colors.disabled
        } else if world.resource::<InputFocus>().get() == Some(entity) {
            colors.focused
        } else {
            colors.normal
        };
        if world.get::<BackgroundColor>(entity).is_none() {
            return Err(missing(entity, None, entity, "BackgroundColor"));
        }
        if world.get::<BorderColor>(entity).is_none() {
            return Err(missing(entity, None, entity, "BorderColor"));
        }
        if let Some(mut background) = world.get_mut::<BackgroundColor>(entity) {
            background.set_if_neq(BackgroundColor(colors.background));
        }
        if let Some(mut border) = world.get_mut::<BorderColor>(entity) {
            border.set_if_neq(BorderColor::all(colors.border));
        }
    } else {
        let root = world
            .get::<ChildOf>(entity)
            .map(ChildOf::parent)
            .ok_or(StyleIssue::Parent {
                option: entity,
                parent: None,
            })?;
        if world.get::<WidgetrySegmented>(root).is_none() {
            return Err(StyleIssue::Parent {
                option: entity,
                parent: Some(root),
            });
        }
        let colors = world
            .get::<ColorState>(root)
            .ok_or_else(|| missing(root, Some(entity), root, "ColorState"))?
            .0
            .resolve(&theme);
        let colors = world
            .get::<OptionColorState>(entity)
            .ok_or_else(|| missing(root, Some(entity), entity, "OptionColorState"))?
            .0
            .resolve(&colors.option);
        let colors = if world.get::<Checked>(entity).is_some() {
            colors.checked
        } else {
            colors.unchecked
        };
        let colors = if disabled {
            colors.disabled
        } else if world
            .get::<Hovered>(entity)
            .is_some_and(|hovered| hovered.0)
        {
            colors.hovered
        } else {
            colors.normal
        };
        if world.get::<BackgroundColor>(entity).is_none() {
            return Err(missing(root, Some(entity), entity, "BackgroundColor"));
        }
        if world.get::<ResolvedForeground>(entity).is_none() {
            return Err(missing(root, Some(entity), entity, "ResolvedForeground"));
        }
        if let Some(mut background) = world.get_mut::<BackgroundColor>(entity) {
            background.set_if_neq(BackgroundColor(colors.background));
        }
        if let Some(mut foreground) = world.get_mut::<ResolvedForeground>(entity) {
            foreground.set_if_neq(ResolvedForeground(colors.foreground));
        }
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static STYLE_WORK: std::cell::RefCell<Vec<Entity>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

    #[test]
    fn idle_style_does_no_work_and_local_changes_only_resolve_the_target() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetrySegmentedPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetrySegmented Children [
                @WidgetrySegmentedOption-- @WidgetrySegmentedOption
            ] })
            .unwrap()
            .id();
        app.update();
        app.update();
        STYLE_WORK.with_borrow_mut(Vec::clear);
        app.update();
        STYLE_WORK.with_borrow(|work| assert!(work.is_empty()));
        let option = app.world().get::<Children>(root).unwrap()[1];
        app.world_mut().entity_mut(option).insert(Hovered(true));
        app.update();
        STYLE_WORK.with_borrow_mut(|work| {
            assert_eq!(*work, [option]);
            work.clear();
        });
        app.update();
        STYLE_WORK.with_borrow(|work| assert!(work.is_empty()));
    }

    #[test]
    fn removed_output_reports_once_and_recovers_after_reinsertion() {
        let mut app = scene_app();
        app.set_error_handler(ErrorCapture::handler());
        app.add_plugins(crate::WidgetrySegmentedPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetrySegmented Children [@WidgetrySegmentedOption] })
            .unwrap()
            .id();
        app.update();
        let option = app.world().get::<Children>(root).unwrap()[0];
        let errors = ErrorCapture::default();
        app.world_mut()
            .entity_mut(option)
            .remove::<BackgroundColor>();
        errors.run(|| app.update());
        assert_eq!(errors.take().len(), 1);
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        app.world_mut()
            .entity_mut(option)
            .insert(BackgroundColor::default());
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        assert_eq!(
            app.world().get::<BackgroundColor>(option).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .segmented
                .option
                .checked
                .normal
                .background
        );
    }
}
