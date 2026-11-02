use crate::WidgetryListKey;
use crate::{
    ListError, ListFailure,
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
    fn validate(&self, entity: Entity) -> Result<(), ListFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, entity, "list_view.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "list_view")
                        .field("field", "background"),
                )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, entity, "list_view.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "list_view")
                        .field("field", "border"),
                )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, entity, "list_view.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "list_view")
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
    fn validate(&self, entity: Entity) -> Result<(), ListFailure> {
        self.normal.validate(entity).map_err(|error| {
            error
                .with_context(ErrorFrame::new("list_view.validate_colors").field("state", "normal"))
        })?;
        self.hovered.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("list_view.validate_colors").field("state", "hovered"),
            )
        })?;
        self.pressed.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("list_view.validate_colors").field("state", "pressed"),
            )
        })?;
        self.selected.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("list_view.validate_colors").field("state", "selected"),
            )
        })?;
        self.disabled.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("list_view.validate_colors").field("state", "disabled"),
            )
        })?;
        if let Some(color) = self.active_border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, entity, "list_view.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "list_view")
                        .field("field", "active_border"),
                )
            })?;
        }
        if let Some(color) = self.disabled_active_border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, entity, "list_view.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "list_view")
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
    fn validate(&self, entity: Entity) -> Result<(), ListFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, entity, "list_view.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "list_view")
                        .field("field", "background"),
                )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, entity, "list_view.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "list_view")
                        .field("field", "border"),
                )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                core(error, entity, "list_view.validate_colors").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                        .field("consumer", "list_view")
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
    fn validate(&self, entity: Entity) -> Result<(), ListFailure> {
        self.normal.validate(entity).map_err(|error| {
            error
                .with_context(ErrorFrame::new("list_view.validate_colors").field("state", "normal"))
        })?;
        self.focused.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("list_view.validate_colors").field("state", "focused"),
            )
        })?;
        self.disabled.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("list_view.validate_colors").field("state", "disabled"),
            )
        })?;
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
    fn validate(&self, entity: Entity) -> Result<(), ListFailure> {
        self.container.validate(entity).map_err(|error| {
            error.with_context(
                ErrorFrame::new("list_view.validate_colors").field("state", "container"),
            )
        })?;
        self.item.validate(entity).map_err(|error| {
            error.with_context(ErrorFrame::new("list_view.validate_colors").field("state", "item"))
        })?;
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
#[derive(Component)]
pub(crate) struct ColorState(
    pub(crate) WidgetryListViewColorOverrides,
    fn(&World, Entity) -> bool,
    fn(&World, Entity) -> Option<Entity>,
);
impl ColorState {
    pub(crate) fn new<T: Send + Sync + 'static, K: WidgetryListKey>() -> Self {
        Self(
            WidgetryListViewColorOverrides::default(),
            |world, entity| world.get::<crate::WidgetryListView<T, K>>(entity).is_some(),
            |world, entity| {
                world.get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryListView<T, K>>>(entity).map(|owner|owner.entity)
            },
        )
    }
}
impl WidgetryListViewColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, ListFailure> {
        world
            .get::<ColorState>(entity)
            .filter(|state| (state.1)(world, entity))
            .map(|state| &state.0)
            .ok_or_else(|| failure(ListError::InvalidColorTarget { entity }, "list_view.colors"))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, ListFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<ColorState>(entity)
            .and_then(|state| (state.2)(world, entity))
        {
            return Err(failure(
                ListError::ManagedColor { entity, owner },
                "list_view.set_colors",
            ));
        }
        colors.validate(entity).map_err(|error| {
            error.with_context(ErrorFrame::new("list_view.set_colors").field("view", entity))
        })?;
        let mut state = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| failure(ListError::InvalidColorTarget { entity }, "list_view.colors"))?;
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
                .map_err(ListFailure::into_bevy_error)
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, ListFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial<T: Send + Sync + 'static, K: WidgetryListKey>(
        self,
        entity: Entity,
    ) -> Result<ColorState, ListFailure> {
        self.validate(entity)?;
        let mut state = ColorState::new::<T, K>();
        state.0 = self;
        Ok(state)
    }
}
