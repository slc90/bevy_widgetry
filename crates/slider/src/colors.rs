use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySliderStateColorOverrides {
    pub track: Option<Color>,
    pub fill: Option<Color>,
    pub thumb: Option<Color>,
    pub focus_ring: Option<Color>,
}
impl WidgetrySliderStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.track {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.fill {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.thumb {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.focus_ring {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySliderStateColors,
    ) -> bevy_widgetry_theme::WidgetrySliderStateColors {
        bevy_widgetry_theme::WidgetrySliderStateColors {
            track: self.track.unwrap_or(theme.track),
            fill: self.fill.unwrap_or(theme.fill),
            thumb: self.thumb.unwrap_or(theme.thumb),
            focus_ring: self.focus_ring.unwrap_or(theme.focus_ring),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySliderColorOverrides {
    pub normal: WidgetrySliderStateColorOverrides,
    pub hovered: WidgetrySliderStateColorOverrides,
    pub pressed: WidgetrySliderStateColorOverrides,
    pub disabled: WidgetrySliderStateColorOverrides,
}
impl WidgetrySliderColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySliderColors,
    ) -> bevy_widgetry_theme::WidgetrySliderColors {
        bevy_widgetry_theme::WidgetrySliderColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetrySliderColorOverrides);
impl WidgetrySliderColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetrySlider>(entity).is_none() {
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
        self.validate()
            .map_err(|error| bevy_widgetry_core::scene::logged_error(error.to_string()))?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "WidgetrySliderColorOverrides 目标不存在、类型不匹配或内部颜色状态缺失"
    );
    BevyError::error("WidgetrySliderColorOverrides 目标不存在、类型不匹配或内部颜色状态缺失")
}
