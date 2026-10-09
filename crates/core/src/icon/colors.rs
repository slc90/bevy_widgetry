use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryIconStateColorOverrides {
    pub foreground: Option<Color>,
}
impl WidgetryIconStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.foreground {
            crate::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryIconStateColors,
    ) -> bevy_widgetry_theme::WidgetryIconStateColors {
        bevy_widgetry_theme::WidgetryIconStateColors {
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryIconColorOverrides {
    pub normal: WidgetryIconStateColorOverrides,
    pub disabled: WidgetryIconStateColorOverrides,
}
impl WidgetryIconColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryIconColors,
    ) -> bevy_widgetry_theme::WidgetryIconColors {
        bevy_widgetry_theme::WidgetryIconColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryIconColorOverrides);
impl WidgetryIconColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<super::WidgetryIcon>(entity).is_none() {
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
            .get::<crate::color::WidgetryStyleOwner<super::WidgetryIcon>>(entity)
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
        "WidgetryIconColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryIconColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
