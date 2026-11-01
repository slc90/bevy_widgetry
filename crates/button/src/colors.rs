use crate::{
    ButtonError, ButtonFailure,
    error::{core, failure},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryButtonStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryButtonStateColorOverrides {
    fn validate(&self, root: Entity, state: &'static str) -> Result<(), ButtonFailure> {
        for (field, color) in [
            ("background", self.background),
            ("border", self.border),
            ("foreground", self.foreground),
        ] {
            if let Some(color) = color {
                bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                    core(error, root, "button.validate_colors").with_context(
                        ErrorFrame::new("button.color")
                            .field("state", state)
                            .field("field", field),
                    )
                })?;
            }
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryButtonStateColors,
    ) -> bevy_widgetry_theme::WidgetryButtonStateColors {
        bevy_widgetry_theme::WidgetryButtonStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryButtonColorOverrides {
    pub normal: WidgetryButtonStateColorOverrides,
    pub hovered: WidgetryButtonStateColorOverrides,
    pub pressed: WidgetryButtonStateColorOverrides,
    pub disabled: WidgetryButtonStateColorOverrides,
}
impl WidgetryButtonColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ButtonFailure> {
        self.normal.validate(root, "normal")?;
        self.hovered.validate(root, "hovered")?;
        self.pressed.validate(root, "pressed")?;
        self.disabled.validate(root, "disabled")?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryButtonColors,
    ) -> bevy_widgetry_theme::WidgetryButtonColors {
        bevy_widgetry_theme::WidgetryButtonColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryButtonColorOverrides);
impl WidgetryButtonColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, ButtonFailure> {
        if world.get::<crate::WidgetryButton>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| {
                failure(
                    ButtonError::MissingPart {
                        root: entity,
                        part: "ColorState",
                    },
                    "button.get_colors",
                )
            })
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, ButtonFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryButton>>(entity)
        {
            return Err(failure(
                ButtonError::ManagedColors {
                    root: entity,
                    owner: owner.entity,
                },
                "button.set_colors",
            ));
        }
        colors.validate(entity)?;
        let mut state = world.get_mut::<ColorState>(entity).ok_or_else(|| {
            failure(
                ButtonError::MissingPart {
                    root: entity,
                    part: "ColorState",
                },
                "button.set_colors",
            )
        })?;
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, ButtonFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, ButtonFailure> {
        self.validate(root)?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(root: Entity) -> ButtonFailure {
    failure(ButtonError::InvalidTarget { root }, "button.get_colors")
}
