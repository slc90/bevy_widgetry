use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWaveformStateColorOverrides {
    pub background: Option<Color>,
    pub palette: Option<Vec<Color>>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWaveformColorOverrides {
    pub normal: WidgetryWaveformStateColorOverrides,
    pub disabled: WidgetryWaveformStateColorOverrides,
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryWaveformColorOverrides);
#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) struct ResolvedColors {
    pub(crate) background: Color,
    pub(crate) palette: Vec<Color>,
}
impl Default for ResolvedColors {
    fn default() -> Self {
        let colors = bevy_widgetry_theme::WIDGETRY_DARK_THEME.waveform.normal;
        Self {
            background: colors.background,
            palette: colors.palette.to_vec(),
        }
    }
}
impl WidgetryWaveformStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(palette) = &self.palette {
            if palette.len() < 2
                || palette
                    .iter()
                    .zip(palette.iter().cycle().skip(1))
                    .any(|(a, b)| a.to_linear() == b.to_linear())
            {
                widgetry_error!(
                    count = palette.len(),
                    "Waveform palette 至少两色且循环相邻颜色不能相同"
                );
                return Err(BevyError::error(
                    "Waveform palette 至少两色且循环相邻颜色不能相同",
                ));
            }
            for &color in palette {
                bevy_widgetry_core::color::validate_color(color)?;
            }
        }
        Ok(())
    }
}
impl WidgetryWaveformColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()
    }
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::Waveform>(entity).is_none() {
            return Err(invalid(entity));
        }
        world
            .get::<ColorState>(entity)
            .map(|c| &c.0)
            .ok_or_else(|| invalid(entity))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, BevyError> {
        Self::get(world, entity)?;
        colors.validate()?;
        let mut state = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| invalid(entity))?;
        if state.0 == colors {
            return Ok(false);
        }
        state.0 = colors;
        Ok(true)
    }
    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands
            .queue(move |world: &mut World| Self::set_in_world(world, entity, colors).map(|_| ()));
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self) -> Result<ColorState, BevyError> {
        self.validate()?;
        Ok(ColorState(self))
    }
}
fn invalid(entity: Entity) -> BevyError {
    widgetry_error!(?entity, "Waveform 颜色目标不存在或类型不匹配");
    BevyError::error("Waveform 颜色目标不存在或类型不匹配")
}
fn apply_colors(world: &mut World, roots: Vec<Entity>) {
    let theme = world
        .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
        .colors()
        .waveform;
    for entity in roots {
        let Some(overrides) = world.get::<ColorState>(entity).map(|state| state.0.clone()) else {
            continue;
        };
        let disabled = world.get::<bevy::ui::InteractionDisabled>(entity).is_some();
        let (overrides, theme) = if disabled {
            (&overrides.disabled, theme.disabled)
        } else {
            (&overrides.normal, theme.normal)
        };
        let color = ResolvedColors {
            background: overrides.background.unwrap_or(theme.background),
            palette: overrides
                .palette
                .as_deref()
                .unwrap_or(theme.palette)
                .to_vec(),
        };
        if world.get::<ResolvedColors>(entity) != Some(&color) {
            world.entity_mut(entity).insert(color);
        }
    }
}
pub(crate) fn update(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, (
            With<ColorState>,
            Or<(
                Changed<ColorState>,
                Changed<bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>,
                Added<crate::Waveform>,
            )>,
        )>()
        .iter(world)
        .collect::<Vec<_>>();
    apply_colors(world, roots);
}
pub(crate) fn refresh(
    _event: On<bevy_widgetry_theme::WidgetryThemeChanged>,
    mut commands: Commands,
) {
    commands.queue(refresh_all);
}
fn refresh_all(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<ColorState>>()
        .iter(world)
        .collect::<Vec<_>>();
    apply_colors(world, roots);
}
