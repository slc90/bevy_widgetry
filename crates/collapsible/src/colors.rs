use bevy::prelude::*;
use bevy_widgetry_theme::WidgetryCollapsibleColors;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCollapsibleColorOverrides {
    pub background: Option<Color>,
    pub header_background: Option<Color>,
    pub content_background: Option<Color>,
    pub border: Option<Color>,
    pub separator: Option<Color>,
}

#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryCollapsibleColorOverrides);

impl WidgetryCollapsibleColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        for color in [
            self.background,
            self.header_background,
            self.content_background,
            self.border,
            self.separator,
        ]
        .into_iter()
        .flatten()
        {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }

    pub(crate) fn initial(self) -> Result<ColorState, BevyError> {
        self.validate()?;
        Ok(ColorState(self))
    }

    pub(crate) fn resolve(&self, theme: &WidgetryCollapsibleColors) -> WidgetryCollapsibleColors {
        WidgetryCollapsibleColors {
            background: self.background.unwrap_or(theme.background),
            header_background: self.header_background.unwrap_or(theme.header_background),
            content_background: self.content_background.unwrap_or(theme.content_background),
            border: self.border.unwrap_or(theme.border),
            separator: self.separator.unwrap_or(theme.separator),
        }
    }

    pub fn get(world: &World, root: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryCollapsible>(root).is_none() {
            return Err(crate::state::invalid("Collapsible color target is invalid"));
        }
        world
            .get::<ColorState>(root)
            .map(|state| &state.0)
            .ok_or_else(|| crate::state::invalid("Collapsible color state is missing"))
    }

    pub fn set_in_world(world: &mut World, root: Entity, colors: Self) -> Result<bool, BevyError> {
        Self::get(world, root)?;
        colors.validate()?;
        let mut current = world
            .get_mut::<ColorState>(root)
            .ok_or_else(|| crate::state::invalid("Collapsible color state is missing"))?;
        if current.0 == colors {
            return Ok(false);
        }
        current.0 = colors;
        Ok(true)
    }

    pub fn set(commands: &mut Commands, root: Entity, colors: Self) {
        commands
            .queue(move |world: &mut World| Self::set_in_world(world, root, colors).map(|_| ()));
    }

    pub fn clear_in_world(world: &mut World, root: Entity) -> Result<bool, BevyError> {
        Self::set_in_world(world, root, default())
    }
    pub fn clear(commands: &mut Commands, root: Entity) {
        Self::set(commands, root, default());
    }
}
