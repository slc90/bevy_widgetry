use super::{IconError, IconFailure, error::failure};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::bevy::into_bevy_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryIconStateColorOverrides {
    pub foreground: Option<Color>,
}
impl WidgetryIconStateColorOverrides {
    fn validate(&self, entity: Option<Entity>, state: &'static str) -> Result<(), IconFailure> {
        if let Some(color) = self.foreground {
            let rgba = color.to_linear();
            if ![rgba.red, rgba.green, rgba.blue, rgba.alpha]
                .iter()
                .all(|value| value.is_finite())
                || !(0.0..=1.0).contains(&rgba.alpha)
            {
                return Err(failure(
                    IconError::InvalidColor {
                        entity,
                        state,
                        color,
                    },
                    "validate icon color",
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryIconStateColors,
    ) -> bevy_widgetry_theme::WidgetryIconStateColors {
        bevy_widgetry_theme::WidgetryIconStateColors {
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryIconColorOverrides {
    pub normal: WidgetryIconStateColorOverrides,
    pub disabled: WidgetryIconStateColorOverrides,
}
impl WidgetryIconColorOverrides {
    fn validate(&self, entity: Option<Entity>) -> Result<(), IconFailure> {
        self.normal.validate(entity, "normal")?;
        self.disabled.validate(entity, "disabled")?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryIconColors,
    ) -> bevy_widgetry_theme::WidgetryIconColors {
        bevy_widgetry_theme::WidgetryIconColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryIconColorOverrides);
impl WidgetryIconColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, IconFailure> {
        if world.get::<super::WidgetryIcon>(entity).is_none() {
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
    ) -> Result<bool, IconFailure> {
        Self::get(world, entity)?;
        if let Some(owner) =
            world.get::<crate::color::WidgetryStyleOwner<super::WidgetryIcon>>(entity)
        {
            return Err(failure(
                IconError::ManagedColor {
                    entity,
                    owner: owner.entity,
                },
                "set icon colors",
            ));
        }
        colors.validate(Some(entity))?;
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
                .map_err(into_bevy_error)
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, IconFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, entity: Entity) -> Result<ColorState, IconFailure> {
        self.validate(Some(entity))?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(entity: Entity) -> IconFailure {
    failure(
        IconError::InvalidTarget {
            entity,
            missing: "WidgetryIcon/ColorState",
            path: None,
        },
        "access icon colors",
    )
}
