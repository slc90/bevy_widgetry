use crate::{WindowError, WindowFailure, WindowFault, runtime_error};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowButtonStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryWindowButtonStateColorOverrides {
    fn validate(&self) -> Result<(), WindowFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| WindowError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "window")
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| WindowError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "window")
                            .field("field", "foreground"),
                    )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowButtonStateColors,
    ) -> bevy_widgetry_theme::WidgetryWindowButtonStateColors {
        bevy_widgetry_theme::WidgetryWindowButtonStateColors {
            background: self.background.unwrap_or(theme.background),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowButtonColorOverrides {
    pub normal: WidgetryWindowButtonStateColorOverrides,
    pub hovered: WidgetryWindowButtonStateColorOverrides,
    pub pressed: WidgetryWindowButtonStateColorOverrides,
    pub disabled: WidgetryWindowButtonStateColorOverrides,
}
impl WidgetryWindowButtonColorOverrides {
    fn validate(&self) -> Result<(), WindowFailure> {
        self.normal.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("state", "normal"))
        })?;
        self.hovered.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("state", "hovered"))
        })?;
        self.pressed.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("state", "pressed"))
        })?;
        self.disabled.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("state", "disabled"))
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowButtonColors,
    ) -> bevy_widgetry_theme::WidgetryWindowButtonColors {
        bevy_widgetry_theme::WidgetryWindowButtonColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryWindowStateColorOverrides {
    fn validate(&self) -> Result<(), WindowFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| WindowError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "window")
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| WindowError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "window")
                            .field("field", "border"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| WindowError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "window")
                            .field("field", "foreground"),
                    )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowStateColors,
    ) -> bevy_widgetry_theme::WidgetryWindowStateColors {
        bevy_widgetry_theme::WidgetryWindowStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowSurfaceColorOverrides {
    pub normal: WidgetryWindowStateColorOverrides,
    pub disabled: WidgetryWindowStateColorOverrides,
}
impl WidgetryWindowSurfaceColorOverrides {
    fn validate(&self) -> Result<(), WindowFailure> {
        self.normal.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("state", "normal"))
        })?;
        self.disabled.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("state", "disabled"))
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowSurfaceColors,
    ) -> bevy_widgetry_theme::WidgetryWindowSurfaceColors {
        bevy_widgetry_theme::WidgetryWindowSurfaceColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowColorOverrides {
    pub frame: WidgetryWindowSurfaceColorOverrides,
    pub title_bar: WidgetryWindowSurfaceColorOverrides,
    pub minimize: WidgetryWindowButtonColorOverrides,
    pub maximize: WidgetryWindowButtonColorOverrides,
    pub close: WidgetryWindowButtonColorOverrides,
    pub image_tint: Option<Color>,
}
impl WidgetryWindowColorOverrides {
    pub(crate) fn validate(&self) -> Result<(), WindowFailure> {
        self.frame.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("part", "frame"))
        })?;
        self.title_bar.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("part", "title_bar"))
        })?;
        self.minimize.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("part", "minimize"))
        })?;
        self.maximize.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("part", "maximize"))
        })?;
        self.close.validate().map_err(|error| {
            error.with_context(ErrorFrame::new("window.colors.validate").field("part", "close"))
        })?;
        if let Some(color) = self.image_tint {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| WindowError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "window")
                            .field("field", "image_tint"),
                    )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowColors,
    ) -> bevy_widgetry_theme::WidgetryWindowColors {
        bevy_widgetry_theme::WidgetryWindowColors {
            frame: self.frame.resolve(&theme.frame),
            title_bar: self.title_bar.resolve(&theme.title_bar),
            minimize: self.minimize.resolve(&theme.minimize),
            maximize: self.maximize.resolve(&theme.maximize),
            close: self.close.resolve(&theme.close),
            image_tint: self.image_tint.unwrap_or(theme.image_tint),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryWindowColorOverrides);
impl WidgetryWindowColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, WindowFailure> {
        if world
            .get::<crate::window_root::WindowRoot>(entity)
            .is_none()
        {
            return Err(runtime_error::failure(
                world,
                entity,
                WindowFault::MissingPart {
                    entity,
                    component: "WindowRoot",
                },
                "window.colors.get",
            ));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| {
                runtime_error::failure(
                    world,
                    entity,
                    WindowFault::MissingPart {
                        entity,
                        component: "ColorState",
                    },
                    "window.colors.get",
                )
            })
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, WindowFailure> {
        Self::get(world, entity).map_err(|error| {
            error.with_context(runtime_error::frame(world, entity, "window.colors.set"))
        })?;
        if let Some(owner) = world.get::<bevy_widgetry_core::color::WidgetryStyleOwner<
            crate::window_root::WindowRoot,
        >>(entity)
        {
            return Err(runtime_error::failure(
                world,
                entity,
                WindowFault::ManagedColors {
                    entity,
                    owner: owner.entity,
                },
                "window.colors.set",
            ));
        }
        let frame = runtime_error::frame(world, entity, "window.colors.set");
        colors
            .validate()
            .map_err(|error| error.with_context(frame.clone()))?;
        let mut state = world.get_mut::<ColorState>(entity).ok_or_else(|| {
            WindowFailure::new(
                WindowError::Structure {
                    fault: WindowFault::MissingPart {
                        entity,
                        component: "ColorState",
                    },
                },
                frame,
            )
        })?;
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
                .map_err(WindowFailure::into_bevy_error)
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, WindowFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self) -> Result<ColorState, WindowFailure> {
        self.validate()?;
        Ok(ColorState(self))
    }
}
