use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use bevy_widgetry_theme::WidgetryProgressBarColors;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryProgressBarColorOverrides {
    pub track: Option<Color>,
    pub fill: Option<Color>,
    pub border: Option<Color>,
}

#[derive(Component)]
pub(crate) struct ColorState(pub(crate) WidgetryProgressBarColorOverrides);

impl WidgetryProgressBarColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        for (field, color) in [
            ("track", self.track),
            ("fill", self.fill),
            ("border", self.border),
        ] {
            let Some(color) = color else {
                continue;
            };
            bevy_widgetry_core::color::validate_color(color).map_err(|error| {
                error
                    .with_context(
                        bevy_widgetry_core::scene::ErrorFrame::new("validate widget colors")
                            .field("consumer", "progress_bar")
                            .field("field", field),
                    )
                    .into_bevy_error()
            })?;
        }
        Ok(())
    }

    pub(crate) fn initial(self) -> Result<ColorState, BevyError> {
        self.validate()?;
        Ok(ColorState(self))
    }

    pub(crate) fn resolve(&self, theme: &WidgetryProgressBarColors) -> WidgetryProgressBarColors {
        WidgetryProgressBarColors {
            track: self.track.unwrap_or(theme.track),
            fill: self.fill.unwrap_or(theme.fill),
            border: self.border.unwrap_or(theme.border),
        }
    }

    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryProgressBar>(entity).is_none() {
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
    ) -> Result<bool, BevyError> {
        Self::get(world, entity)?;
        bevy::ecs::error::ContextExt::with_context(colors.validate(), || {
            format!("set widget colors: root={entity}")
        })?;
        let mut current = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| invalid_target(entity))?;
        if current.0 == colors {
            return Ok(false);
        }
        current.0 = colors;
        Ok(true)
    }

    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands
            .queue(move |world: &mut World| Self::set_in_world(world, entity, colors).map(|_| ()));
    }

    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        Self::set_in_world(world, entity, default())
    }

    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, default());
    }
}

fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "ProgressBar 颜色目标不存在、类型不匹配或内部状态缺失"
    );
    BevyError::error("ProgressBar color target is missing or invalid")
}
