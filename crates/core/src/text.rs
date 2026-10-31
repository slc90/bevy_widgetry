mod colors;
pub use colors::{WidgetryTextColorOverrides, WidgetryTextStateColorOverrides};

use crate::diagnostics::{CoreError, CoreFailure, FailureState};
use crate::{foreground::InheritedForeground, ui::WidgetryUiSystems};
use bevy::prelude::*;
use bevy::text::TextSpan;
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::DiagnosticBatch};
use bevy_widgetry_theme::WidgetryThemeMode;
use std::sync::Arc;

#[derive(Component, FromTemplate, Default, Clone)]
#[require(colors::ColorState, TextStructureState)]
pub struct WidgetryText;

#[derive(Component, Default)]
struct TextStructureState(FailureState<&'static str>);

#[derive(Default, Clone, Debug)]
pub struct WidgetryTextProps {
    pub colors: WidgetryTextColorOverrides,
}

// Color markers can be added directly to existing Text, so they cannot depend on Scene construction.
impl SceneComponent for WidgetryText {
    type Props = WidgetryTextProps;

    fn scene(props: Self::Props) -> impl Scene {
        bsn! { WidgetryText template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error())) }
    }
}

impl WidgetryText {
    pub fn set_color_in_world(
        world: &mut World,
        entity: Entity,
        color: Color,
    ) -> Result<bool, CoreFailure> {
        WidgetryTextColorOverrides::set_in_world(
            world,
            entity,
            WidgetryTextColorOverrides {
                normal: WidgetryTextStateColorOverrides {
                    foreground: Some(color),
                },
                disabled: WidgetryTextStateColorOverrides {
                    foreground: Some(color),
                },
            },
        )
    }

    pub fn clear_color_in_world(world: &mut World, entity: Entity) -> Result<bool, CoreFailure> {
        WidgetryTextColorOverrides::clear_in_world(world, entity)
    }
    pub fn set_color(commands: &mut Commands, entity: Entity, color: Color) {
        commands.queue(move |world: &mut World| {
            Self::set_color_in_world(world, entity, color)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
    pub fn clear_color(commands: &mut Commands, entity: Entity) {
        WidgetryTextColorOverrides::clear(commands, entity);
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        apply_text_colors.in_set(WidgetryUiSystems::ContentColors),
    );
}

fn apply_text_colors(
    mode: Res<WidgetryThemeMode>,
    mut query: Query<
        (
            Entity,
            Option<&colors::ColorState>,
            Option<&InheritedForeground>,
            &crate::foreground::ContentContext,
            Option<&mut TextColor>,
            &mut TextStructureState,
        ),
        (
            With<WidgetryText>,
            Without<bevy::text::EditableText>,
            Or<(With<Text>, With<TextSpan>)>,
        ),
    >,
) -> Result<(), BevyError> {
    let mut failures = DiagnosticBatch::default();
    for (entity, overrides, inherited, context, output, mut state) in &mut query {
        if !context.ui_text {
            continue;
        }
        let missing = if overrides.is_none() {
            Some("WidgetryText ColorState")
        } else if output.is_none() {
            Some("TextColor")
        } else {
            None
        };
        if let Some(missing) = missing {
            let error = state.0.failure(missing, || {
                Arc::new(crate::diagnostics::failure(
                    CoreError::InvalidTarget { entity, missing },
                    "project Text color",
                ))
            });
            for diagnostic in error.shared_diagnostics() {
                failures.push_shared(diagnostic);
            }
            continue;
        }
        let (Some(overrides), Some(mut output)) = (overrides, output) else {
            continue;
        };
        state
            .0
            .recover(|| ErrorFrame::new("Text structure restored").field("entity", entity));
        let disabled = context.disabled;
        let resolved = overrides.0.resolve(&mode.colors().text);
        let (local, theme) = if disabled {
            (&overrides.0.disabled, resolved.disabled)
        } else {
            (&overrides.0.normal, resolved.normal)
        };
        let color = local
            .foreground
            .or_else(|| {
                inherited
                    .filter(|source| source.disabled == disabled)
                    .map(|source| source.color)
            })
            .unwrap_or(theme.foreground);
        output.set_if_neq(TextColor(color));
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};

    #[test]
    fn owned_text_structure_failures_keep_all_origins_until_actual_recovery() {
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        logs.run(|| errors.run(|| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), crate::ui::WidgetryUiPlugin)).init_asset::<Image>();
            app.set_error_handler(ErrorCapture::handler());
            app.edit_schedule(PostUpdate, |schedule| { schedule.set_executor(SingleThreadedExecutor::new()); });
            let a = app.world_mut().spawn((Node::default(), Text::new("a"), WidgetryText)).id();
            let b = app.world_mut().spawn((Node::default(), Text::new("b"), WidgetryText)).id();
            app.update();
            app.world_mut().entity_mut(a).remove::<colors::ColorState>();
            app.world_mut().entity_mut(b).remove::<TextColor>();
            app.update();
            app.update();
            let records = errors.take_records();
            assert_eq!(records.len(), 2);
            let ids: Vec<Vec<_>> = records.iter().map(|record| {
                assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
                let diagnostics: Vec<_> = record.error.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().collect();
                assert_eq!(diagnostics.len(), 2);
                diagnostics.iter().map(|diagnostic| {
                    assert!(matches!(diagnostic.source().unwrap().downcast_ref::<CoreError>(), Some(CoreError::InvalidTarget { entity, .. }) if [a, b].contains(entity)));
                    assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
                    diagnostic.origin().id().clone()
                }).collect()
            }).collect();
            assert_eq!(ids[0], ids[1]);
            assert_ne!(ids[0][0], ids[0][1]);
            app.world_mut().entity_mut(a).insert(colors::ColorState::default());
            app.world_mut().entity_mut(b).insert(TextColor::default());
            app.update();
            app.update();
            assert!(errors.take_records().is_empty());
            assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
        }));
    }
}
