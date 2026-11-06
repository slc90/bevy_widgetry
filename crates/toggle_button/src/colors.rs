use crate::{
    ToggleButtonError, ToggleButtonFailure,
    error::{core, failure},
};
use bevy::prelude::*;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryToggleButtonStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryToggleButtonStateColorOverrides {
    fn validate(&self, root: Entity, state: &'static str) -> Result<(), ToggleButtonFailure> {
        for (field, color) in [
            ("background", self.background),
            ("border", self.border),
            ("foreground", self.foreground),
        ] {
            if let Some(color) = color {
                bevy_widgetry_core::color::validate_color(color)
                    .map_err(|error| core(error, root, state, field))?;
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
pub struct WidgetryToggleButtonColorOverrides {
    pub normal: WidgetryToggleButtonStateColorOverrides,
    pub hovered: WidgetryToggleButtonStateColorOverrides,
    pub pressed: WidgetryToggleButtonStateColorOverrides,
    pub disabled: WidgetryToggleButtonStateColorOverrides,
    pub selected: WidgetryToggleButtonStateColorOverrides,
    pub selected_hovered: WidgetryToggleButtonStateColorOverrides,
    pub selected_pressed: WidgetryToggleButtonStateColorOverrides,
    pub selected_disabled: WidgetryToggleButtonStateColorOverrides,
}
impl WidgetryToggleButtonColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ToggleButtonFailure> {
        self.normal.validate(root, "normal")?;
        self.hovered.validate(root, "hovered")?;
        self.pressed.validate(root, "pressed")?;
        self.disabled.validate(root, "disabled")?;
        self.selected.validate(root, "selected")?;
        self.selected_hovered.validate(root, "selected_hovered")?;
        self.selected_pressed.validate(root, "selected_pressed")?;
        self.selected_disabled.validate(root, "selected_disabled")?;
        Ok(())
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryToggleButtonColorOverrides);
impl WidgetryToggleButtonColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, ToggleButtonFailure> {
        if world.get::<crate::WidgetryToggleButton>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| missing_state(entity))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, ToggleButtonFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world.get::<bevy_widgetry_core::color::WidgetryStyleOwner<
            crate::WidgetryToggleButton,
        >>(entity)
        {
            return Err(failure(
                ToggleButtonError::ManagedColors {
                    root: entity,
                    owner: owner.entity,
                },
                "toggle_button.set_colors",
            ));
        }
        colors.validate(entity)?;
        let mut state = world
            .get_mut::<ColorState>(entity)
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, ToggleButtonFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, ToggleButtonFailure> {
        self.validate(root)?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(root: Entity) -> ToggleButtonFailure {
    failure(
        ToggleButtonError::InvalidTarget { root },
        "toggle_button.get_colors",
    )
}

fn missing_state(root: Entity) -> ToggleButtonFailure {
    failure(
        ToggleButtonError::MissingPart {
            root,
            part: "ColorState",
        },
        "toggle_button.get_colors",
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn confirmed_toggle_missing_color_state_is_an_internal_failure() {
        let mut world = World::new();
        let root = world.spawn(crate::WidgetryToggleButton).id();
        world.entity_mut(root).remove::<ColorState>();
        let failure =
            WidgetryToggleButtonColorOverrides::clear_in_world(&mut world, root).unwrap_err();
        assert!(
            matches!(failure.kind(), ToggleButtonError::MissingPart { root: target, part: "ColorState" } if *target == root)
        );
        assert!(world.get::<ColorState>(root).is_none());
        assert!(!crate::WidgetryToggleButtonHeadless::selected(&world, root).unwrap());
    }
}
