use crate::{WidgetryTable, WidgetryTableColumnId, WidgetryTableModel, WidgetryTableRowId};
use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum WidgetryTableSortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct WidgetryTableSort {
    pub column: WidgetryTableColumnId,
    pub direction: WidgetryTableSortDirection,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetryTableViewUpdate {
    pub rows: Vec<WidgetryTableRowId>,
    pub sort: Option<WidgetryTableSort>,
    pub filters: HashSet<WidgetryTableColumnId>,
}

#[derive(Component, Clone, Debug, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub struct WidgetryTableViewState {
    rows: Vec<WidgetryTableRowId>,
    indices: HashMap<WidgetryTableRowId, usize>,
    sort: Option<WidgetryTableSort>,
    filters: HashSet<WidgetryTableColumnId>,
    controlled: bool,
    source: Entity,
    #[reflect(ignore)]
    tick: Tick,
    structure_version: u64,
}

impl Default for WidgetryTableViewState {
    fn default() -> Self {
        Self {
            rows: default(),
            indices: default(),
            sort: None,
            filters: default(),
            controlled: false,
            source: Entity::PLACEHOLDER,
            tick: Tick::new(0),
            structure_version: 0,
        }
    }
}

impl WidgetryTableViewState {
    pub fn ordered_rows(&self) -> &[WidgetryTableRowId] {
        &self.rows
    }
    pub fn row_id(&self, display_index: usize) -> Option<WidgetryTableRowId> {
        self.rows.get(display_index).copied()
    }
    pub fn display_index(&self, row: WidgetryTableRowId) -> Option<usize> {
        self.indices.get(&row).copied()
    }
    pub fn sort(&self) -> Option<WidgetryTableSort> {
        self.sort
    }
    pub fn filters(&self) -> &HashSet<WidgetryTableColumnId> {
        &self.filters
    }
    pub fn is_controlled(&self) -> bool {
        self.controlled
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn reindex(&mut self) {
        self.indices = self
            .rows
            .iter()
            .copied()
            .enumerate()
            .map(|(index, row)| (row, index))
            .collect();
    }
}

impl<T: Send + Sync + 'static> WidgetryTable<T> {
    pub fn update_view(
        world: &mut World,
        root: Entity,
        update: WidgetryTableViewUpdate,
    ) -> Result<bool, BevyError> {
        update_view::<T>(world, root, Some(update))
            .inspect_err(|error| widgetry_error!(?root, %error, "Table View 投影更新失败"))
    }

    pub fn queue_update_view(
        commands: &mut Commands,
        root: Entity,
        update: WidgetryTableViewUpdate,
    ) {
        commands.queue(move |world: &mut World| Self::update_view(world, root, update).map(|_| ()));
    }

    pub fn reset_view(world: &mut World, root: Entity) -> Result<bool, BevyError> {
        update_view::<T>(world, root, None)
            .inspect_err(|error| widgetry_error!(?root, %error, "Table View 默认投影恢复失败"))
    }

    pub fn queue_reset_view(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| Self::reset_view(world, root).map(|_| ()));
    }
}

fn update_view<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    update: Option<WidgetryTableViewUpdate>,
) -> Result<bool, BevyError> {
    let source = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table View update requires a live matching View"))?
        .source();
    let entity = world.get_entity(source).map_err(BevyError::error)?;
    let model = entity
        .get_ref::<WidgetryTableModel<T>>()
        .ok_or_else(|| BevyError::error("Table View update requires a live matching Model"))?;
    let mut next = WidgetryTableViewState {
        source,
        tick: model.last_changed(),
        structure_version: model.structure_version(),
        ..default()
    };
    if let Some(update) = update {
        let mut seen = HashSet::new();
        if update
            .rows
            .iter()
            .any(|row| model.row_index(*row).is_none() || !seen.insert(*row))
            || update
                .sort
                .is_some_and(|sort| model.column_index(sort.column).is_none())
            || update
                .filters
                .iter()
                .any(|column| model.column_index(*column).is_none())
        {
            return Err(BevyError::error(
                "Table View update contains duplicate or stale source-local identities",
            ));
        }
        next.rows = update.rows;
        next.sort = update.sort;
        next.filters = update.filters;
        next.controlled = true;
    } else {
        next.rows = (0..model.row_count())
            .filter_map(|index| model.row_id(index))
            .collect();
    }
    next.reindex();
    let same = world
        .get::<WidgetryTableViewState>(root)
        .is_some_and(|old| {
            old.source == source
                && old.rows == next.rows
                && old.sort == next.sort
                && old.filters == next.filters
                && old.controlled == next.controlled
        });
    if same {
        return Ok(false);
    }
    world.entity_mut(root).insert(next);
    Ok(true)
}

pub(crate) fn refresh<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), BevyError> {
    let source = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table row projection requires a live View"))?
        .source();
    let entity = world.get_entity(source).map_err(BevyError::error)?;
    let model = entity
        .get_ref::<WidgetryTableModel<T>>()
        .ok_or_else(|| BevyError::error("Table row projection requires a matching Model"))?;
    let tick = model.last_changed();
    // A second structural edit after a synchronous API may carry the same ECS tick.
    let old = world.get::<WidgetryTableViewState>(root);
    if old.is_some_and(|old| {
        old.source == source
            && old.tick == tick
            && old.structure_version == model.structure_version()
    }) {
        return Ok(());
    }
    let mut next = old
        .filter(|old| old.source == source)
        .cloned()
        .unwrap_or_default();
    if next.controlled {
        next.rows.retain(|row| model.row_index(*row).is_some());
        if next
            .sort
            .is_some_and(|sort| model.column_index(sort.column).is_none())
        {
            next.sort = None;
        }
        next.filters
            .retain(|column| model.column_index(*column).is_some());
    } else {
        next.rows = (0..model.row_count())
            .filter_map(|index| model.row_id(index))
            .collect();
    }
    next.source = source;
    next.tick = tick;
    next.structure_version = model.structure_version();
    next.reindex();
    world.entity_mut(root).insert(next);
    Ok(())
}
