use crate::{
    ComboError, ComboFailure,
    error::{core, failure},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewItemStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryListViewItemStateColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "background"),
                )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "border"),
                )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "foreground"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewItemStateColors,
    ) -> bevy_widgetry_theme::WidgetryListViewItemStateColors {
        bevy_widgetry_theme::WidgetryListViewItemStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewItemColorOverrides {
    pub normal: WidgetryListViewItemStateColorOverrides,
    pub hovered: WidgetryListViewItemStateColorOverrides,
    pub pressed: WidgetryListViewItemStateColorOverrides,
    pub selected: WidgetryListViewItemStateColorOverrides,
    pub disabled: WidgetryListViewItemStateColorOverrides,
    pub active_border: Option<Color>,
    pub disabled_active_border: Option<Color>,
}
impl WidgetryListViewItemColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        self.normal.validate(root)?;
        self.hovered.validate(root)?;
        self.pressed.validate(root)?;
        self.selected.validate(root)?;
        self.disabled.validate(root)?;
        if let Some(color) = self.active_border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "active_border"),
                )
            })?;
        }
        if let Some(color) = self.disabled_active_border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "disabled_active_border"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewItemColors,
    ) -> bevy_widgetry_theme::WidgetryListViewItemColors {
        bevy_widgetry_theme::WidgetryListViewItemColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            selected: self.selected.resolve(&theme.selected),
            disabled: self.disabled.resolve(&theme.disabled),
            active_border: self.active_border.unwrap_or(theme.active_border),
            disabled_active_border: self
                .disabled_active_border
                .unwrap_or(theme.disabled_active_border),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewContainerStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryListViewContainerStateColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "background"),
                )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "border"),
                )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "foreground"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewContainerStateColors,
    ) -> bevy_widgetry_theme::WidgetryListViewContainerStateColors {
        bevy_widgetry_theme::WidgetryListViewContainerStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewContainerColorOverrides {
    pub normal: WidgetryListViewContainerStateColorOverrides,
    pub focused: WidgetryListViewContainerStateColorOverrides,
    pub disabled: WidgetryListViewContainerStateColorOverrides,
}
impl WidgetryListViewContainerColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        self.normal.validate(root)?;
        self.focused.validate(root)?;
        self.disabled.validate(root)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewContainerColors,
    ) -> bevy_widgetry_theme::WidgetryListViewContainerColors {
        bevy_widgetry_theme::WidgetryListViewContainerColors {
            normal: self.normal.resolve(&theme.normal),
            focused: self.focused.resolve(&theme.focused),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewColorOverrides {
    pub container: WidgetryListViewContainerColorOverrides,
    pub item: WidgetryListViewItemColorOverrides,
}
impl WidgetryListViewColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        self.container.validate(root)?;
        self.item.validate(root)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewColors,
    ) -> bevy_widgetry_theme::WidgetryListViewColors {
        bevy_widgetry_theme::WidgetryListViewColors {
            container: self.container.resolve(&theme.container),
            item: self.item.resolve(&theme.item),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxPopupStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryComboBoxPopupStateColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "background"),
                )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "border"),
                )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "foreground"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxPopupStateColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxPopupStateColors {
        bevy_widgetry_theme::WidgetryComboBoxPopupStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxPopupColorOverrides {
    pub normal: WidgetryComboBoxPopupStateColorOverrides,
    pub disabled: WidgetryComboBoxPopupStateColorOverrides,
}
impl WidgetryComboBoxPopupColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        self.normal.validate(root)?;
        self.disabled.validate(root)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxPopupColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxPopupColors {
        bevy_widgetry_theme::WidgetryComboBoxPopupColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxFieldStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub indicator_foreground: Option<Color>,
}
impl WidgetryComboBoxFieldStateColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "background"),
                )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "border"),
                )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "foreground"),
                )
            })?;
        }
        if let Some(color) = self.indicator_foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, root, "combo_box.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "combo_box")
                        .field("field", "indicator_foreground"),
                )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxFieldStateColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxFieldStateColors {
        bevy_widgetry_theme::WidgetryComboBoxFieldStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            indicator_foreground: self
                .indicator_foreground
                .unwrap_or(theme.indicator_foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxFieldColorOverrides {
    pub normal: WidgetryComboBoxFieldStateColorOverrides,
    pub hovered: WidgetryComboBoxFieldStateColorOverrides,
    pub pressed: WidgetryComboBoxFieldStateColorOverrides,
    pub open: WidgetryComboBoxFieldStateColorOverrides,
    pub disabled: WidgetryComboBoxFieldStateColorOverrides,
}
impl WidgetryComboBoxFieldColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        self.normal.validate(root)?;
        self.hovered.validate(root)?;
        self.pressed.validate(root)?;
        self.open.validate(root)?;
        self.disabled.validate(root)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxFieldColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxFieldColors {
        bevy_widgetry_theme::WidgetryComboBoxFieldColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            open: self.open.resolve(&theme.open),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxColorOverrides {
    pub field: WidgetryComboBoxFieldColorOverrides,
    pub popup: WidgetryComboBoxPopupColorOverrides,
    pub list: WidgetryListViewColorOverrides,
}
impl WidgetryComboBoxColorOverrides {
    fn validate(&self, root: Entity) -> Result<(), ComboFailure> {
        self.field.validate(root)?;
        self.popup.validate(root)?;
        self.list.validate(root)?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxColors {
        bevy_widgetry_theme::WidgetryComboBoxColors {
            field: self.field.resolve(&theme.field),
            popup: self.popup.resolve(&theme.popup),
            list: self.list.resolve(&theme.list),
        }
    }
}
#[derive(Component)]
pub(crate) struct ColorState(
    pub(crate) WidgetryComboBoxColorOverrides,
    fn(&World, Entity) -> bool,
    fn(&World, Entity) -> Option<Entity>,
);
impl ColorState {
    pub(crate) fn new<T: Send + Sync + 'static>() -> Self {
        Self(
            WidgetryComboBoxColorOverrides::default(),
            |world, entity| world.get::<crate::WidgetryComboBox<T>>(entity).is_some(),
            |world, entity| {
                world.get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryComboBox<T>>>(entity).map(|owner|owner.entity)
            },
        )
    }
}
impl WidgetryComboBoxColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, ComboFailure> {
        world
            .get::<ColorState>(entity)
            .filter(|state| (state.1)(world, entity))
            .map(|state| &state.0)
            .ok_or_else(|| invalid_target(entity))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, ComboFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<ColorState>(entity)
            .and_then(|state| (state.2)(world, entity))
        {
            return Err(failure(
                ComboError::ManagedColors {
                    root: entity,
                    owner,
                },
                "combo_box.set_colors",
            ));
        }
        colors.validate(entity).map_err(|error| {
            error.with_context(ErrorFrame::new("combo_box.set_colors").field("combo", entity))
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
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, entity, colors)
                .map(|_| ())
                .map_err(ComboFailure::into_bevy_error)
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, ComboFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial<T: Send + Sync + 'static>(
        self,
        root: Entity,
    ) -> Result<ColorState, ComboFailure> {
        self.validate(root)?;
        let mut state = ColorState::new::<T>();
        state.0 = self;
        Ok(state)
    }
}
#[cold]
fn invalid_target(root: Entity) -> ComboFailure {
    failure(ComboError::InvalidColorTarget { root }, "combo_box.colors")
}
