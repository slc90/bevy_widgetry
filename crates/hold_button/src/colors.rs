use crate::{
    HoldButtonError, HoldButtonFailure,
    error::{core, failure},
};
use bevy::prelude::*;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryHoldButtonStateColorOverrides {
    pub background: Option<Color>,
    pub liquid: Option<Color>,
    pub wave: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryHoldButtonStateColorOverrides {
    fn validate(&self, root: Entity, state: &'static str) -> Result<(), HoldButtonFailure> {
        for (field, color) in [
            ("background", self.background),
            ("liquid", self.liquid),
            ("wave", self.wave),
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
        theme: &bevy_widgetry_theme::WidgetryHoldButtonStateColors,
    ) -> bevy_widgetry_theme::WidgetryHoldButtonStateColors {
        bevy_widgetry_theme::WidgetryHoldButtonStateColors {
            background: self.background.unwrap_or(theme.background),
            liquid: self.liquid.unwrap_or(theme.liquid),
            wave: self.wave.unwrap_or(theme.wave),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryHoldButtonColorOverrides {
    pub normal: WidgetryHoldButtonStateColorOverrides,
    pub hovered: WidgetryHoldButtonStateColorOverrides,
    pub pressed: WidgetryHoldButtonStateColorOverrides,
    pub disabled: WidgetryHoldButtonStateColorOverrides,
    pub success_border: Option<Color>,
}
impl WidgetryHoldButtonColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), HoldButtonFailure> {
        self.normal.validate(root, "normal")?;
        self.hovered.validate(root, "hovered")?;
        self.pressed.validate(root, "pressed")?;
        self.disabled.validate(root, "disabled")?;
        if let Some(color) = self.success_border {
            bevy_widgetry_core::color::validate_color(color)
                .map_err(|error| core(error, root, "success", "success_border"))?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryHoldButtonColors,
    ) -> bevy_widgetry_theme::WidgetryHoldButtonColors {
        bevy_widgetry_theme::WidgetryHoldButtonColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
            success_border: self.success_border.unwrap_or(theme.success_border),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryHoldButtonColorOverrides);
impl WidgetryHoldButtonColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, HoldButtonFailure> {
        if world.get::<crate::WidgetryHoldButton>(entity).is_none() {
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
    ) -> Result<bool, HoldButtonFailure> {
        Self::get(world, entity)?;
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, HoldButtonFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, HoldButtonFailure> {
        self.validate(root)?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(root: Entity) -> HoldButtonFailure {
    failure(
        HoldButtonError::InvalidTarget { root },
        "hold_button.get_colors",
    )
}

fn missing_state(root: Entity) -> HoldButtonFailure {
    failure(
        HoldButtonError::MissingPart {
            root,
            part: "ColorState",
        },
        "hold_button.get_colors",
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn missing_private_colors_are_distinct_from_invalid_targets() {
        let mut world = World::new();
        let root = world.spawn(crate::WidgetryHoldButton).id();
        world.entity_mut(root).remove::<ColorState>();
        let error = WidgetryHoldButtonColorOverrides::clear_in_world(&mut world, root).unwrap_err();
        assert!(
            matches!(error.kind(), HoldButtonError::MissingPart { root: target, part: "ColorState" } if *target == root)
        );
        assert!(world.get::<ColorState>(root).is_none());
    }

    #[test]
    fn liquid_and_wave_failures_name_the_actual_color_field() {
        for field in ["liquid", "wave"] {
            let invalid = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
            let mut colors = WidgetryHoldButtonStateColorOverrides::default();
            if field == "liquid" {
                colors.liquid = invalid;
            } else {
                colors.wave = invalid;
            }
            let record = bevy_widgetry_test_utils::CapturedError {
                error: colors
                    .validate(Entity::PLACEHOLDER, "normal")
                    .unwrap_err()
                    .into_bevy_error(),
                context: bevy::ecs::error::ErrorContext::Command {
                    name: "color validation".into(),
                },
            };
            let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
            assert!(
                diagnostic
                    .contexts()
                    .iter()
                    .flat_map(|context| context.fields())
                    .any(|(name, value)| name == "field" && value == field)
            );
        }
    }
}
