use crate::{
    ProgressBarError, ProgressBarFailure,
    error::{core, failure, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_theme::WidgetryProgressBarColors;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryProgressBarColorOverrides {
    pub track: Option<Color>,
    pub fill: Option<Color>,
    pub border: Option<Color>,
}

#[derive(Component)]
pub(crate) struct ColorState(pub(crate) WidgetryProgressBarColorOverrides);

impl WidgetryProgressBarColorOverrides {
    fn validate(&self) -> Result<(), ProgressBarFailure> {
        for (field, color) in [
            ("track", self.track),
            ("fill", self.fill),
            ("border", self.border),
        ] {
            let Some(color) = color else {
                continue;
            };
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("progress_bar.validate_colors").field("field", field),
                )
            })?;
        }
        Ok(())
    }

    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, ProgressBarFailure> {
        self.validate().map_err(|error| {
            error
                .with_context(ErrorFrame::new("progress_bar.initialize_colors").field("root", root))
        })?;
        Ok(ColorState(self))
    }

    pub(crate) fn resolve(&self, theme: &WidgetryProgressBarColors) -> WidgetryProgressBarColors {
        WidgetryProgressBarColors {
            track: self.track.unwrap_or(theme.track),
            fill: self.fill.unwrap_or(theme.fill),
            border: self.border.unwrap_or(theme.border),
        }
    }

    pub fn get(world: &World, entity: Entity) -> Result<&Self, ProgressBarFailure> {
        if world.get::<crate::WidgetryProgressBar>(entity).is_none() {
            return Err(failure(
                ProgressBarError::InvalidRoot { root: entity },
                ErrorFrame::new("progress_bar.color_target"),
            ));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| missing(entity, "ColorState"))
    }

    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, ProgressBarFailure> {
        Self::get(world, entity)?;
        colors.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("progress_bar.set_colors").field("root", entity))
        })?;
        let mut current = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| missing(entity, "ColorState"))?;
        if current.0 == colors {
            return Ok(false);
        }
        current.0 = colors;
        Ok(true)
    }

    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, entity, colors)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }

    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, ProgressBarFailure> {
        Self::set_in_world(world, entity, default())
    }

    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, default());
    }
}
