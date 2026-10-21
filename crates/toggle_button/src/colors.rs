use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryToggleButtonStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryToggleButtonStateColorOverrides {
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
        theme: &bevy_widgetry_theme::WidgetryButtonStateColors,
    ) -> bevy_widgetry_theme::WidgetryButtonStateColors {
        bevy_widgetry_theme::WidgetryButtonStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryToggleButtonColorOverrides {
    pub normal: WidgetryToggleButtonStateColorOverrides,
    pub hovered: WidgetryToggleButtonStateColorOverrides,
    pub pressed: WidgetryToggleButtonStateColorOverrides,
    pub disabled: WidgetryToggleButtonStateColorOverrides,
    pub selected: WidgetryToggleButtonStateColorOverrides,
    pub selected_hovered: WidgetryToggleButtonStateColorOverrides,
    pub selected_pressed: WidgetryToggleButtonStateColorOverrides,
    pub selected_disabled: WidgetryToggleButtonStateColorOverrides,
}
impl WidgetryToggleButtonColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        self.selected.validate()?;
        self.selected_hovered.validate()?;
        self.selected_pressed.validate()?;
        self.selected_disabled.validate()?;
        Ok(())
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryToggleButtonColorOverrides);
impl WidgetryToggleButtonColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryToggleButton>(entity).is_none() {
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryToggleButton>>(
                entity,
            )
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
        "WidgetryToggleButtonColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryToggleButtonColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
