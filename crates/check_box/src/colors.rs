use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub mark: Option<Color>,
}
impl WidgetryCheckBoxStateColorOverrides {
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
        if let Some(color) = self.mark {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxStateColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxStateColors {
        bevy_widgetry_theme::WidgetryCheckBoxStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            mark: self.mark.unwrap_or(theme.mark),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxInteractionColorOverrides {
    pub normal: WidgetryCheckBoxStateColorOverrides,
    pub hovered: WidgetryCheckBoxStateColorOverrides,
    pub pressed: WidgetryCheckBoxStateColorOverrides,
    pub disabled: WidgetryCheckBoxStateColorOverrides,
}
impl WidgetryCheckBoxInteractionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxInteractionColors {
        bevy_widgetry_theme::WidgetryCheckBoxInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxColorOverrides {
    pub unchecked: WidgetryCheckBoxInteractionColorOverrides,
    pub checked: WidgetryCheckBoxInteractionColorOverrides,
    pub indeterminate: WidgetryCheckBoxInteractionColorOverrides,
}
impl WidgetryCheckBoxColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.unchecked.validate()?;
        self.checked.validate()?;
        self.indeterminate.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxColors {
        bevy_widgetry_theme::WidgetryCheckBoxColors {
            unchecked: self.unchecked.resolve(&theme.unchecked),
            checked: self.checked.resolve(&theme.checked),
            indeterminate: self.indeterminate.resolve(&theme.indeterminate),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryCheckBoxColorOverrides);
impl WidgetryCheckBoxColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryCheckBox>(entity).is_none()
            && world
                .get::<crate::WidgetryTriStateCheckbox>(entity)
                .is_none()
        {
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryCheckBox>>(entity)
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
        "WidgetryCheckBoxColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryCheckBoxColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
