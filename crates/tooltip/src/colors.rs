use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTooltipStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryTooltipStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
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
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
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
    fn validate(&self) -> Result<(), BevyError> {
        self.popup.validate()?;
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
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryTooltip>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| invalid_target(entity))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, BevyError> {
        Self::get(world, entity)?;
        if world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTooltip>>(entity)
            .is_some()
        {
            return Err(invalid_target(entity));
        }
        colors.validate()?;
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
        commands
            .queue(move |world: &mut World| Self::set_in_world(world, entity, colors).map(|_| ()));
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self) -> Result<ColorState, BevyError> {
        self.validate()?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "WidgetryTooltipColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryTooltipColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
