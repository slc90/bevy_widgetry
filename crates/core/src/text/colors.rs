use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextStateColorOverrides {
    pub foreground: Option<Color>,
}
impl WidgetryTextStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.foreground {
            crate::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextStateColors,
    ) -> bevy_widgetry_theme::WidgetryTextStateColors {
        bevy_widgetry_theme::WidgetryTextStateColors {
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextColorOverrides {
    pub normal: WidgetryTextStateColorOverrides,
    pub disabled: WidgetryTextStateColorOverrides,
}
impl WidgetryTextColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextColors,
    ) -> bevy_widgetry_theme::WidgetryTextColors {
        bevy_widgetry_theme::WidgetryTextColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryTextColorOverrides);
impl WidgetryTextColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<super::WidgetryText>(entity).is_none() {
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
            .get::<crate::color::WidgetryStyleOwner<super::WidgetryText>>(entity)
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
        "WidgetryTextColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryTextColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
