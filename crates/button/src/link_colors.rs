use crate::{
    LinkButtonError, LinkButtonFailure,
    link_error::{core, failure},
};
use bevy::prelude::*;
use bevy_widgetry_theme::WidgetryLinkButtonColors;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryLinkButtonColorOverrides {
    pub normal: Option<Color>,
    pub hovered: Option<Color>,
    pub pressed: Option<Color>,
    pub disabled: Option<Color>,
}

#[derive(Component, Default)]
pub(crate) struct LinkColorState(pub(crate) WidgetryLinkButtonColorOverrides);

impl WidgetryLinkButtonColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), LinkButtonFailure> {
        for (field, color) in [
            ("normal", self.normal),
            ("hovered", self.hovered),
            ("pressed", self.pressed),
            ("disabled", self.disabled),
        ] {
            let Some(color) = color else {
                continue;
            };
            bevy_widgetry_core::color::validate_color(color)
                .map_err(|error| core(error, root, field))?;
        }
        Ok(())
    }

    pub(crate) fn resolve(&self, theme: &WidgetryLinkButtonColors) -> WidgetryLinkButtonColors {
        WidgetryLinkButtonColors {
            normal: self.normal.unwrap_or(theme.normal),
            hovered: self.hovered.unwrap_or(theme.hovered),
            pressed: self.pressed.unwrap_or(theme.pressed),
            disabled: self.disabled.unwrap_or(theme.disabled),
        }
    }

    pub fn get(world: &World, entity: Entity) -> Result<&Self, LinkButtonFailure> {
        if world.get::<crate::WidgetryLinkButton>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<LinkColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| missing_state(entity))
    }

    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, LinkButtonFailure> {
        Self::get(world, entity)?;
        colors.validate(entity)?;
        let mut state = world
            .get_mut::<LinkColorState>(entity)
            .ok_or_else(|| missing_state(entity))?;
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

    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, LinkButtonFailure> {
        Self::set_in_world(world, entity, Self::default())
    }

    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }

    pub(crate) fn initial(self, root: Entity) -> Result<LinkColorState, LinkButtonFailure> {
        self.validate(root)?;
        Ok(LinkColorState(self))
    }
}

fn invalid_target(root: Entity) -> LinkButtonFailure {
    failure(
        LinkButtonError::InvalidTarget { root },
        "link_button.get_colors",
    )
}

fn missing_state(root: Entity) -> LinkButtonFailure {
    failure(
        LinkButtonError::MissingPart {
            root,
            part: "LinkColorState",
        },
        "link_button.get_colors",
    )
}
