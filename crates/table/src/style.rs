use crate::WidgetryTable;
use crate::projection::{TableRuntime, required};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Selected};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Clone, Default)]
pub struct WidgetryTableRegionStyle {
    pub padding: UiRect,
    pub border: UiRect,
}

#[derive(Component, Clone)]
pub struct WidgetryTableStyle {
    pub table: WidgetryTableRegionStyle,
    pub column_header: WidgetryTableRegionStyle,
    pub row_header: WidgetryTableRegionStyle,
    pub corner: WidgetryTableRegionStyle,
    pub cell: WidgetryTableRegionStyle,
}

impl Default for WidgetryTableStyle {
    fn default() -> Self {
        let item = WidgetryTableRegionStyle {
            padding: UiRect::horizontal(px(6)),
            border: UiRect::all(px(1)),
        };
        Self {
            table: WidgetryTableRegionStyle {
                border: UiRect::all(px(1)),
                ..default()
            },
            column_header: item.clone(),
            row_header: item.clone(),
            corner: item.clone(),
            cell: item,
        }
    }
}

fn style_shell(
    world: &mut World,
    root: Entity,
    entity: Entity,
    style: &WidgetryTableRegionStyle,
    colors: &bevy_widgetry_theme::WidgetryTableRegionColors,
    disabled: bool,
) -> Result<(), BevyError> {
    let (selected, focused) = crate::interaction::appearance(world, root, entity);
    if selected {
        if world.get::<Selected>(entity).is_none() {
            world.entity_mut(entity).insert(Selected);
        }
    } else if world.get::<Selected>(entity).is_some() {
        world.entity_mut(entity).remove::<Selected>();
    }
    let hovered = world
        .get::<Hovered>(entity)
        .is_some_and(|hovered| hovered.0);
    let disabled = disabled || world.get::<InteractionDisabled>(entity).is_some();
    let state = if disabled {
        colors.disabled
    } else if hovered {
        colors.hovered
    } else if selected {
        colors.selected
    } else {
        colors.normal
    };
    let background = state.background;
    let border = if focused {
        if disabled {
            colors.disabled_focused_border
        } else {
            colors.focused_border
        }
    } else {
        state.border
    };
    let foreground = state.foreground;
    required(world.get_mut::<BackgroundColor>(entity))?.set_if_neq(BackgroundColor(background));
    required(world.get_mut::<BorderColor>(entity))?.set_if_neq(BorderColor::all(border));
    if let Some(mut color) = world.get_mut::<ResolvedForeground>(entity)
        && *color != ResolvedForeground(foreground)
    {
        color.set_if_neq(ResolvedForeground(foreground));
    }
    apply_geometry(world, entity, style)
}

pub(super) fn apply_geometry(
    world: &mut World,
    entity: Entity,
    style: &WidgetryTableRegionStyle,
) -> Result<(), BevyError> {
    let mut node = required(world.get_mut::<Node>(entity))?;
    if node.padding != style.padding {
        node.padding = style.padding;
    }
    if node.border != style.border {
        node.border = style.border;
    }
    Ok(())
}

pub(crate) fn update_colors<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let theme = world.resource::<WidgetryThemeMode>().colors().table;
    let roots = world
        .query_filtered::<Entity, With<WidgetryTable<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let Some(runtime) = world.get::<TableRuntime>(root).cloned() else {
            continue;
        };
        let style = required(world.get::<WidgetryTableStyle>(root))?.clone();
        let colors = required(world.get::<crate::colors::ColorState>(root))?
            .0
            .resolve(&theme);
        let disabled = world.get::<InteractionDisabled>(root).is_some();
        style_shell(world, root, root, &style.table, &colors.table, disabled)?;
        style_shell(
            world,
            root,
            runtime.corner,
            &style.corner,
            &colors.corner,
            disabled,
        )?;
        for entity in runtime.cells.values() {
            style_shell(world, root, *entity, &style.cell, &colors.cell, disabled)?;
        }
        for entity in runtime.column_headers.values() {
            style_shell(
                world,
                root,
                *entity,
                &style.column_header,
                &colors.column_header,
                disabled,
            )?;
        }
        for entity in runtime.row_headers.values() {
            style_shell(
                world,
                root,
                *entity,
                &style.row_header,
                &colors.row_header,
                disabled,
            )?;
        }
    }
    Ok(())
}
pub(crate) fn refresh_theme<T: Send + Sync + 'static>(
    _event: On<bevy_widgetry_theme::WidgetryThemeChanged>,
    mut commands: Commands,
) {
    commands.queue(update_colors::<T>);
}
