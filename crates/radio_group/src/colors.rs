use crate::{
    RadioGroupError, RadioGroupFailure,
    error::{core, failure},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryRadioOptionStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub dot: Option<Color>,
}
impl WidgetryRadioOptionStateColorOverrides {
    fn validate(
        &self,
        entity: Entity,
        selection: &'static str,
        interaction: &'static str,
    ) -> Result<(), RadioGroupFailure> {
        for (field, color) in [
            ("background", self.background),
            ("border", self.border),
            ("foreground", self.foreground),
            ("dot", self.dot),
        ] {
            if let Some(color) = color {
                bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                    core(
                        error,
                        ErrorFrame::new("radio.validate_colors")
                            .field("entity", entity)
                            .field("state", format_args!("option.{selection}.{interaction}"))
                            .field("field", field),
                    )
                })?;
            }
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
    fn validate(&self, entity: Entity, selection: &'static str) -> Result<(), RadioGroupFailure> {
        self.normal.validate(entity, selection, "normal")?;
        self.hovered.validate(entity, selection, "hovered")?;
        self.disabled.validate(entity, selection, "disabled")?;
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
    fn validate(&self, entity: Entity) -> Result<(), RadioGroupFailure> {
        self.unchecked.validate(entity, "unchecked")?;
        self.checked.validate(entity, "checked")?;
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
    fn validate(&self, entity: Entity, state: &'static str) -> Result<(), RadioGroupFailure> {
        for (field, color) in [
            ("background", self.background),
            ("border", self.border),
            ("foreground", self.foreground),
        ] {
            if let Some(color) = color {
                bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                    core(
                        error,
                        ErrorFrame::new("radio.validate_colors")
                            .field("entity", entity)
                            .field("state", format_args!("container.{state}"))
                            .field("field", field),
                    )
                })?;
            }
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
    fn validate(&self, entity: Entity) -> Result<(), RadioGroupFailure> {
        self.normal.validate(entity, "normal")?;
        self.focused.validate(entity, "focused")?;
        self.disabled.validate(entity, "disabled")?;
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
    fn validate(&self, entity: Entity) -> Result<(), RadioGroupFailure> {
        self.container.validate(entity)?;
        self.option.validate(entity)?;
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
    pub fn get(world: &World, entity: Entity) -> Result<&Self, RadioGroupFailure> {
        if world.get::<crate::WidgetryRadioGroup>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| missing_state(entity, false, Some(entity)))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, RadioGroupFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryRadioGroup>>(entity)
        {
            return Err(failure(
                RadioGroupError::ManagedColors {
                    entity,
                    owner: owner.entity,
                },
                ErrorFrame::new("radio_group.set_colors").field("group", entity),
            ));
        }
        colors.validate(entity).map_err(|error| {
            error.with_context(ErrorFrame::new("radio_group.set_colors").field("group", entity))
        })?;
        let mut state = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| missing_state(entity, false, Some(entity)))?;
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, RadioGroupFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<ColorState, RadioGroupFailure> {
        self.validate(entity).map_err(|error| {
            error.with_context(ErrorFrame::new("radio_group.initial_colors").field("group", entity))
        })?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(entity: Entity) -> RadioGroupFailure {
    failure(
        RadioGroupError::InvalidTarget {
            entity,
            widget: "RadioGroup",
        },
        ErrorFrame::new("radio_group.get_colors"),
    )
}
fn missing_state(entity: Entity, option: bool, group: Option<Entity>) -> RadioGroupFailure {
    failure(
        RadioGroupError::MissingPart {
            group,
            option: option.then_some(entity),
            entity,
            part: if option {
                "OptionColorState"
            } else {
                "ColorState"
            },
        },
        ErrorFrame::new("radio.get_colors"),
    )
}

#[derive(Component, Default)]
pub(crate) struct OptionColorState(pub(crate) WidgetryRadioOptionColorOverrides);
impl WidgetryRadioOptionColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, RadioGroupFailure> {
        if world.get::<crate::WidgetryRadioOption>(entity).is_none() {
            return Err(invalid_option_target(entity));
        }
        world
            .get::<OptionColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| {
                missing_state(
                    entity,
                    true,
                    world.get::<ChildOf>(entity).map(ChildOf::parent),
                )
            })
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, RadioGroupFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world.get::<bevy_widgetry_core::color::WidgetryStyleOwner<
            crate::WidgetryRadioOption,
        >>(entity)
        {
            return Err(failure(
                RadioGroupError::ManagedColors {
                    entity,
                    owner: owner.entity,
                },
                ErrorFrame::new("radio_option.set_colors").field("option", entity),
            ));
        }
        let group = world.get::<ChildOf>(entity).map(ChildOf::parent);
        colors.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("radio_option.set_colors")
                    .field("option", entity)
                    .field("group", format_args!("{group:?}")),
            )
        })?;
        let mut state = world
            .get_mut::<OptionColorState>(entity)
            .ok_or_else(|| missing_state(entity, true, group))?;
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, RadioGroupFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<OptionColorState, RadioGroupFailure> {
        self.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("radio_option.initial_colors").field("option", entity),
            )
        })?;
        Ok(OptionColorState(self))
    }
}

fn invalid_option_target(entity: Entity) -> RadioGroupFailure {
    failure(
        RadioGroupError::InvalidTarget {
            entity,
            widget: "RadioOption",
        },
        ErrorFrame::new("radio_option.get_colors"),
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn missing_private_color_states_keep_group_and_option_identity() {
        let mut world = World::new();
        let group = world.spawn(crate::WidgetryRadioGroup).id();
        let option = world
            .spawn((crate::WidgetryRadioOption, ChildOf(group)))
            .id();
        world.entity_mut(option).remove::<OptionColorState>();
        let failure =
            WidgetryRadioOptionColorOverrides::clear_in_world(&mut world, option).unwrap_err();
        assert!(
            matches!(failure.kind(), RadioGroupError::MissingPart { group: Some(root), option: Some(target), entity, part: "OptionColorState" } if *root == group && *target == option && *entity == option)
        );
        world.entity_mut(group).remove::<ColorState>();
        let failure =
            WidgetryRadioGroupColorOverrides::clear_in_world(&mut world, group).unwrap_err();
        assert!(
            matches!(failure.kind(), RadioGroupError::MissingPart { group: Some(root), option: None, entity, part: "ColorState" } if *root == group && *entity == group)
        );
    }
}
