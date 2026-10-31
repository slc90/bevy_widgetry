use crate::diagnostics::{CoreError, CoreFailure, failure};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextStateColorOverrides {
    pub foreground: Option<Color>,
}
impl WidgetryTextStateColorOverrides {
    fn validate(&self, entity: Entity, state: &'static str) -> Result<(), CoreFailure> {
        if let Some(color) = self.foreground {
            crate::color::validate_color(color).map_err(|error| {
                error.with_context(
                    ErrorFrame::new("validate Text colors")
                        .field("entity", entity)
                        .field("state", state),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextStateColors,
    ) -> bevy_widgetry_theme::WidgetryTextStateColors {
        bevy_widgetry_theme::WidgetryTextStateColors {
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextColorOverrides {
    pub normal: WidgetryTextStateColorOverrides,
    pub disabled: WidgetryTextStateColorOverrides,
}
impl WidgetryTextColorOverrides {
    fn validate(&self, entity: Entity) -> Result<(), CoreFailure> {
        self.normal.validate(entity, "normal")?;
        self.disabled.validate(entity, "disabled")?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextColors,
    ) -> bevy_widgetry_theme::WidgetryTextColors {
        bevy_widgetry_theme::WidgetryTextColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryTextColorOverrides);
impl WidgetryTextColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, CoreFailure> {
        if world.get::<super::WidgetryText>(entity).is_none() {
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
    ) -> Result<bool, CoreFailure> {
        Self::get(world, entity)?;
        if let Some(owner) =
            world.get::<crate::color::WidgetryStyleOwner<super::WidgetryText>>(entity)
        {
            return Err(failure(
                CoreError::ManagedColor {
                    entity,
                    owner: owner.entity,
                },
                "set Text colors",
            ));
        }
        colors.validate(entity)?;
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
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, entity, colors)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, CoreFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<ColorState, CoreFailure> {
        self.validate(entity)?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(entity: Entity) -> CoreFailure {
    failure(
        CoreError::InvalidTarget {
            entity,
            missing: "WidgetryText/ColorState",
        },
        "access Text colors",
    )
}
