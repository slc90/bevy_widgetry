use crate::{
    SplitterFailure,
    error::{core, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySplitterColorOverrides {
    pub normal: Option<Color>,
    pub hovered: Option<Color>,
    pub dragging: Option<Color>,
    pub disabled: Option<Color>,
}

#[derive(Component)]
pub(crate) struct ColorState(pub(crate) WidgetrySplitterColorOverrides);

impl WidgetrySplitterColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), SplitterFailure> {
        for (field, color) in [
            ("normal", self.normal),
            ("hovered", self.hovered),
            ("dragging", self.dragging),
            ("disabled", self.disabled),
        ] {
            let Some(color) = color else {
                continue;
            };
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("splitter.validate_colors")
                        .field("root", root)
                        .field("field", field),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, SplitterFailure> {
        self.validate(root)?;
        Ok(ColorState(self))
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySplitterColors,
    ) -> bevy_widgetry_theme::WidgetrySplitterColors {
        bevy_widgetry_theme::WidgetrySplitterColors {
            normal: self.normal.unwrap_or(theme.normal),
            hovered: self.hovered.unwrap_or(theme.hovered),
            dragging: self.dragging.unwrap_or(theme.dragging),
            disabled: self.disabled.unwrap_or(theme.disabled),
        }
    }
    pub fn get(world: &World, entity: Entity) -> Result<&Self, SplitterFailure> {
        crate::WidgetrySplitter::split_ratio(world, entity)?;
        world
            .get::<ColorState>(entity)
            .map(|s| &s.0)
            .ok_or_else(|| missing(Some(entity), entity, "ColorState"))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, SplitterFailure> {
        if Self::get(world, entity)? == &colors {
            return Ok(false);
        }
        colors.validate(entity)?;
        world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| missing(Some(entity), entity, "ColorState"))?
            .0 = colors;
        Ok(true)
    }
    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, entity, colors)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, SplitterFailure> {
        Self::set_in_world(world, entity, default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, default());
    }
}
