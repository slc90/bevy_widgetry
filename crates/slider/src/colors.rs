use crate::{
    SliderError, SliderFailure,
    error::{core, failure, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySliderStateColorOverrides {
    pub track: Option<Color>,
    pub fill: Option<Color>,
    pub thumb: Option<Color>,
    pub focus_ring: Option<Color>,
}
impl WidgetrySliderStateColorOverrides {
    fn validate(&self) -> Result<(), SliderFailure> {
        if let Some(color) = self.track {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("slider.validate_colors").field("field", "track"),
                )
            })?;
        }
        if let Some(color) = self.fill {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("slider.validate_colors").field("field", "fill"),
                )
            })?;
        }
        if let Some(color) = self.thumb {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("slider.validate_colors").field("field", "thumb"),
                )
            })?;
        }
        if let Some(color) = self.focus_ring {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("slider.validate_colors").field("field", "focus_ring"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySliderStateColors,
    ) -> bevy_widgetry_theme::WidgetrySliderStateColors {
        bevy_widgetry_theme::WidgetrySliderStateColors {
            track: self.track.unwrap_or(theme.track),
            fill: self.fill.unwrap_or(theme.fill),
            thumb: self.thumb.unwrap_or(theme.thumb),
            focus_ring: self.focus_ring.unwrap_or(theme.focus_ring),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetrySliderColorOverrides {
    pub normal: WidgetrySliderStateColorOverrides,
    pub hovered: WidgetrySliderStateColorOverrides,
    pub pressed: WidgetrySliderStateColorOverrides,
    pub disabled: WidgetrySliderStateColorOverrides,
}
impl WidgetrySliderColorOverrides {
    fn validate(&self) -> Result<(), SliderFailure> {
        for (state, colors) in [
            ("normal", &self.normal),
            ("hovered", &self.hovered),
            ("pressed", &self.pressed),
            ("disabled", &self.disabled),
        ] {
            colors.validate().map_err(|error| {
                error.with_context(ErrorFrame::new("slider.validate_palette").field("state", state))
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetrySliderColors,
    ) -> bevy_widgetry_theme::WidgetrySliderColors {
        bevy_widgetry_theme::WidgetrySliderColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetrySliderColorOverrides);
impl WidgetrySliderColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, SliderFailure> {
        if world.get::<crate::WidgetrySlider>(entity).is_none() {
            return Err(failure(
                SliderError::InvalidRoot { root: entity },
                ErrorFrame::new("slider.color_target"),
            ));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| missing(entity, entity, "ColorState"))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, SliderFailure> {
        Self::get(world, entity)?;
        colors.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("slider.set_colors").field("root", entity))
        })?;
        let mut state = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| missing(entity, entity, "ColorState"))?;
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, SliderFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, SliderFailure> {
        self.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("slider.initialize_colors").field("root", root))
        })?;
        Ok(ColorState(self))
    }
}
