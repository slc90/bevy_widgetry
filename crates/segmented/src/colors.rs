use crate::{
    SegmentedError, SegmentedFailure,
    error::{core, failure},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySegmentedOptionStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetrySegmentedOptionStateColorOverrides {
    fn validate(
        &self,
        entity: Entity,
        selection: &'static str,
        interaction: &'static str,
    ) -> Result<(), SegmentedFailure> {
        for (field, color) in [
            ("background", self.background),
            ("foreground", self.foreground),
        ] {
            if let Some(color) = color {
                bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                    core(
                        error,
                        ErrorFrame::new("segmented.validate_colors")
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
    fn validate(&self, entity: Entity, selection: &'static str) -> Result<(), SegmentedFailure> {
        self.normal.validate(entity, selection, "normal")?;
        self.hovered.validate(entity, selection, "hovered")?;
        self.disabled.validate(entity, selection, "disabled")?;
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
    fn validate(&self, entity: Entity) -> Result<(), SegmentedFailure> {
        self.unchecked.validate(entity, "unchecked")?;
        self.checked.validate(entity, "checked")?;
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
    fn validate(&self, entity: Entity, state: &'static str) -> Result<(), SegmentedFailure> {
        for (field, color) in [("background", self.background), ("border", self.border)] {
            if let Some(color) = color {
                bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                    core(
                        error,
                        ErrorFrame::new("segmented.validate_colors")
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
    fn validate(&self, entity: Entity) -> Result<(), SegmentedFailure> {
        self.normal.validate(entity, "normal")?;
        self.focused.validate(entity, "focused")?;
        self.disabled.validate(entity, "disabled")?;
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
    fn validate(&self, entity: Entity) -> Result<(), SegmentedFailure> {
        self.container.validate(entity)?;
        self.option.validate(entity)?;
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
    pub fn get(world: &World, entity: Entity) -> Result<&Self, SegmentedFailure> {
        if world.get::<crate::WidgetrySegmented>(entity).is_none() {
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
    ) -> Result<bool, SegmentedFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetrySegmented>>(entity)
        {
            return Err(failure(
                SegmentedError::ManagedColors {
                    entity,
                    owner: owner.entity,
                },
                ErrorFrame::new("segmented.set_colors").field("group", entity),
            ));
        }
        colors.validate(entity).map_err(|error| {
            error.with_context(ErrorFrame::new("segmented.set_colors").field("group", entity))
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, SegmentedFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<ColorState, SegmentedFailure> {
        self.validate(entity).map_err(|error| {
            error.with_context(ErrorFrame::new("segmented.initial_colors").field("group", entity))
        })?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(entity: Entity) -> SegmentedFailure {
    failure(
        SegmentedError::InvalidTarget {
            entity,
            widget: "Segmented",
        },
        ErrorFrame::new("segmented.get_colors"),
    )
}
fn missing_state(entity: Entity, option: bool, group: Option<Entity>) -> SegmentedFailure {
    failure(
        SegmentedError::MissingPart {
            group,
            option: option.then_some(entity),
            entity,
            part: if option {
                "OptionColorState"
            } else {
                "ColorState"
            },
        },
        ErrorFrame::new("segmented.get_colors"),
    )
}

#[derive(Component, Default)]
pub(crate) struct OptionColorState(pub(crate) WidgetrySegmentedOptionColorOverrides);
impl WidgetrySegmentedOptionColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, SegmentedFailure> {
        if world
            .get::<crate::WidgetrySegmentedOption>(entity)
            .is_none()
        {
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
    ) -> Result<bool, SegmentedFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world.get::<bevy_widgetry_core::color::WidgetryStyleOwner<
            crate::WidgetrySegmentedOption,
        >>(entity)
        {
            return Err(failure(
                SegmentedError::ManagedColors {
                    entity,
                    owner: owner.entity,
                },
                ErrorFrame::new("segmented_option.set_colors").field("option", entity),
            ));
        }
        let group = world.get::<ChildOf>(entity).map(ChildOf::parent);
        colors.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("segmented_option.set_colors")
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, SegmentedFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<OptionColorState, SegmentedFailure> {
        self.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("segmented_option.initial_colors").field("option", entity),
            )
        })?;
        Ok(OptionColorState(self))
    }
}

fn invalid_option_target(entity: Entity) -> SegmentedFailure {
    failure(
        SegmentedError::InvalidTarget {
            entity,
            widget: "SegmentedOption",
        },
        ErrorFrame::new("segmented_option.get_colors"),
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn missing_private_color_states_keep_group_and_option_identity() {
        let mut world = World::new();
        let group = world.spawn(crate::WidgetrySegmented).id();
        let option = world
            .spawn((crate::WidgetrySegmentedOption, ChildOf(group)))
            .id();
        world.entity_mut(option).remove::<OptionColorState>();
        let failure =
            WidgetrySegmentedOptionColorOverrides::clear_in_world(&mut world, option).unwrap_err();
        assert!(
            matches!(failure.kind(), SegmentedError::MissingPart { group: Some(root), option: Some(target), entity, part: "OptionColorState" } if *root == group && *target == option && *entity == option)
        );
        world.entity_mut(group).remove::<ColorState>();
        let failure =
            WidgetrySegmentedColorOverrides::clear_in_world(&mut world, group).unwrap_err();
        assert!(
            matches!(failure.kind(), SegmentedError::MissingPart { group: Some(root), option: None, entity, part: "ColorState" } if *root == group && *entity == group)
        );
    }
}
