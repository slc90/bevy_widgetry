use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
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
    fn validate(&self) -> Result<(), BevyError> {
        for color in [self.normal, self.hovered, self.pressed, self.disabled]
            .into_iter()
            .flatten()
        {
            bevy_widgetry_core::color::validate_color(color)?;
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

    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryLinkButton>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<LinkColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| invalid_target(entity))
    }

    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, BevyError> {
        Self::get(world, entity)?;
        colors.validate()?;
        let mut state = world
            .get_mut::<LinkColorState>(entity)
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

    pub(crate) fn initial(self) -> Result<LinkColorState, BevyError> {
        self.validate()?;
        Ok(LinkColorState(self))
    }
}

fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "WidgetryLinkButtonColorOverrides 目标不存在或类型不匹配"
    );
    BevyError::error("WidgetryLinkButtonColorOverrides 目标不存在或类型不匹配")
}
