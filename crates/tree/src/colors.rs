use crate::{TreeError, TreeFailure, error::failure};
use bevy::prelude::*;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTreeExpanderStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryTreeExpanderStateColorOverrides {
    fn validate(&self, view: Entity) -> Result<(), TreeFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
                            .field("field", "border"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
                            .field("field", "foreground"),
                    )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTreeExpanderStateColors,
    ) -> bevy_widgetry_theme::WidgetryTreeExpanderStateColors {
        bevy_widgetry_theme::WidgetryTreeExpanderStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTreeExpanderInteractionColorOverrides {
    pub normal: WidgetryTreeExpanderStateColorOverrides,
    pub hovered: WidgetryTreeExpanderStateColorOverrides,
    pub pressed: WidgetryTreeExpanderStateColorOverrides,
    pub disabled: WidgetryTreeExpanderStateColorOverrides,
}
impl WidgetryTreeExpanderInteractionColorOverrides {
    fn validate(&self, view: Entity) -> Result<(), TreeFailure> {
        self.normal.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "normal"),
            )
        })?;
        self.hovered.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "hovered"),
            )
        })?;
        self.pressed.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "pressed"),
            )
        })?;
        self.disabled.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "disabled"),
            )
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTreeExpanderInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryTreeExpanderInteractionColors {
        bevy_widgetry_theme::WidgetryTreeExpanderInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTreeExpanderColorOverrides {
    pub collapsed: WidgetryTreeExpanderInteractionColorOverrides,
    pub expanded: WidgetryTreeExpanderInteractionColorOverrides,
}
impl WidgetryTreeExpanderColorOverrides {
    fn validate(&self, view: Entity) -> Result<(), TreeFailure> {
        self.collapsed.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors")
                    .field("part", "collapsed"),
            )
        })?;
        self.expanded.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "expanded"),
            )
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTreeExpanderColors,
    ) -> bevy_widgetry_theme::WidgetryTreeExpanderColors {
        bevy_widgetry_theme::WidgetryTreeExpanderColors {
            collapsed: self.collapsed.resolve(&theme.collapsed),
            expanded: self.expanded.resolve(&theme.expanded),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewItemStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryListViewItemStateColorOverrides {
    fn validate(&self, view: Entity) -> Result<(), TreeFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
                            .field("field", "border"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
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
    fn validate(&self, view: Entity) -> Result<(), TreeFailure> {
        self.normal.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "normal"),
            )
        })?;
        self.hovered.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "hovered"),
            )
        })?;
        self.pressed.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "pressed"),
            )
        })?;
        self.selected.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "selected"),
            )
        })?;
        self.disabled.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "disabled"),
            )
        })?;
        if let Some(color) = self.active_border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
                            .field("field", "active_border"),
                    )
            })?;
        }
        if let Some(color) = self.disabled_active_border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
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
    fn validate(&self, view: Entity) -> Result<(), TreeFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
                            .field("field", "border"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| TreeError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("view", view)
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
    fn validate(&self, view: Entity) -> Result<(), TreeFailure> {
        self.normal.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "normal"),
            )
        })?;
        self.focused.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "focused"),
            )
        })?;
        self.disabled.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "disabled"),
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
pub struct WidgetryTreeColorOverrides {
    pub container: WidgetryListViewContainerColorOverrides,
    pub item: WidgetryListViewItemColorOverrides,
    pub expander: WidgetryTreeExpanderColorOverrides,
}
impl WidgetryTreeColorOverrides {
    fn validate(&self, view: Entity) -> Result<(), TreeFailure> {
        self.container.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors")
                    .field("part", "container"),
            )
        })?;
        self.item.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "item"),
            )
        })?;
        self.expander.validate(view).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("tree.colors").field("part", "expander"),
            )
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTreeColors,
    ) -> bevy_widgetry_theme::WidgetryTreeColors {
        bevy_widgetry_theme::WidgetryTreeColors {
            container: self.container.resolve(&theme.container),
            item: self.item.resolve(&theme.item),
            expander: self.expander.resolve(&theme.expander),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryTreeColorOverrides);
impl WidgetryTreeColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, TreeFailure> {
        if world.get::<crate::WidgetryTreeView>(entity).is_none() {
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
    ) -> Result<bool, TreeFailure> {
        Self::get(world, entity)?;
        if let Some(owner) = world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTreeView>>(entity)
        {
            return Err(failure(
                TreeError::ManagedColors {
                    view: entity,
                    owner: owner.entity,
                },
                "tree.colors.set",
            ));
        }
        colors.validate(entity)?;
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
                .map_err(TreeFailure::into_bevy_error)
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, TreeFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self, view: Entity) -> Result<ColorState, TreeFailure> {
        self.validate(view)?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(entity: Entity) -> TreeFailure {
    failure(
        TreeError::InvalidColors { view: entity },
        "tree.colors.target",
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    #[test]
    fn invalid_override_keeps_core_source_and_managed_owner_identity() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(crate::WidgetryTreePlugin);
        let root = app.world_mut().spawn_empty().id();
        let source = app
            .world_mut()
            .spawn(crate::WidgetryTreeModel::new(root))
            .id();
        let view = bevy_widgetry_core::scene::spawn_scene(
            app.world_mut(),
            bsn! {@crate::WidgetryTreeView{@source:source}},
        )
        .unwrap();
        let original = WidgetryTreeColorOverrides::get(app.world(), view)
            .unwrap()
            .clone();
        let mut invalid = original.clone();
        invalid.expander.expanded.pressed.foreground =
            Some(Color::linear_rgba(f32::NAN, 0., 0., 1.));
        let error =
            WidgetryTreeColorOverrides::set_in_world(app.world_mut(), view, invalid).unwrap_err();
        assert!(matches!(error.kind(), TreeError::Core { .. }));
        assert!(std::error::Error::source(error.kind()).is_some());
        for field in ["expander", "expanded", "pressed", "foreground"] {
            assert!(
                error
                    .contexts()
                    .iter()
                    .any(|frame| frame.fields().iter().any(|(_, value)| value == field))
            );
        }
        assert_eq!(
            WidgetryTreeColorOverrides::get(app.world(), view).unwrap(),
            &original
        );
        let owner = app.world_mut().spawn_empty().id();
        app.world_mut()
            .entity_mut(view)
            .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<
                crate::WidgetryTreeView,
            >::new(owner));
        let error = WidgetryTreeColorOverrides::clear_in_world(app.world_mut(), view).unwrap_err();
        assert!(
            matches!(error.kind(),TreeError::ManagedColors{view:actual,owner:actual_owner} if (*actual,*actual_owner)==(view,owner))
        );
    }
}
