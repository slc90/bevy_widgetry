use crate::{
    TooltipError, TooltipFailure,
    error::{core, failure},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTooltipStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryTooltipStateColorOverrides {
    fn validate(&self, entity: Entity) -> Result<(), TooltipFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)
                .map_err(|error| core(error, entity, "popup.normal.background"))?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)
                .map_err(|error| core(error, entity, "popup.normal.border"))?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)
                .map_err(|error| core(error, entity, "popup.normal.foreground"))?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTooltipStateColors,
    ) -> bevy_widgetry_theme::WidgetryTooltipStateColors {
        bevy_widgetry_theme::WidgetryTooltipStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTooltipPopupColorOverrides {
    pub normal: WidgetryTooltipStateColorOverrides,
}
impl WidgetryTooltipPopupColorOverrides {
    fn validate(&self, entity: Entity) -> Result<(), TooltipFailure> {
        self.normal.validate(entity)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTooltipPopupColors,
    ) -> bevy_widgetry_theme::WidgetryTooltipPopupColors {
        bevy_widgetry_theme::WidgetryTooltipPopupColors {
            normal: self.normal.resolve(&theme.normal),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTooltipColorOverrides {
    pub popup: WidgetryTooltipPopupColorOverrides,
}
impl WidgetryTooltipColorOverrides {
    fn validate(&self, entity: Entity) -> Result<(), TooltipFailure> {
        self.popup.validate(entity)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTooltipColors,
    ) -> bevy_widgetry_theme::WidgetryTooltipColors {
        bevy_widgetry_theme::WidgetryTooltipColors {
            popup: self.popup.resolve(&theme.popup),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryTooltipColorOverrides);
impl WidgetryTooltipColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, TooltipFailure> {
        if world.get::<crate::WidgetryTooltip>(entity).is_none() {
            return Err(failure(
                TooltipError::InvalidTarget { entity },
                ErrorFrame::new("tooltip.colors"),
            ));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| {
                failure(
                    TooltipError::MissingColorState { entity },
                    ErrorFrame::new("tooltip.colors"),
                )
            })
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, TooltipFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTooltip>>(entity)
        {
            return Err(failure(
                TooltipError::ManagedColor {
                    entity,
                    owner: owner.entity,
                },
                ErrorFrame::new("tooltip.set_colors"),
            ));
        }
        colors.validate(entity).map_err(|error| {
            error.with_context(ErrorFrame::new("tooltip.set_colors").field("anchor", entity))
        })?;
        let mut state = world.get_mut::<ColorState>(entity).ok_or_else(|| {
            failure(
                TooltipError::MissingColorState { entity },
                ErrorFrame::new("tooltip.colors"),
            )
        })?;
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
                .map_err(TooltipFailure::into_bevy_error)
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, TooltipFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<ColorState, TooltipFailure> {
        self.validate(entity)?;
        Ok(ColorState(self))
    }
}
