use crate::{
    ScrollAreaError, ScrollAreaFailure, ScrollAxis,
    error::{PartIssue, core, failure},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollThumbStateColorOverrides {
    pub background: Option<Color>,
}
impl WidgetryScrollThumbStateColorOverrides {
    fn validate(
        &self,
        root: Entity,
        axis: ScrollAxis,
        state: &'static str,
    ) -> Result<(), ScrollAreaFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("scroll_area.validate_colors")
                        .field("root", root)
                        .field("axis", format_args!("{axis:?}"))
                        .field("state", state)
                        .field("field", "background"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollThumbStateColors,
    ) -> bevy_widgetry_theme::WidgetryScrollThumbStateColors {
        bevy_widgetry_theme::WidgetryScrollThumbStateColors {
            background: self.background.unwrap_or(theme.background),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollThumbColorOverrides {
    pub normal: WidgetryScrollThumbStateColorOverrides,
    pub hovered: WidgetryScrollThumbStateColorOverrides,
    pub dragged: WidgetryScrollThumbStateColorOverrides,
    pub disabled: WidgetryScrollThumbStateColorOverrides,
}
impl WidgetryScrollThumbColorOverrides {
    fn validate(&self, root: Entity, axis: ScrollAxis) -> Result<(), ScrollAreaFailure> {
        self.normal.validate(root, axis, "thumb.normal")?;
        self.hovered.validate(root, axis, "thumb.hovered")?;
        self.dragged.validate(root, axis, "thumb.dragged")?;
        self.disabled.validate(root, axis, "thumb.disabled")?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollThumbColors,
    ) -> bevy_widgetry_theme::WidgetryScrollThumbColors {
        bevy_widgetry_theme::WidgetryScrollThumbColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            dragged: self.dragged.resolve(&theme.dragged),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollTrackStateColorOverrides {
    pub background: Option<Color>,
}
impl WidgetryScrollTrackStateColorOverrides {
    fn validate(
        &self,
        root: Entity,
        axis: ScrollAxis,
        state: &'static str,
    ) -> Result<(), ScrollAreaFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("scroll_area.validate_colors")
                        .field("root", root)
                        .field("axis", format_args!("{axis:?}"))
                        .field("state", state)
                        .field("field", "background"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollTrackStateColors,
    ) -> bevy_widgetry_theme::WidgetryScrollTrackStateColors {
        bevy_widgetry_theme::WidgetryScrollTrackStateColors {
            background: self.background.unwrap_or(theme.background),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollTrackColorOverrides {
    pub normal: WidgetryScrollTrackStateColorOverrides,
    pub disabled: WidgetryScrollTrackStateColorOverrides,
}
impl WidgetryScrollTrackColorOverrides {
    fn validate(&self, root: Entity, axis: ScrollAxis) -> Result<(), ScrollAreaFailure> {
        self.normal.validate(root, axis, "track.normal")?;
        self.disabled.validate(root, axis, "track.disabled")?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollTrackColors,
    ) -> bevy_widgetry_theme::WidgetryScrollTrackColors {
        bevy_widgetry_theme::WidgetryScrollTrackColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollAxisColorOverrides {
    pub track: WidgetryScrollTrackColorOverrides,
    pub thumb: WidgetryScrollThumbColorOverrides,
}
impl WidgetryScrollAxisColorOverrides {
    fn validate(&self, root: Entity, axis: ScrollAxis) -> Result<(), ScrollAreaFailure> {
        self.track.validate(root, axis)?;
        self.thumb.validate(root, axis)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollAxisColors,
    ) -> bevy_widgetry_theme::WidgetryScrollAxisColors {
        bevy_widgetry_theme::WidgetryScrollAxisColors {
            track: self.track.resolve(&theme.track),
            thumb: self.thumb.resolve(&theme.thumb),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollAreaColorOverrides {
    pub horizontal: WidgetryScrollAxisColorOverrides,
    pub vertical: WidgetryScrollAxisColorOverrides,
}
impl WidgetryScrollAreaColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ScrollAreaFailure> {
        self.horizontal.validate(root, ScrollAxis::Horizontal)?;
        self.vertical.validate(root, ScrollAxis::Vertical)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollAreaColors,
    ) -> bevy_widgetry_theme::WidgetryScrollAreaColors {
        bevy_widgetry_theme::WidgetryScrollAreaColors {
            horizontal: self.horizontal.resolve(&theme.horizontal),
            vertical: self.vertical.resolve(&theme.vertical),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryScrollAreaColorOverrides);
impl WidgetryScrollAreaColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, ScrollAreaFailure> {
        if world.get::<crate::WidgetryScrollArea>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| {
                PartIssue::new(entity, "ColorState", None).into_failure(
                    world,
                    entity,
                    "scroll_area.get_colors",
                )
            })
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, ScrollAreaFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryScrollArea>>(entity)
        {
            return Err(failure(
                ScrollAreaError::ManagedColors {
                    root: entity,
                    owner: owner.entity,
                },
                ErrorFrame::new("scroll_area.set_colors"),
            ));
        }
        colors.validate(entity)?;
        let mut state = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| invalid_target(entity))?;
        if state.0 == colors {
            return Ok(false);
        }
        state.0 = colors;
        Ok(true)
    }
    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, entity, colors)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, ScrollAreaFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, ScrollAreaFailure> {
        self.validate(root)?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(root: Entity) -> ScrollAreaFailure {
    failure(
        ScrollAreaError::InvalidTarget { root },
        ErrorFrame::new("scroll_area.get_colors"),
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

    #[test]
    fn missing_color_states_batch_without_recreating_sources_on_theme_refresh() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let roots: Vec<_> = (0..2)
            .map(|_| {
                app.world_mut()
                    .spawn_scene(bsn! { @crate::WidgetryScrollArea })
                    .unwrap()
                    .id()
            })
            .collect();
        app.update();
        for root in &roots {
            app.world_mut().entity_mut(*root).remove::<ColorState>();
        }
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| errors.run(|| {
            app.update();
            let first = errors.take_records();
            let first: Vec<_> = first[0].managed().unwrap().diagnostics().collect();
            assert_eq!(first.len(), 2);
            for diagnostic in &first {
                assert!(matches!(diagnostic.source().unwrap().downcast_ref::<ScrollAreaError>(), Some(ScrollAreaError::MissingPart { context, entity, part: "ColorState" }) if roots.contains(entity) && context.root == *entity));
            }
            bevy_widgetry_theme::WidgetryTheme::set_in_world(app.world_mut(), bevy_widgetry_theme::WIDGETRY_LIGHT_THEME).unwrap();
            let repeated = errors.take_records();
            let repeated: Vec<_> = repeated[0].managed().unwrap().diagnostics().collect();
            assert_eq!(repeated.len(), 2);
            for diagnostic in repeated {
                assert!(first.iter().any(|original| std::sync::Arc::ptr_eq(original.origin(), diagnostic.origin())));
                assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
            }
            for root in roots { app.world_mut().entity_mut(root).insert(ColorState::default()); }
            app.update();
            assert!(errors.take_records().is_empty());
            assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
        }));
    }
}
