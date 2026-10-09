use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTreeExpanderStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryTreeExpanderStateColorOverrides {
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
        theme: &bevy_widgetry_theme::WidgetryTreeExpanderStateColors,
    ) -> bevy_widgetry_theme::WidgetryTreeExpanderStateColors {
        bevy_widgetry_theme::WidgetryTreeExpanderStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTreeExpanderInteractionColorOverrides {
    pub normal: WidgetryTreeExpanderStateColorOverrides,
    pub hovered: WidgetryTreeExpanderStateColorOverrides,
    pub pressed: WidgetryTreeExpanderStateColorOverrides,
    pub disabled: WidgetryTreeExpanderStateColorOverrides,
}
impl WidgetryTreeExpanderInteractionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTreeExpanderInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryTreeExpanderInteractionColors {
        bevy_widgetry_theme::WidgetryTreeExpanderInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTreeExpanderColorOverrides {
    pub collapsed: WidgetryTreeExpanderInteractionColorOverrides,
    pub expanded: WidgetryTreeExpanderInteractionColorOverrides,
}
impl WidgetryTreeExpanderColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.collapsed.validate()?;
        self.expanded.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTreeExpanderColors,
    ) -> bevy_widgetry_theme::WidgetryTreeExpanderColors {
        bevy_widgetry_theme::WidgetryTreeExpanderColors {
            collapsed: self.collapsed.resolve(&theme.collapsed),
            expanded: self.expanded.resolve(&theme.expanded),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewItemStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryListViewItemStateColorOverrides {
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
        theme: &bevy_widgetry_theme::WidgetryListViewItemStateColors,
    ) -> bevy_widgetry_theme::WidgetryListViewItemStateColors {
        bevy_widgetry_theme::WidgetryListViewItemStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewItemColorOverrides {
    pub normal: WidgetryListViewItemStateColorOverrides,
    pub hovered: WidgetryListViewItemStateColorOverrides,
    pub pressed: WidgetryListViewItemStateColorOverrides,
    pub selected: WidgetryListViewItemStateColorOverrides,
    pub disabled: WidgetryListViewItemStateColorOverrides,
    pub active_border: Option<Color>,
    pub disabled_active_border: Option<Color>,
}
impl WidgetryListViewItemColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.selected.validate()?;
        self.disabled.validate()?;
        if let Some(color) = self.active_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.disabled_active_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewItemColors,
    ) -> bevy_widgetry_theme::WidgetryListViewItemColors {
        bevy_widgetry_theme::WidgetryListViewItemColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            selected: self.selected.resolve(&theme.selected),
            disabled: self.disabled.resolve(&theme.disabled),
            active_border: self.active_border.unwrap_or(theme.active_border),
            disabled_active_border: self
                .disabled_active_border
                .unwrap_or(theme.disabled_active_border),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewContainerStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryListViewContainerStateColorOverrides {
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
        theme: &bevy_widgetry_theme::WidgetryListViewContainerStateColors,
    ) -> bevy_widgetry_theme::WidgetryListViewContainerStateColors {
        bevy_widgetry_theme::WidgetryListViewContainerStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewContainerColorOverrides {
    pub normal: WidgetryListViewContainerStateColorOverrides,
    pub focused: WidgetryListViewContainerStateColorOverrides,
    pub disabled: WidgetryListViewContainerStateColorOverrides,
}
impl WidgetryListViewContainerColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.focused.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewContainerColors,
    ) -> bevy_widgetry_theme::WidgetryListViewContainerColors {
        bevy_widgetry_theme::WidgetryListViewContainerColors {
            normal: self.normal.resolve(&theme.normal),
            focused: self.focused.resolve(&theme.focused),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTreeColorOverrides {
    pub container: WidgetryListViewContainerColorOverrides,
    pub item: WidgetryListViewItemColorOverrides,
    pub expander: WidgetryTreeExpanderColorOverrides,
}
impl WidgetryTreeColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.container.validate()?;
        self.item.validate()?;
        self.expander.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTreeColors,
    ) -> bevy_widgetry_theme::WidgetryTreeColors {
        bevy_widgetry_theme::WidgetryTreeColors {
            container: self.container.resolve(&theme.container),
            item: self.item.resolve(&theme.item),
            expander: self.expander.resolve(&theme.expander),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryTreeColorOverrides);
impl WidgetryTreeColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryTreeView>(entity).is_none() {
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
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTreeView>>(entity)
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
        "WidgetryTreeColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryTreeColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
