use crate::model::invalid;
use bevy::prelude::*;
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
    fn validate(&self) -> Result<(), BevyError> {
        for color in [
            self.background,
            self.border,
            self.foreground,
            self.adjacent,
            self.disabled,
            self.selected,
            self.hovered,
            self.pressed,
        ]
        .into_iter()
        .flatten()
        {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }

    pub(crate) fn initial(self) -> Result<ColorState, BevyError> {
        self.validate()?;
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

    pub fn get(world: &World, root: Entity) -> Result<&Self, BevyError> {
        crate::model::state(world, root)?;
        world
            .get::<ColorState>(root)
            .map(|state| &state.0)
            .ok_or_else(|| invalid("DatePicker colors are missing"))
    }

    pub fn set_in_world(world: &mut World, root: Entity, colors: Self) -> Result<bool, BevyError> {
        Self::get(world, root)?;
        colors.validate()?;
        let mut current = world
            .get_mut::<ColorState>(root)
            .ok_or_else(|| invalid("DatePicker colors are missing"))?;
        if current.0 == colors {
            return Ok(false);
        }
        current.0 = colors;
        Ok(true)
    }

    pub fn set(commands: &mut Commands, root: Entity, colors: Self) {
        commands
            .queue(move |world: &mut World| Self::set_in_world(world, root, colors).map(|_| ()));
    }

    pub fn clear_in_world(world: &mut World, root: Entity) -> Result<bool, BevyError> {
        Self::set_in_world(world, root, Self::default())
    }

    pub fn clear(commands: &mut Commands, root: Entity) {
        Self::set(commands, root, Self::default());
    }
}
