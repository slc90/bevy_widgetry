use crate::{
    CollapsibleFailure, CollapsibleStructureError as Problem,
    error::{core, missing, structural},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
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
    fn validate(&self) -> Result<(), CollapsibleFailure> {
        for (field, color) in [
            ("background", self.background),
            ("header_background", self.header_background),
            ("content_background", self.content_background),
            ("border", self.border),
            ("separator", self.separator),
        ] {
            let Some(color) = color else {
                continue;
            };
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("validate widget colors")
                        .field("consumer", "collapsible")
                        .field("field", field),
                )
            })?;
        }
        Ok(())
    }

    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, CollapsibleFailure> {
        self.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("collapsible.initialize_colors").field("root", root))
        })?;
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

    pub fn get(world: &World, root: Entity) -> Result<&Self, CollapsibleFailure> {
        if world.get::<crate::WidgetryCollapsible>(root).is_none() {
            return Err(structural(
                Problem::InvalidRoot { root },
                ErrorFrame::new("collapsible.color_target"),
            ));
        }
        world
            .get::<ColorState>(root)
            .map(|state| &state.0)
            .ok_or_else(|| missing(root, root, "ColorState"))
    }

    pub fn set_in_world(
        world: &mut World,
        root: Entity,
        colors: Self,
    ) -> Result<bool, CollapsibleFailure> {
        Self::get(world, root)?;
        colors.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("collapsible.set_colors").field("root", root))
        })?;
        let mut current = world
            .get_mut::<ColorState>(root)
            .ok_or_else(|| missing(root, root, "ColorState"))?;
        if current.0 == colors {
            return Ok(false);
        }
        current.0 = colors;
        Ok(true)
    }

    pub fn set(commands: &mut Commands, root: Entity, colors: Self) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, root, colors)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }

    pub fn clear_in_world(world: &mut World, root: Entity) -> Result<bool, CollapsibleFailure> {
        Self::set_in_world(world, root, default())
    }
    pub fn clear(commands: &mut Commands, root: Entity) {
        Self::set(commands, root, default());
    }
}
