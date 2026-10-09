use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowButtonStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryWindowButtonStateColorOverrides {
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
        theme: &bevy_widgetry_theme::WidgetryWindowButtonStateColors,
    ) -> bevy_widgetry_theme::WidgetryWindowButtonStateColors {
        bevy_widgetry_theme::WidgetryWindowButtonStateColors {
            background: self.background.unwrap_or(theme.background),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowButtonColorOverrides {
    pub normal: WidgetryWindowButtonStateColorOverrides,
    pub hovered: WidgetryWindowButtonStateColorOverrides,
    pub pressed: WidgetryWindowButtonStateColorOverrides,
    pub disabled: WidgetryWindowButtonStateColorOverrides,
}
impl WidgetryWindowButtonColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowButtonColors,
    ) -> bevy_widgetry_theme::WidgetryWindowButtonColors {
        bevy_widgetry_theme::WidgetryWindowButtonColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryWindowStateColorOverrides {
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
        theme: &bevy_widgetry_theme::WidgetryWindowStateColors,
    ) -> bevy_widgetry_theme::WidgetryWindowStateColors {
        bevy_widgetry_theme::WidgetryWindowStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowSurfaceColorOverrides {
    pub normal: WidgetryWindowStateColorOverrides,
    pub disabled: WidgetryWindowStateColorOverrides,
}
impl WidgetryWindowSurfaceColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowSurfaceColors,
    ) -> bevy_widgetry_theme::WidgetryWindowSurfaceColors {
        bevy_widgetry_theme::WidgetryWindowSurfaceColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowColorOverrides {
    pub frame: WidgetryWindowSurfaceColorOverrides,
    pub title_bar: WidgetryWindowSurfaceColorOverrides,
    pub minimize: WidgetryWindowButtonColorOverrides,
    pub maximize: WidgetryWindowButtonColorOverrides,
    pub close: WidgetryWindowButtonColorOverrides,
    pub image_tint: Option<Color>,
}
impl WidgetryWindowColorOverrides {
    pub(crate) fn validate(&self) -> Result<(), BevyError> {
        self.frame.validate()?;
        self.title_bar.validate()?;
        self.minimize.validate()?;
        self.maximize.validate()?;
        self.close.validate()?;
        if let Some(color) = self.image_tint {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowColors,
    ) -> bevy_widgetry_theme::WidgetryWindowColors {
        bevy_widgetry_theme::WidgetryWindowColors {
            frame: self.frame.resolve(&theme.frame),
            title_bar: self.title_bar.resolve(&theme.title_bar),
            minimize: self.minimize.resolve(&theme.minimize),
            maximize: self.maximize.resolve(&theme.maximize),
            close: self.close.resolve(&theme.close),
            image_tint: self.image_tint.unwrap_or(theme.image_tint),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryWindowColorOverrides);
impl WidgetryWindowColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world
            .get::<crate::window_root::WindowRoot>(entity)
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::window_root::WindowRoot>>(
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
        "WidgetryWindowColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryWindowColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
