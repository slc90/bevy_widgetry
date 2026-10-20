use crate::{WidgetryTable, WidgetryTableColumnId, WidgetryTableModel, WidgetryTableRowId};
use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
    columns: Vec<WidgetryTableColumnId>,
    column_indices: HashMap<WidgetryTableColumnId, usize>,
    manual_columns: bool,
    #[reflect(ignore)]
    instance: Option<Arc<()>>,
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
            columns: default(),
            column_indices: default(),
            manual_columns: false,
            instance: None,
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
    pub fn ordered_columns(&self) -> &[WidgetryTableColumnId] {
        &self.columns
    }
    pub fn column_id(&self, display_index: usize) -> Option<WidgetryTableColumnId> {
        self.columns.get(display_index).copied()
    }
    pub fn column_display_index(&self, column: WidgetryTableColumnId) -> Option<usize> {
        self.column_indices.get(&column).copied()
    }
    pub fn has_manual_column_order(&self) -> bool {
        self.manual_columns
    }
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

    fn matches_model(&self, source: Entity, instance: &Arc<()>) -> bool {
        self.source == source
            && self
                .instance
                .as_ref()
                .is_some_and(|old| Arc::ptr_eq(old, instance))
    }

    fn repair_columns<T: Send + Sync + 'static>(&mut self, model: &WidgetryTableModel<T>) {
        if self.manual_columns {
            self.columns
                .retain(|column| model.column_index(*column).is_some());
            let mut seen = self.columns.iter().copied().collect::<HashSet<_>>();
            self.columns.extend(
                (0..model.column_count())
                    .filter_map(|i| model.column_id(i))
                    .filter(|id| seen.insert(*id)),
            );
        } else {
            self.columns = (0..model.column_count())
                .filter_map(|i| model.column_id(i))
                .collect();
        }
    }

    fn reindex(&mut self) {
        self.column_indices = self
            .columns
            .iter()
            .copied()
            .enumerate()
            .map(|(i, c)| (c, i))
            .collect();
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
    pub fn reset_column_order(world: &mut World, root: Entity) -> Result<bool, BevyError> {
        (|| {
            refresh::<T>(world, root)?;
            crate::column_reorder::cancel(world, root);
            let mut next = world
                .get::<WidgetryTableViewState>(root)
                .cloned()
                .ok_or_else(|| BevyError::error("Table column reset requires View state"))?;
            if !next.manual_columns {
                return Ok(false);
            }
            let model = world
                .get::<WidgetryTableModel<T>>(next.source)
                .ok_or_else(|| BevyError::error("Table column reset requires Model"))?;
            next.columns = (0..model.column_count())
                .filter_map(|i| model.column_id(i))
                .collect();
            next.manual_columns = false;
            next.reindex();
            world.entity_mut(root).insert(next);
            world.trigger(crate::WidgetryTableEvent {
                entity: root,
                kind: crate::WidgetryTableEventKind::ColumnOrderChanged,
            });
            Ok(true)
        })()
        .inspect_err(|error| widgetry_error!(?root,%error,"Table 列顺序重置失败"))
    }

    pub fn queue_reset_column_order(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| Self::reset_column_order(world, root).map(|_| ()));
    }

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
    let previous = world
        .get::<WidgetryTableViewState>(root)
        .filter(|old| old.matches_model(source, model.instance()));
    let mut next = WidgetryTableViewState {
        columns: previous.map_or_else(Vec::new, |old| old.columns.clone()),
        manual_columns: previous.is_some_and(|old| old.manual_columns),
        instance: Some(model.instance().clone()),
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
    next.repair_columns(&model);
    next.reindex();
    let same = world
        .get::<WidgetryTableViewState>(root)
        .is_some_and(|old| {
            old.matches_model(source, model.instance())
                && old.columns == next.columns
                && old.manual_columns == next.manual_columns
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
        old.matches_model(source, model.instance())
            && old.tick == tick
            && old.structure_version == model.structure_version()
    }) {
        return Ok(());
    }
    let mut next = old
        .filter(|old| old.matches_model(source, model.instance()))
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
    next.repair_columns(&model);
    next.instance = Some(model.instance().clone());
    next.source = source;
    next.tick = tick;
    next.structure_version = model.structure_version();
    next.reindex();
    world.entity_mut(root).insert(next);
    Ok(())
}

pub(crate) fn move_column<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    column: WidgetryTableColumnId,
    index: usize,
) -> Result<bool, BevyError> {
    refresh::<T>(world, root)?;
    let mut next = world
        .get::<WidgetryTableViewState>(root)
        .cloned()
        .ok_or_else(|| BevyError::error("Table column move requires View state"))?;
    let old = next
        .column_display_index(column)
        .ok_or_else(|| BevyError::error("Table column move requires a live ColumnId"))?;
    if index >= next.columns.len() {
        return Err(BevyError::error("Table column move index out of bounds"));
    }
    if old == index {
        return Ok(false);
    }
    next.columns.remove(old);
    next.columns.insert(index, column);
    next.manual_columns = true;
    next.reindex();
    world.entity_mut(root).insert(next);
    world.trigger(crate::WidgetryTableEvent {
        entity: root,
        kind: crate::WidgetryTableEventKind::ColumnOrderChanged,
    });
    Ok(true)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableHeaderValue};

    #[test]
    fn manual_column_order_retains_identity_and_appends_new_schema_columns() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        let world = app.world_mut();
        let mut model = WidgetryTableModel::<u32>::default();
        let mut ids = Vec::new();
        for c in 0..3 {
            ids.push(
                model
                    .push_column(WidgetryTableColumn::new(
                        WidgetryTableHeaderValue::new(c),
                        c,
                        |r: &u32, c: &u32| WidgetryTableCellValue::new(r + c),
                    ))
                    .unwrap(),
            );
        }
        model.push_row(1).unwrap();
        let source = world.spawn(model).id();
        let root = world
            .spawn_scene(bsn! {@WidgetryTable::<u32>{@source:source}})
            .unwrap()
            .id();
        refresh::<u32>(world, root).unwrap();
        assert!(move_column::<u32>(world, root, ids[2], 0).unwrap());
        assert_eq!(
            world
                .get::<WidgetryTableViewState>(root)
                .unwrap()
                .ordered_columns(),
            &[ids[2], ids[0], ids[1]]
        );
        let mut model = world.get_mut::<WidgetryTableModel<u32>>(source).unwrap();
        let new = model
            .insert_column(
                1,
                WidgetryTableColumn::new(
                    WidgetryTableHeaderValue::new(3),
                    3,
                    |r: &u32, c: &u32| WidgetryTableCellValue::new(r + c),
                ),
            )
            .unwrap();
        assert!(model.move_column(0, 3));
        refresh::<u32>(world, root).unwrap();
        assert_eq!(
            world
                .get::<WidgetryTableViewState>(root)
                .unwrap()
                .ordered_columns(),
            &[ids[2], ids[0], ids[1], new]
        );
    }
}
