use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryHoldButtonStateColorOverrides {
    pub background: Option<Color>,
    pub liquid: Option<Color>,
    pub wave: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryHoldButtonStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "hold_button")
                            .field("field", "background"),
                    )
                    .into_bevy_error()
            })?;
        }
        for (field, color) in [("liquid", self.liquid), ("wave", self.wave)] {
            let Some(color) = color else {
                continue;
            };
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "hold_button")
                            .field("field", field),
                    )
                    .into_bevy_error()
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "hold_button")
                            .field("field", "border"),
                    )
                    .into_bevy_error()
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "hold_button")
                            .field("field", "foreground"),
                    )
                    .into_bevy_error()
            })?;
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
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        if let Some(color) = self.success_border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "hold_button")
                            .field("field", "success_border"),
                    )
                    .into_bevy_error()
            })?;
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
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryHoldButton>(entity).is_none() {
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
        bevy::ecs::error::ContextExt::with_context(colors.validate(), || {
            format!("set widget colors: root={entity}")
        })?;
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
        "WidgetryHoldButtonColorOverrides 目标不存在、类型不匹配或内部状态缺失"
    );
    BevyError::error("WidgetryHoldButtonColorOverrides 目标不存在、类型不匹配或内部状态缺失")
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod diagnostic_tests {
    use super::*;

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
                error: colors.validate().unwrap_err(),
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
