use crate::{MessageBoxError, MessageBoxFailure, MessageBoxFault, error, runtime_error};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryButtonStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryButtonStateColorOverrides {
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
                            .field("field", "border"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
                            .field("field", "foreground"),
                    )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryButtonStateColors,
    ) -> bevy_widgetry_theme::WidgetryButtonStateColors {
        bevy_widgetry_theme::WidgetryButtonStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryButtonColorOverrides {
    pub normal: WidgetryButtonStateColorOverrides,
    pub hovered: WidgetryButtonStateColorOverrides,
    pub pressed: WidgetryButtonStateColorOverrides,
    pub disabled: WidgetryButtonStateColorOverrides,
}
impl WidgetryButtonColorOverrides {
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        self.normal.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "normal"),
            )
        })?;
        self.hovered.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "hovered"),
            )
        })?;
        self.pressed.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "pressed"),
            )
        })?;
        self.disabled.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "disabled"),
            )
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryButtonColors,
    ) -> bevy_widgetry_theme::WidgetryButtonColors {
        bevy_widgetry_theme::WidgetryButtonColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryMessageBoxBodyStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryMessageBoxBodyStateColorOverrides {
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
                            .field("field", "foreground"),
                    )
            })?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryMessageBoxBodyStateColors,
    ) -> bevy_widgetry_theme::WidgetryMessageBoxBodyStateColors {
        bevy_widgetry_theme::WidgetryMessageBoxBodyStateColors {
            background: self.background.unwrap_or(theme.background),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryMessageBoxBodyColorOverrides {
    pub normal: WidgetryMessageBoxBodyStateColorOverrides,
    pub disabled: WidgetryMessageBoxBodyStateColorOverrides,
}
impl WidgetryMessageBoxBodyColorOverrides {
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        self.normal.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "normal"),
            )
        })?;
        self.disabled.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "disabled"),
            )
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryMessageBoxBodyColors,
    ) -> bevy_widgetry_theme::WidgetryMessageBoxBodyColors {
        bevy_widgetry_theme::WidgetryMessageBoxBodyColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowButtonStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryWindowButtonStateColorOverrides {
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
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
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        self.normal.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "normal"),
            )
        })?;
        self.hovered.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "hovered"),
            )
        })?;
        self.pressed.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "pressed"),
            )
        })?;
        self.disabled.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "disabled"),
            )
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
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
                            .field("field", "background"),
                    )
            })?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
                            .field("field", "border"),
                    )
            })?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
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
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        self.normal.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "normal"),
            )
        })?;
        self.disabled.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("state", "disabled"),
            )
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
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        self.frame.validate().map_err(|failure| {
            failure
                .with_context(ErrorFrame::new("message_box.colors.validate").field("part", "frame"))
        })?;
        self.title_bar.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("part", "title_bar"),
            )
        })?;
        self.minimize.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("part", "minimize"),
            )
        })?;
        self.maximize.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("part", "maximize"),
            )
        })?;
        self.close.validate().map_err(|failure| {
            failure
                .with_context(ErrorFrame::new("message_box.colors.validate").field("part", "close"))
        })?;
        if let Some(color) = self.image_tint {
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .map_kind(|source| MessageBoxError::Core {
                        source: Box::new(source),
                    })
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "message_box")
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryMessageBoxColorOverrides {
    pub window: WidgetryWindowColorOverrides,
    pub body: WidgetryMessageBoxBodyColorOverrides,
    pub action_button: WidgetryButtonColorOverrides,
}
impl WidgetryMessageBoxColorOverrides {
    fn validate(&self) -> Result<(), MessageBoxFailure> {
        self.window.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("part", "window"),
            )
        })?;
        self.body.validate().map_err(|failure| {
            failure
                .with_context(ErrorFrame::new("message_box.colors.validate").field("part", "body"))
        })?;
        self.action_button.validate().map_err(|failure| {
            failure.with_context(
                ErrorFrame::new("message_box.colors.validate").field("part", "action_button"),
            )
        })?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryMessageBoxColors,
    ) -> bevy_widgetry_theme::WidgetryMessageBoxColors {
        bevy_widgetry_theme::WidgetryMessageBoxColors {
            window: self.window.resolve(&theme.window),
            body: self.body.resolve(&theme.body),
            action_button: self.action_button.resolve(&theme.action_button),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryMessageBoxColorOverrides);
impl WidgetryMessageBoxColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, MessageBoxFailure> {
        if world.get::<crate::WidgetryMessageBox>(entity).is_none() {
            return Err(runtime_error::failure(
                world,
                entity,
                entity,
                MessageBoxFault::MissingPart {
                    entity,
                    component: "WidgetryMessageBox",
                },
                "message_box.colors.get",
            ));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| {
                runtime_error::failure(
                    world,
                    entity,
                    entity,
                    MessageBoxFault::MissingPart {
                        entity,
                        component: "ColorState",
                    },
                    "message_box.colors.get",
                )
            })
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, MessageBoxFailure> {
        Self::get(world, entity).map_err(|failure| {
            failure.with_context(error::frame(
                world,
                entity,
                entity,
                "message_box.colors.set",
            ))
        })?;
        if let Some(owner) = world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryMessageBox>>(entity)
        {
            return Err(runtime_error::failure(
                world,
                entity,
                entity,
                MessageBoxFault::ManagedColors {
                    entity,
                    owner: owner.entity,
                },
                "message_box.colors.set",
            ));
        }
        colors.validate().map_err(|failure| {
            failure.with_context(error::frame(
                world,
                entity,
                entity,
                "message_box.colors.set",
            ))
        })?;
        let Some(mut state) = world.get_mut::<ColorState>(entity) else {
            return Err(runtime_error::failure(
                world,
                entity,
                entity,
                MessageBoxFault::MissingPart {
                    entity,
                    component: "ColorState",
                },
                "message_box.colors.set",
            ));
        };
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
                .map_err(MessageBoxFailure::into_bevy_error)
        });
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, MessageBoxFailure> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self) -> Result<ColorState, MessageBoxFailure> {
        self.validate()?;
        Ok(ColorState(self))
    }
}
