use crate::projection::required;
use crate::{
    WidgetryTable, WidgetryTableColumnHeader, WidgetryTableColumnId, WidgetryTableEvent,
    WidgetryTableEventKind, WidgetryTableModel, WidgetryTableSortDirection, WidgetryTableViewState,
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, Button};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_internal_log::widgetry_error;
use std::collections::HashSet;

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableHeaderActions {
    pub sortable: HashSet<WidgetryTableColumnId>,
    pub filterable: HashSet<WidgetryTableColumnId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum WidgetryTableHeaderActionKind {
    Sort,
    Filter,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub struct WidgetryTableHeaderAction {
    pub column: WidgetryTableColumnId,
    pub kind: WidgetryTableHeaderActionKind,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Controls {
    sort: bool,
    filter: bool,
}

pub(crate) fn ensure(
    world: &mut World,
    root: Entity,
    header: Entity,
    column: WidgetryTableColumnId,
) -> Result {
    let config = world.get::<WidgetryTableHeaderActions>(root);
    let next = Controls {
        sort: config.is_some_and(|config| config.sortable.contains(&column)),
        filter: config.is_some_and(|config| config.filterable.contains(&column)),
    };
    let old = world.get::<Children>(header).and_then(|children| {
        children.iter().find_map(|entity| {
            world
                .get::<Controls>(entity)
                .map(|controls| (entity, *controls))
        })
    });
    if !next.sort && !next.filter {
        if let Some((entity, _)) = old {
            world.despawn(entity);
        }
        return Ok(());
    }
    let controls = if let Some((entity, _)) = old.filter(|(_, old)| *old == next) {
        entity
    } else {
        if let Some((entity, _)) = old {
            world.despawn(entity);
        }
        let buttons = [
            (next.sort, WidgetryTableHeaderActionKind::Sort),
            (next.filter, WidgetryTableHeaderActionKind::Filter),
        ]
        .into_iter()
        .filter(|(enabled, _)| *enabled)
        .map(|(_, kind)| bsn! {
            Button TabIndex(-1)
            template(move |_| Ok(WidgetryTableHeaderAction { column, kind }))
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { width: px(24), height: percent(100), flex_shrink: 0.0, align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            Children [@WidgetryIcon { @path: {BuiltinIcon::TableSort.path()}, @max_size: {Some(UVec2::splat(16))} }]
        })
        .collect::<Vec<_>>();
        let entity = spawn_scene(
            world,
            bsn! {
                template(move |_| Ok(next))
                Node { flex_shrink: 0.0, height: percent(100), margin: UiRect::right(px(6)) }
                Children [{buttons}]
            },
        )
        .map_err(BevyError::error)?;
        world.entity_mut(header).add_child(entity);
        entity
    };
    let state = required(world.get::<WidgetryTableViewState>(root))?;
    let sort = state
        .sort()
        .filter(|sort| sort.column == column)
        .map(|sort| sort.direction);
    let filtered = state.filters().contains(&column);
    let buttons = world
        .get::<Children>(controls)
        .map(|children| children.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    for button in buttons {
        let action = required(world.get::<WidgetryTableHeaderAction>(button))?;
        let icon = match action.kind {
            WidgetryTableHeaderActionKind::Sort => match sort {
                None => BuiltinIcon::TableSort,
                Some(WidgetryTableSortDirection::Ascending) => BuiltinIcon::ChevronUp,
                Some(WidgetryTableSortDirection::Descending) => BuiltinIcon::ChevronDown,
            },
            WidgetryTableHeaderActionKind::Filter => {
                if filtered {
                    BuiltinIcon::TableFilterActive
                } else {
                    BuiltinIcon::TableFilter
                }
            }
        };
        let icon_entity = required(required(world.get::<Children>(button))?.first().copied())?;
        let path = icon.path();
        if required(world.get::<WidgetryIcon>(icon_entity))?.path() != Some(&path) {
            WidgetryIcon::set_svg_in_world(world, icon_entity, path)?;
        }
    }
    Ok(())
}

pub(crate) fn on_activate<T: Send + Sync + 'static>(
    event: On<Activate>,
    actions: Query<(), With<WidgetryTableHeaderAction>>,
    mut commands: Commands,
) {
    if !actions.contains(event.entity) {
        return;
    }
    let target = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        activate::<T>(world, target)
            .inspect_err(|error| widgetry_error!(?target, %error, "Table 表头请求失败"))
    });
}

fn activate<T: Send + Sync + 'static>(world: &mut World, target: Entity) -> Result {
    let Some(action) = world.get::<WidgetryTableHeaderAction>(target).copied() else {
        return Ok(());
    };
    let mut entity = target;
    let mut matching_header = false;
    let root = loop {
        if world.get::<InteractionDisabled>(entity).is_some() {
            return Ok(());
        }
        if let Some(header) = world.get::<WidgetryTableColumnHeader>(entity) {
            matching_header = header.column == action.column;
        }
        if world.get::<WidgetryTable<T>>(entity).is_some() {
            break entity;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return Ok(());
        };
        entity = parent.parent();
    };
    if !matching_header {
        return Ok(());
    }
    let Some(config) = world.get::<WidgetryTableHeaderActions>(root) else {
        return Ok(());
    };
    let enabled = match action.kind {
        WidgetryTableHeaderActionKind::Sort => config.sortable.contains(&action.column),
        WidgetryTableHeaderActionKind::Filter => config.filterable.contains(&action.column),
    };
    if !enabled {
        return Ok(());
    }
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    if model.column_index(action.column).is_none() {
        return Ok(());
    }
    let state = required(world.get::<WidgetryTableViewState>(root))?;
    let kind = match action.kind {
        WidgetryTableHeaderActionKind::Sort => {
            let direction = match state
                .sort()
                .filter(|sort| sort.column == action.column)
                .map(|sort| sort.direction)
            {
                None => Some(WidgetryTableSortDirection::Ascending),
                Some(WidgetryTableSortDirection::Ascending) => {
                    Some(WidgetryTableSortDirection::Descending)
                }
                Some(WidgetryTableSortDirection::Descending) => None,
            };
            WidgetryTableEventKind::SortRequested {
                column: action.column,
                direction,
            }
        }
        WidgetryTableHeaderActionKind::Filter => {
            WidgetryTableEventKind::FilterRequested(action.column)
        }
    };
    world.trigger(WidgetryTableEvent { entity: root, kind });
    Ok(())
}
