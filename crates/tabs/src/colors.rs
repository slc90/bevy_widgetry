use crate::{
    TabsError, TabsFailure,
    error::{core, failure, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTabStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}

impl WidgetryTabStateColorOverrides {
    fn validate(&self) -> Result<(), TabsFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("validate widget colors")
                        .field("consumer", "tabs")
                        .field("field", "background"),
                )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("validate widget colors")
                        .field("consumer", "tabs")
                        .field("field", "border"),
                )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("validate widget colors")
                        .field("consumer", "tabs")
                        .field("field", "foreground"),
                )
            })?;
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
    fn validate(&self) -> Result<(), TabsFailure> {
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
    fn validate(&self) -> Result<(), TabsFailure> {
        self.unselected.validate()?;
        self.selected.validate()?;
        if let Some(color) = self.focus_border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("validate widget colors")
                        .field("consumer", "tabs")
                        .field("field", "focus_border"),
                )
            })?;
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
    fn validate(&self) -> Result<(), TabsFailure> {
        self.header.validate()?;
        if let Some(color) = self.bar_background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("validate widget colors")
                        .field("consumer", "tabs")
                        .field("field", "bar_background"),
                )
            })?;
        }
        if let Some(color) = self.panel_background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("validate widget colors")
                        .field("consumer", "tabs")
                        .field("field", "panel_background"),
                )
            })?;
        }
        if let Some(color) = self.separator {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("validate widget colors")
                        .field("consumer", "tabs")
                        .field("field", "separator"),
                )
            })?;
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
    pub fn get(world: &World, entity: Entity) -> Result<&Self, TabsFailure> {
        if world.get::<crate::WidgetryTabs>(entity).is_none() {
            return Err(invalid_target(entity, "WidgetryTabs"));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| missing(entity, entity, "ColorState"))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, TabsFailure> {
        Self::get(world, entity)?;
        if world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTabs>>(entity)
            .is_some()
        {
            return Err(invalid_target(entity, "WidgetryTabs"));
        }
        colors.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("tabs.set_colors").field("entity", entity))
        })?;
        let mut state = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| missing(entity, entity, "ColorState"))?;
        if state.0 == colors {
            return Ok(false);
        }
        state.0 = colors;
        Ok(true)
    }
    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, entity, colors)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, TabsFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<ColorState, TabsFailure> {
        self.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("tabs.initialize_colors").field("entity", entity))
        })?;
        Ok(ColorState(self))
    }
}

#[derive(Component, Default)]
pub(crate) struct TabColorState(pub(crate) WidgetryTabColorOverrides);

impl WidgetryTabColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, TabsFailure> {
        if world.get::<crate::WidgetryTab>(entity).is_none() {
            return Err(invalid_target(entity, "WidgetryTab"));
        }
        world
            .get::<TabColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| missing(entity, entity, "TabColorState"))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, TabsFailure> {
        Self::get(world, entity)?;
        if world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTab>>(entity)
            .is_some()
        {
            return Err(invalid_target(entity, "WidgetryTab"));
        }
        colors.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("tabs.set_colors").field("entity", entity))
        })?;
        let mut state = world
            .get_mut::<TabColorState>(entity)
            .ok_or_else(|| missing(entity, entity, "TabColorState"))?;
        if state.0 == colors {
            return Ok(false);
        }
        state.0 = colors;
        Ok(true)
    }
    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, entity, colors)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, TabsFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<TabColorState, TabsFailure> {
        self.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("tabs.initialize_colors").field("entity", entity))
        })?;
        Ok(TabColorState(self))
    }
}

fn invalid_target(entity: Entity, expected: &'static str) -> TabsFailure {
    failure(
        TabsError::InvalidTarget { entity, expected },
        ErrorFrame::new("tabs.color_target"),
    )
}
