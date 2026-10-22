use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySegmentedOptionStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetrySegmentedOptionStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySegmentedOptionStateColors,
    ) -> bevy_widgetry_theme::WidgetrySegmentedOptionStateColors {
        bevy_widgetry_theme::WidgetrySegmentedOptionStateColors {
            background: self.background.unwrap_or(theme.background),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySegmentedOptionInteractionColorOverrides {
    pub normal: WidgetrySegmentedOptionStateColorOverrides,
    pub hovered: WidgetrySegmentedOptionStateColorOverrides,
    pub disabled: WidgetrySegmentedOptionStateColorOverrides,
}
impl WidgetrySegmentedOptionInteractionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySegmentedOptionInteractionColors,
    ) -> bevy_widgetry_theme::WidgetrySegmentedOptionInteractionColors {
        bevy_widgetry_theme::WidgetrySegmentedOptionInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySegmentedOptionColorOverrides {
    pub unchecked: WidgetrySegmentedOptionInteractionColorOverrides,
    pub checked: WidgetrySegmentedOptionInteractionColorOverrides,
}
impl WidgetrySegmentedOptionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.unchecked.validate()?;
        self.checked.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySegmentedOptionColors,
    ) -> bevy_widgetry_theme::WidgetrySegmentedOptionColors {
        bevy_widgetry_theme::WidgetrySegmentedOptionColors {
            unchecked: self.unchecked.resolve(&theme.unchecked),
            checked: self.checked.resolve(&theme.checked),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySegmentedContainerStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
}
impl WidgetrySegmentedContainerStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySegmentedContainerStateColors,
    ) -> bevy_widgetry_theme::WidgetrySegmentedContainerStateColors {
        bevy_widgetry_theme::WidgetrySegmentedContainerStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySegmentedContainerColorOverrides {
    pub normal: WidgetrySegmentedContainerStateColorOverrides,
    pub focused: WidgetrySegmentedContainerStateColorOverrides,
    pub disabled: WidgetrySegmentedContainerStateColorOverrides,
}
impl WidgetrySegmentedContainerColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.focused.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySegmentedContainerColors,
    ) -> bevy_widgetry_theme::WidgetrySegmentedContainerColors {
        bevy_widgetry_theme::WidgetrySegmentedContainerColors {
            normal: self.normal.resolve(&theme.normal),
            focused: self.focused.resolve(&theme.focused),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySegmentedColorOverrides {
    pub container: WidgetrySegmentedContainerColorOverrides,
    pub option: WidgetrySegmentedOptionColorOverrides,
}
impl WidgetrySegmentedColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.container.validate()?;
        self.option.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySegmentedColors,
    ) -> bevy_widgetry_theme::WidgetrySegmentedColors {
        bevy_widgetry_theme::WidgetrySegmentedColors {
            container: self.container.resolve(&theme.container),
            option: self.option.resolve(&theme.option),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetrySegmentedColorOverrides);
impl WidgetrySegmentedColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetrySegmented>(entity).is_none() {
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetrySegmented>>(entity)
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
        "WidgetrySegmentedColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetrySegmentedColorOverrides 目标不存在、类型不匹配或属于托管部件")
}

#[derive(Component, Default)]
pub(crate) struct OptionColorState(pub(crate) WidgetrySegmentedOptionColorOverrides);
impl WidgetrySegmentedOptionColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world
            .get::<crate::WidgetrySegmentedOption>(entity)
            .is_none()
        {
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetrySegmentedOption>>(
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
        "SegmentedOption 颜色目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("SegmentedOption 颜色目标不存在、类型不匹配或属于托管部件")
}
