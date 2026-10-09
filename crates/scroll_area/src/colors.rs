use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollThumbStateColorOverrides {
    pub background: Option<Color>,
}
impl WidgetryScrollThumbStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollThumbStateColors,
    ) -> bevy_widgetry_theme::WidgetryScrollThumbStateColors {
        bevy_widgetry_theme::WidgetryScrollThumbStateColors {
            background: self.background.unwrap_or(theme.background),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollThumbColorOverrides {
    pub normal: WidgetryScrollThumbStateColorOverrides,
    pub hovered: WidgetryScrollThumbStateColorOverrides,
    pub dragged: WidgetryScrollThumbStateColorOverrides,
    pub disabled: WidgetryScrollThumbStateColorOverrides,
}
impl WidgetryScrollThumbColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.dragged.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollThumbColors,
    ) -> bevy_widgetry_theme::WidgetryScrollThumbColors {
        bevy_widgetry_theme::WidgetryScrollThumbColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            dragged: self.dragged.resolve(&theme.dragged),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollTrackStateColorOverrides {
    pub background: Option<Color>,
}
impl WidgetryScrollTrackStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollTrackStateColors,
    ) -> bevy_widgetry_theme::WidgetryScrollTrackStateColors {
        bevy_widgetry_theme::WidgetryScrollTrackStateColors {
            background: self.background.unwrap_or(theme.background),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollTrackColorOverrides {
    pub normal: WidgetryScrollTrackStateColorOverrides,
    pub disabled: WidgetryScrollTrackStateColorOverrides,
}
impl WidgetryScrollTrackColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollTrackColors,
    ) -> bevy_widgetry_theme::WidgetryScrollTrackColors {
        bevy_widgetry_theme::WidgetryScrollTrackColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollAxisColorOverrides {
    pub track: WidgetryScrollTrackColorOverrides,
    pub thumb: WidgetryScrollThumbColorOverrides,
}
impl WidgetryScrollAxisColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.track.validate()?;
        self.thumb.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollAxisColors,
    ) -> bevy_widgetry_theme::WidgetryScrollAxisColors {
        bevy_widgetry_theme::WidgetryScrollAxisColors {
            track: self.track.resolve(&theme.track),
            thumb: self.thumb.resolve(&theme.thumb),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollAreaColorOverrides {
    pub horizontal: WidgetryScrollAxisColorOverrides,
    pub vertical: WidgetryScrollAxisColorOverrides,
}
impl WidgetryScrollAreaColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.horizontal.validate()?;
        self.vertical.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollAreaColors,
    ) -> bevy_widgetry_theme::WidgetryScrollAreaColors {
        bevy_widgetry_theme::WidgetryScrollAreaColors {
            horizontal: self.horizontal.resolve(&theme.horizontal),
            vertical: self.vertical.resolve(&theme.vertical),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryScrollAreaColorOverrides);
impl WidgetryScrollAreaColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryScrollArea>(entity).is_none() {
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryScrollArea>>(entity)
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
        "WidgetryScrollAreaColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryScrollAreaColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
