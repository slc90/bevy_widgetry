use crate::{
    DatePickerFailure,
    error::{core, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_theme::WidgetryDatePickerColors;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryDatePickerColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub adjacent: Option<Color>,
    pub disabled: Option<Color>,
    pub selected: Option<Color>,
    pub hovered: Option<Color>,
    pub pressed: Option<Color>,
}

#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryDatePickerColorOverrides);

impl WidgetryDatePickerColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), DatePickerFailure> {
        for (field, color) in [
            ("background", self.background),
            ("border", self.border),
            ("foreground", self.foreground),
            ("adjacent", self.adjacent),
            ("disabled", self.disabled),
            ("selected", self.selected),
            ("hovered", self.hovered),
            ("pressed", self.pressed),
        ] {
            let Some(color) = color else {
                continue;
            };
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "date_picker.validate_colors")
                    .with_context(ErrorFrame::new("date_picker.color_field").field("field", field))
            })?;
        }
        Ok(())
    }

    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, DatePickerFailure> {
        self.validate(root)?;
        Ok(ColorState(self))
    }

    pub(crate) fn resolve(&self, base: &WidgetryDatePickerColors) -> WidgetryDatePickerColors {
        WidgetryDatePickerColors {
            background: self.background.unwrap_or(base.background),
            border: self.border.unwrap_or(base.border),
            foreground: self.foreground.unwrap_or(base.foreground),
            adjacent: self.adjacent.unwrap_or(base.adjacent),
            disabled: self.disabled.unwrap_or(base.disabled),
            selected: self.selected.unwrap_or(base.selected),
            hovered: self.hovered.unwrap_or(base.hovered),
            pressed: self.pressed.unwrap_or(base.pressed),
        }
    }

    pub fn get(world: &World, root: Entity) -> Result<&Self, DatePickerFailure> {
        crate::model::state(world, root)?;
        world
            .get::<ColorState>(root)
            .map(|state| &state.0)
            .ok_or_else(|| missing(root, root, "ColorState", "date_picker.colors"))
    }

    pub fn set_in_world(
        world: &mut World,
        root: Entity,
        colors: Self,
    ) -> Result<bool, DatePickerFailure> {
        Self::get(world, root)?;
        colors.validate(root).map_err(|error| {
            error.with_context(ErrorFrame::new("date_picker.set_colors").field("root", root))
        })?;
        let mut current = world
            .get_mut::<ColorState>(root)
            .ok_or_else(|| missing(root, root, "ColorState", "date_picker.colors"))?;
        if current.0 == colors {
            return Ok(false);
        }
        current.0 = colors;
        Ok(true)
    }

    pub fn set(commands: &mut Commands, root: Entity, colors: Self) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, root, colors)
                .map(|_| ())
                .map_err(DatePickerFailure::into_bevy_error)
        });
    }

    pub fn clear_in_world(world: &mut World, root: Entity) -> Result<bool, DatePickerFailure> {
        Self::set_in_world(world, root, Self::default())
    }

    pub fn clear(commands: &mut Commands, root: Entity) {
        Self::set(commands, root, Self::default());
    }
}
