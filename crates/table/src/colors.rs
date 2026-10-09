use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTableStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryTableStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTableStateColors,
    ) -> bevy_widgetry_theme::WidgetryTableStateColors {
        bevy_widgetry_theme::WidgetryTableStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTableRegionColorOverrides {
    pub normal: WidgetryTableStateColorOverrides,
    pub hovered: WidgetryTableStateColorOverrides,
    pub selected: WidgetryTableStateColorOverrides,
    pub disabled: WidgetryTableStateColorOverrides,
    pub focused_border: Option<Color>,
    pub disabled_focused_border: Option<Color>,
}
impl WidgetryTableRegionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.selected.validate()?;
        self.disabled.validate()?;
        if let Some(color) = self.focused_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.disabled_focused_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTableRegionColors,
    ) -> bevy_widgetry_theme::WidgetryTableRegionColors {
        bevy_widgetry_theme::WidgetryTableRegionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            selected: self.selected.resolve(&theme.selected),
            disabled: self.disabled.resolve(&theme.disabled),
            focused_border: self.focused_border.unwrap_or(theme.focused_border),
            disabled_focused_border: self
                .disabled_focused_border
                .unwrap_or(theme.disabled_focused_border),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTableColorOverrides {
    pub table: WidgetryTableRegionColorOverrides,
    pub column_header: WidgetryTableRegionColorOverrides,
    pub row_header: WidgetryTableRegionColorOverrides,
    pub corner: WidgetryTableRegionColorOverrides,
    pub cell: WidgetryTableRegionColorOverrides,
}
impl WidgetryTableColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.table.validate()?;
        self.column_header.validate()?;
        self.row_header.validate()?;
        self.corner.validate()?;
        self.cell.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTableColors,
    ) -> bevy_widgetry_theme::WidgetryTableColors {
        bevy_widgetry_theme::WidgetryTableColors {
            table: self.table.resolve(&theme.table),
            column_header: self.column_header.resolve(&theme.column_header),
            row_header: self.row_header.resolve(&theme.row_header),
            corner: self.corner.resolve(&theme.corner),
            cell: self.cell.resolve(&theme.cell),
        }
    }
}
#[derive(Component)]
pub(crate) struct ColorState(
    pub(crate) WidgetryTableColorOverrides,
    fn(&World, Entity) -> bool,
    fn(&World, Entity) -> bool,
);
impl ColorState {
    pub(crate) fn new<T: Send + Sync + 'static>() -> Self {
        Self(
            WidgetryTableColorOverrides::default(),
            |world, entity| world.get::<crate::WidgetryTable<T>>(entity).is_some(),
            |world, entity| {
                world
                    .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryTable<T>>>(
                        entity,
                    )
                    .is_some()
            },
        )
    }
}
impl WidgetryTableColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
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
    ) -> Result<bool, BevyError> {
        Self::get(world, entity)?;
        if world
            .get::<ColorState>(entity)
            .is_some_and(|state| (state.2)(world, entity))
        {
            return Err(invalid_target(entity));
        }
        colors.validate()?;
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
        commands
            .queue(move |world: &mut World| Self::set_in_world(world, entity, colors).map(|_| ()));
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial<T: Send + Sync + 'static>(self) -> Result<ColorState, BevyError> {
        self.validate()?;
        let mut state = ColorState::new::<T>();
        state.0 = self;
        Ok(state)
    }
}
#[cold]
fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "WidgetryTableColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryTableColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
