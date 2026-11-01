use crate::{
    CheckBoxError, CheckBoxFailure,
    error::{core, failure},
};
use bevy::prelude::*;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub mark: Option<Color>,
}
impl WidgetryCheckBoxStateColorOverrides {
    fn validate(
        &self,
        root: Entity,
        state: &'static str,
        interaction: &'static str,
    ) -> Result<(), CheckBoxFailure> {
        for (field, color) in [
            ("background", self.background),
            ("border", self.border),
            ("foreground", self.foreground),
            ("mark", self.mark),
        ] {
            if let Some(color) = color {
                bevy_widgetry_core::color::validate_color(color)
                    .map_err(|error| core(error, root, state, interaction, field))?;
            }
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxStateColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxStateColors {
        bevy_widgetry_theme::WidgetryCheckBoxStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            mark: self.mark.unwrap_or(theme.mark),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxInteractionColorOverrides {
    pub normal: WidgetryCheckBoxStateColorOverrides,
    pub hovered: WidgetryCheckBoxStateColorOverrides,
    pub pressed: WidgetryCheckBoxStateColorOverrides,
    pub disabled: WidgetryCheckBoxStateColorOverrides,
}
impl WidgetryCheckBoxInteractionColorOverrides {
    fn validate(&self, root: Entity, state: &'static str) -> Result<(), CheckBoxFailure> {
        self.normal.validate(root, state, "normal")?;
        self.hovered.validate(root, state, "hovered")?;
        self.pressed.validate(root, state, "pressed")?;
        self.disabled.validate(root, state, "disabled")?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxInteractionColors {
        bevy_widgetry_theme::WidgetryCheckBoxInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxColorOverrides {
    pub unchecked: WidgetryCheckBoxInteractionColorOverrides,
    pub checked: WidgetryCheckBoxInteractionColorOverrides,
    pub indeterminate: WidgetryCheckBoxInteractionColorOverrides,
}
impl WidgetryCheckBoxColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), CheckBoxFailure> {
        self.unchecked.validate(root, "unchecked")?;
        self.checked.validate(root, "checked")?;
        self.indeterminate.validate(root, "indeterminate")?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxColors {
        bevy_widgetry_theme::WidgetryCheckBoxColors {
            unchecked: self.unchecked.resolve(&theme.unchecked),
            checked: self.checked.resolve(&theme.checked),
            indeterminate: self.indeterminate.resolve(&theme.indeterminate),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryCheckBoxColorOverrides);
impl WidgetryCheckBoxColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, CheckBoxFailure> {
        if world.get::<crate::WidgetryCheckBox>(entity).is_none()
            && world
                .get::<crate::WidgetryTriStateCheckbox>(entity)
                .is_none()
        {
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
    ) -> Result<bool, CheckBoxFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryCheckBox>>(entity)
        {
            return Err(failure(
                CheckBoxError::ManagedColors {
                    root: entity,
                    owner: owner.entity,
                },
                "check_box.set_colors",
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
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, CheckBoxFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, root: Entity) -> Result<ColorState, CheckBoxFailure> {
        self.validate(root)?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(root: Entity) -> CheckBoxFailure {
    failure(
        CheckBoxError::InvalidTarget { root },
        "check_box.get_colors",
    )
}
fn missing_state(root: Entity) -> CheckBoxFailure {
    failure(
        CheckBoxError::MissingPart {
            root,
            entity: root,
            part: "ColorState",
        },
        "check_box.get_colors",
    )
}
