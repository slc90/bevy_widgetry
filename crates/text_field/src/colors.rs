use crate::{
    TextFieldError, TextFieldFailure,
    error::{core, failure, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub caret: Option<Color>,
    pub selection_background: Option<Color>,
    pub unfocused_selection_background: Option<Color>,
    pub selection_foreground: Option<Color>,
}
impl WidgetryTextFieldStateColorOverrides {
    fn validate(&self) -> Result<(), TextFieldFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("text_field.validate_colors").field("field", "background"),
                )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("text_field.validate_colors").field("field", "border"),
                )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("text_field.validate_colors").field("field", "foreground"),
                )
            })?;
        }
        if let Some(color) = self.caret {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("text_field.validate_colors").field("field", "caret"),
                )
            })?;
        }
        if let Some(color) = self.selection_background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("text_field.validate_colors")
                        .field("field", "selection_background"),
                )
            })?;
        }
        if let Some(color) = self.unfocused_selection_background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("text_field.validate_colors")
                        .field("field", "unfocused_selection_background"),
                )
            })?;
        }
        if let Some(color) = self.selection_foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("text_field.validate_colors")
                        .field("field", "selection_foreground"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldStateColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldStateColors {
        bevy_widgetry_theme::WidgetryTextFieldStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            caret: self.caret.unwrap_or(theme.caret),
            selection_background: self
                .selection_background
                .unwrap_or(theme.selection_background),
            unfocused_selection_background: self
                .unfocused_selection_background
                .unwrap_or(theme.unfocused_selection_background),
            selection_foreground: self
                .selection_foreground
                .unwrap_or(theme.selection_foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldInteractionColorOverrides {
    pub normal: WidgetryTextFieldStateColorOverrides,
    pub hovered: WidgetryTextFieldStateColorOverrides,
    pub focused: WidgetryTextFieldStateColorOverrides,
    pub disabled: WidgetryTextFieldStateColorOverrides,
}
impl WidgetryTextFieldInteractionColorOverrides {
    fn validate(&self) -> Result<(), TextFieldFailure> {
        self.normal.validate().map_err(|error| {
            error.with_context(
                ErrorFrame::new("text_field.validate_palette").field("state", "normal"),
            )
        })?;
        self.hovered.validate().map_err(|error| {
            error.with_context(
                ErrorFrame::new("text_field.validate_palette").field("state", "hovered"),
            )
        })?;
        self.focused.validate().map_err(|error| {
            error.with_context(
                ErrorFrame::new("text_field.validate_palette").field("state", "focused"),
            )
        })?;
        self.disabled.validate().map_err(|error| {
            error.with_context(
                ErrorFrame::new("text_field.validate_palette").field("state", "disabled"),
            )
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldInteractionColors {
        bevy_widgetry_theme::WidgetryTextFieldInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            focused: self.focused.resolve(&theme.focused),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldColorOverrides {
    pub editable: WidgetryTextFieldInteractionColorOverrides,
    pub read_only: WidgetryTextFieldInteractionColorOverrides,
}
impl WidgetryTextFieldColorOverrides {
    fn validate(&self) -> Result<(), TextFieldFailure> {
        self.editable.validate().map_err(|error| {
            error.with_context(
                ErrorFrame::new("text_field.validate_palette").field("state", "editable"),
            )
        })?;
        self.read_only.validate().map_err(|error| {
            error.with_context(
                ErrorFrame::new("text_field.validate_palette").field("state", "read_only"),
            )
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldColors {
        bevy_widgetry_theme::WidgetryTextFieldColors {
            hint_warning: theme.hint_warning,
            editable: self.editable.resolve(&theme.editable),
            read_only: self.read_only.resolve(&theme.read_only),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryTextFieldColorOverrides);
impl WidgetryTextFieldColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, TextFieldFailure> {
        if world.get::<crate::WidgetryTextField>(entity).is_none()
            && world
                .get::<crate::WidgetryReadOnlyTextField>(entity)
                .is_none()
        {
            return Err(failure(
                TextFieldError::InvalidRoot {
                    root: entity,
                    expected: "TextField / ReadOnlyTextField",
                },
                ErrorFrame::new("text_field.color_target"),
            ));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| missing(entity, entity, "ColorState", "text_field.get_colors"))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, TextFieldFailure> {
        Self::get(world, entity)?;
        if world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTextField>>(entity)
            .is_some()
        {
            return Err(failure(
                TextFieldError::OwnedStyle { root: entity },
                ErrorFrame::new("text_field.set_colors"),
            ));
        }
        colors.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("text_field.set_colors").field("root", entity))
        })?;
        let mut state = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| missing(entity, entity, "ColorState", "text_field.set_colors"))?;
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, TextFieldFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, TextFieldFailure> {
        self.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("text_field.initialize_colors").field("root", root))
        })?;
        Ok(ColorState(self))
    }
}
