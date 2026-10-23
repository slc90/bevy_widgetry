use bevy::prelude::*;

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
    fn validate(&self) -> Result<(), BevyError> {
        for color in [self.normal, self.hovered, self.dragging, self.disabled]
            .into_iter()
            .flatten()
        {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn initial(self) -> Result<ColorState, BevyError> {
        self.validate()
            .map_err(|e| bevy_widgetry_core::scene::logged_error(e.to_string()))?;
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
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        crate::WidgetrySplitter::split_ratio(world, entity)?;
        world
            .get::<ColorState>(entity)
            .map(|s| &s.0)
            .ok_or_else(|| crate::splitter::invalid("Splitter color state is missing"))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, BevyError> {
        if Self::get(world, entity)? == &colors {
            return Ok(false);
        }
        colors.validate()?;
        world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| crate::splitter::invalid("Splitter color state is missing"))?
            .0 = colors;
        Ok(true)
    }
    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands
            .queue(move |world: &mut World| Self::set_in_world(world, entity, colors).map(|_| ()));
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        Self::set_in_world(world, entity, default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, default());
    }
}
