use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub caret: Option<Color>,
    pub selection_background: Option<Color>,
    pub unfocused_selection_background: Option<Color>,
    pub selection_foreground: Option<Color>,
}
impl WidgetryTextFieldStateColorOverrides {
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
        if let Some(color) = self.caret {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.selection_background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.unfocused_selection_background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.selection_foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldStateColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldStateColors {
        bevy_widgetry_theme::WidgetryTextFieldStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            caret: self.caret.unwrap_or(theme.caret),
            selection_background: self
                .selection_background
                .unwrap_or(theme.selection_background),
            unfocused_selection_background: self
                .unfocused_selection_background
                .unwrap_or(theme.unfocused_selection_background),
            selection_foreground: self
                .selection_foreground
                .unwrap_or(theme.selection_foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldInteractionColorOverrides {
    pub normal: WidgetryTextFieldStateColorOverrides,
    pub hovered: WidgetryTextFieldStateColorOverrides,
    pub focused: WidgetryTextFieldStateColorOverrides,
    pub disabled: WidgetryTextFieldStateColorOverrides,
}
impl WidgetryTextFieldInteractionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.focused.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldInteractionColors {
        bevy_widgetry_theme::WidgetryTextFieldInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            focused: self.focused.resolve(&theme.focused),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldColorOverrides {
    pub editable: WidgetryTextFieldInteractionColorOverrides,
    pub read_only: WidgetryTextFieldInteractionColorOverrides,
}
impl WidgetryTextFieldColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.editable.validate()?;
        self.read_only.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldColors {
        bevy_widgetry_theme::WidgetryTextFieldColors {
            editable: self.editable.resolve(&theme.editable),
            read_only: self.read_only.resolve(&theme.read_only),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryTextFieldColorOverrides);
impl WidgetryTextFieldColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryTextField>(entity).is_none()
            && world
                .get::<crate::WidgetryReadOnlyTextField>(entity)
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTextField>>(entity)
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
        "WidgetryTextFieldColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryTextFieldColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
