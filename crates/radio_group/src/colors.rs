use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryRadioOptionStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub dot: Option<Color>,
}
impl WidgetryRadioOptionStateColorOverrides {
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
        if let Some(color) = self.dot {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryRadioOptionStateColors,
    ) -> bevy_widgetry_theme::WidgetryRadioOptionStateColors {
        bevy_widgetry_theme::WidgetryRadioOptionStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            dot: self.dot.unwrap_or(theme.dot),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryRadioOptionInteractionColorOverrides {
    pub normal: WidgetryRadioOptionStateColorOverrides,
    pub hovered: WidgetryRadioOptionStateColorOverrides,
    pub disabled: WidgetryRadioOptionStateColorOverrides,
}
impl WidgetryRadioOptionInteractionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryRadioOptionInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryRadioOptionInteractionColors {
        bevy_widgetry_theme::WidgetryRadioOptionInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryRadioOptionColorOverrides {
    pub unchecked: WidgetryRadioOptionInteractionColorOverrides,
    pub checked: WidgetryRadioOptionInteractionColorOverrides,
}
impl WidgetryRadioOptionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.unchecked.validate()?;
        self.checked.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryRadioOptionColors,
    ) -> bevy_widgetry_theme::WidgetryRadioOptionColors {
        bevy_widgetry_theme::WidgetryRadioOptionColors {
            unchecked: self.unchecked.resolve(&theme.unchecked),
            checked: self.checked.resolve(&theme.checked),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryRadioGroupContainerStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryRadioGroupContainerStateColorOverrides {
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
        theme: &bevy_widgetry_theme::WidgetryRadioGroupContainerStateColors,
    ) -> bevy_widgetry_theme::WidgetryRadioGroupContainerStateColors {
        bevy_widgetry_theme::WidgetryRadioGroupContainerStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryRadioGroupContainerColorOverrides {
    pub normal: WidgetryRadioGroupContainerStateColorOverrides,
    pub focused: WidgetryRadioGroupContainerStateColorOverrides,
    pub disabled: WidgetryRadioGroupContainerStateColorOverrides,
}
impl WidgetryRadioGroupContainerColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.focused.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryRadioGroupContainerColors,
    ) -> bevy_widgetry_theme::WidgetryRadioGroupContainerColors {
        bevy_widgetry_theme::WidgetryRadioGroupContainerColors {
            normal: self.normal.resolve(&theme.normal),
            focused: self.focused.resolve(&theme.focused),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryRadioGroupColorOverrides {
    pub container: WidgetryRadioGroupContainerColorOverrides,
    pub option: WidgetryRadioOptionColorOverrides,
}
impl WidgetryRadioGroupColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.container.validate()?;
        self.option.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryRadioGroupColors,
    ) -> bevy_widgetry_theme::WidgetryRadioGroupColors {
        bevy_widgetry_theme::WidgetryRadioGroupColors {
            container: self.container.resolve(&theme.container),
            option: self.option.resolve(&theme.option),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryRadioGroupColorOverrides);
impl WidgetryRadioGroupColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryRadioGroup>(entity).is_none() {
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryRadioGroup>>(entity)
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
        "WidgetryRadioGroupColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryRadioGroupColorOverrides 目标不存在、类型不匹配或属于托管部件")
}

#[derive(Component, Default)]
pub(crate) struct OptionColorState(pub(crate) WidgetryRadioOptionColorOverrides);
impl WidgetryRadioOptionColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryRadioOption>(entity).is_none() {
            return Err(invalid_option_target(entity));
        }
        world
            .get::<OptionColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| invalid_option_target(entity))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, BevyError> {
        Self::get(world, entity)?;
        if world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryRadioOption>>(
                entity,
            )
            .is_some()
        {
            return Err(invalid_option_target(entity));
        }
        colors.validate()?;
        let mut state = world
            .get_mut::<OptionColorState>(entity)
            .ok_or_else(|| invalid_option_target(entity))?;
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
    pub(crate) fn initial(self) -> Result<OptionColorState, BevyError> {
        self.validate()?;
        Ok(OptionColorState(self))
    }
}

fn invalid_option_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "RadioOption 颜色目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("RadioOption 颜色目标不存在、类型不匹配或属于托管部件")
}
