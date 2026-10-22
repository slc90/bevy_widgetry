use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTabStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}

impl WidgetryTabStateColorOverrides {
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
        base: &bevy_widgetry_theme::WidgetryTabStateColors,
    ) -> bevy_widgetry_theme::WidgetryTabStateColors {
        bevy_widgetry_theme::WidgetryTabStateColors {
            background: self.background.unwrap_or(base.background),
            border: self.border.unwrap_or(base.border),
            foreground: self.foreground.unwrap_or(base.foreground),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTabInteractionColorOverrides {
    pub normal: WidgetryTabStateColorOverrides,
    pub hovered: WidgetryTabStateColorOverrides,
    pub pressed: WidgetryTabStateColorOverrides,
    pub disabled: WidgetryTabStateColorOverrides,
}

impl WidgetryTabInteractionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        base: &bevy_widgetry_theme::WidgetryTabInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryTabInteractionColors {
        bevy_widgetry_theme::WidgetryTabInteractionColors {
            normal: self.normal.resolve(&base.normal),
            hovered: self.hovered.resolve(&base.hovered),
            pressed: self.pressed.resolve(&base.pressed),
            disabled: self.disabled.resolve(&base.disabled),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTabColorOverrides {
    pub unselected: WidgetryTabInteractionColorOverrides,
    pub selected: WidgetryTabInteractionColorOverrides,
    pub focus_border: Option<Color>,
}

impl WidgetryTabColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.unselected.validate()?;
        self.selected.validate()?;
        if let Some(color) = self.focus_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        base: &bevy_widgetry_theme::WidgetryTabColors,
    ) -> bevy_widgetry_theme::WidgetryTabColors {
        bevy_widgetry_theme::WidgetryTabColors {
            unselected: self.unselected.resolve(&base.unselected),
            selected: self.selected.resolve(&base.selected),
            focus_border: self.focus_border.unwrap_or(base.focus_border),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTabsColorOverrides {
    pub header: WidgetryTabColorOverrides,
    pub bar_background: Option<Color>,
    pub panel_background: Option<Color>,
    pub separator: Option<Color>,
}

impl WidgetryTabsColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.header.validate()?;
        if let Some(color) = self.bar_background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.panel_background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.separator {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        base: &bevy_widgetry_theme::WidgetryTabsColors,
    ) -> bevy_widgetry_theme::WidgetryTabsColors {
        bevy_widgetry_theme::WidgetryTabsColors {
            header: self.header.resolve(&base.header),
            bar_background: self.bar_background.unwrap_or(base.bar_background),
            panel_background: self.panel_background.unwrap_or(base.panel_background),
            separator: self.separator.unwrap_or(base.separator),
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryTabsColorOverrides);

impl WidgetryTabsColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryTabs>(entity).is_none() {
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTabs>>(entity)
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

#[derive(Component, Default)]
pub(crate) struct TabColorState(pub(crate) WidgetryTabColorOverrides);

impl WidgetryTabColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryTab>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<TabColorState>(entity)
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTab>>(entity)
            .is_some()
        {
            return Err(invalid_target(entity));
        }
        colors.validate()?;
        let mut state = world
            .get_mut::<TabColorState>(entity)
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
    pub(crate) fn initial(self) -> Result<TabColorState, BevyError> {
        self.validate()?;
        Ok(TabColorState(self))
    }
}

fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(?entity, "Tabs 颜色目标不存在、类型不匹配或属于托管部件");
    BevyError::error("Tabs color target is missing, invalid, or externally owned")
}
