use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct WidgetryTableRowId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct WidgetryTableColumnId(u64);

#[derive(Clone)]
pub struct WidgetryTableCellValue(Arc<dyn Any + Send + Sync>);

#[derive(Clone)]
pub struct WidgetryTableHeaderValue(Arc<dyn Any + Send + Sync>);

pub struct WidgetryTableColumn<T> {
    header: WidgetryTableHeaderValue,
    project: Arc<dyn Fn(&T) -> WidgetryTableCellValue + Send + Sync>,
}

struct RowEntry<T> {
    id: WidgetryTableRowId,
    revision: u64,
    value: T,
}

struct ColumnEntry<T> {
    id: WidgetryTableColumnId,
    revision: u64,
    value: WidgetryTableColumn<T>,
}

#[derive(Component)]
pub struct WidgetryTableModel<T: Send + Sync + 'static> {
    rows: Vec<RowEntry<T>>,
    columns: Vec<ColumnEntry<T>>,
    row_indices: HashMap<WidgetryTableRowId, usize>,
    column_indices: HashMap<WidgetryTableColumnId, usize>,
    next_row: u64,
    next_column: u64,
}

impl<T: Send + Sync + 'static> Default for WidgetryTableModel<T> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            columns: Vec::new(),
            row_indices: default(),
            column_indices: default(),
            next_row: 0,
            next_column: 0,
        }
    }
}

impl<T: Send + Sync + 'static> WidgetryTableModel<T> {
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn push_row(&mut self, value: T) -> Result<WidgetryTableRowId, BevyError> {
        self.insert_row(self.rows.len(), value)
    }

    pub fn insert_row(&mut self, index: usize, value: T) -> Result<WidgetryTableRowId, BevyError> {
        validate_insert(index, self.rows.len(), "Row")?;
        let id = WidgetryTableRowId(allocate(&mut self.next_row, "Row ID")?);
        self.rows.insert(
            index,
            RowEntry {
                id,
                revision: 0,
                value,
            },
        );
        self.index_rows(index, self.rows.len());
        Ok(id)
    }

    pub fn row_id(&self, index: usize) -> Option<WidgetryTableRowId> {
        self.rows.get(index).map(|entry| entry.id)
    }

    pub fn row_index(&self, id: WidgetryTableRowId) -> Option<usize> {
        self.row_indices.get(&id).copied()
    }

    pub fn row(&self, index: usize) -> Option<&T> {
        self.rows.get(index).map(|entry| &entry.value)
    }

    pub fn row_mut(&mut self, index: usize) -> Result<Option<&mut T>, BevyError> {
        let Some(entry) = self.rows.get_mut(index) else {
            return Ok(None);
        };
        advance_revision(&mut entry.revision, "Row revision")?;
        Ok(Some(&mut entry.value))
    }

    pub fn row_revision(&self, index: usize) -> Option<u64> {
        self.rows.get(index).map(|entry| entry.revision)
    }

    pub fn move_row(&mut self, from: usize, to: usize) -> bool {
        let moved = move_entry(&mut self.rows, from, to);
        if moved && from != to {
            self.index_rows(from.min(to), from.max(to) + 1);
        }
        moved
    }

    pub fn remove_row(&mut self, index: usize) -> Option<T> {
        if index >= self.rows.len() {
            return None;
        }
        let entry = self.rows.remove(index);
        self.row_indices.remove(&entry.id);
        self.index_rows(index, self.rows.len());
        Some(entry.value)
    }

    pub fn clear_rows(&mut self) {
        self.rows.clear();
        self.row_indices.clear();
    }

    pub fn push_column(
        &mut self,
        column: WidgetryTableColumn<T>,
    ) -> Result<WidgetryTableColumnId, BevyError> {
        self.insert_column(self.columns.len(), column)
    }

    pub fn insert_column(
        &mut self,
        index: usize,
        column: WidgetryTableColumn<T>,
    ) -> Result<WidgetryTableColumnId, BevyError> {
        validate_insert(index, self.columns.len(), "Column")?;
        let id = WidgetryTableColumnId(allocate(&mut self.next_column, "Column ID")?);
        self.columns.insert(
            index,
            ColumnEntry {
                id,
                revision: 0,
                value: column,
            },
        );
        self.index_columns(index, self.columns.len());
        Ok(id)
    }

    pub fn column_id(&self, index: usize) -> Option<WidgetryTableColumnId> {
        self.columns.get(index).map(|entry| entry.id)
    }

    pub fn column_index(&self, id: WidgetryTableColumnId) -> Option<usize> {
        self.column_indices.get(&id).copied()
    }

    pub fn column(&self, index: usize) -> Option<&WidgetryTableColumn<T>> {
        self.columns.get(index).map(|entry| &entry.value)
    }

    pub fn column_revision(&self, index: usize) -> Option<u64> {
        self.columns.get(index).map(|entry| entry.revision)
    }

    pub fn set_column(
        &mut self,
        index: usize,
        column: WidgetryTableColumn<T>,
    ) -> Result<bool, BevyError> {
        let Some(entry) = self.columns.get_mut(index) else {
            return Ok(false);
        };
        advance_revision(&mut entry.revision, "Column revision")?;
        entry.value = column;
        Ok(true)
    }

    pub fn set_header(
        &mut self,
        index: usize,
        header: WidgetryTableHeaderValue,
    ) -> Result<bool, BevyError> {
        let Some(entry) = self.columns.get_mut(index) else {
            return Ok(false);
        };
        advance_revision(&mut entry.revision, "Column revision")?;
        entry.value.header = header;
        Ok(true)
    }

    pub fn move_column(&mut self, from: usize, to: usize) -> bool {
        let moved = move_entry(&mut self.columns, from, to);
        if moved && from != to {
            self.index_columns(from.min(to), from.max(to) + 1);
        }
        moved
    }

    pub fn remove_column(&mut self, index: usize) -> Option<WidgetryTableColumn<T>> {
        if index >= self.columns.len() {
            return None;
        }
        let entry = self.columns.remove(index);
        self.column_indices.remove(&entry.id);
        self.index_columns(index, self.columns.len());
        Some(entry.value)
    }

    pub fn clear_columns(&mut self) {
        self.columns.clear();
        self.column_indices.clear();
    }

    pub fn cell(
        &self,
        row: WidgetryTableRowId,
        column: WidgetryTableColumnId,
    ) -> Option<WidgetryTableCellValue> {
        self.cell_at(self.row_index(row)?, self.column_index(column)?)
    }

    pub(crate) fn cell_at(&self, row: usize, column: usize) -> Option<WidgetryTableCellValue> {
        let row = self.rows.get(row)?;
        let column = self.columns.get(column)?;
        Some((column.value.project)(&row.value))
    }

    pub fn header(&self, column: WidgetryTableColumnId) -> Option<&WidgetryTableHeaderValue> {
        self.column(self.column_index(column)?)
            .map(WidgetryTableColumn::header)
    }

    fn index_rows(&mut self, start: usize, end: usize) {
        for index in start..end {
            self.row_indices.insert(self.rows[index].id, index);
        }
    }

    fn index_columns(&mut self, start: usize, end: usize) {
        for index in start..end {
            self.column_indices.insert(self.columns[index].id, index);
        }
    }
}

fn validate_insert(index: usize, len: usize, axis: &str) -> Result<(), BevyError> {
    if index <= len {
        return Ok(());
    }
    widgetry_error!(index, len, axis, "Table Model 插入 index 越界");
    Err(BevyError::error(format!(
        "Table {axis} insertion index out of bounds"
    )))
}

fn allocate(counter: &mut u64, kind: &str) -> Result<u64, BevyError> {
    let id = *counter;
    let Some(next) = counter.checked_add(1) else {
        widgetry_error!(kind, "Table Model counter 已耗尽");
        return Err(BevyError::error(format!("Table {kind} exhausted")));
    };
    *counter = next;
    Ok(id)
}

fn advance_revision(revision: &mut u64, kind: &str) -> Result<(), BevyError> {
    allocate(revision, kind).map(|_| ())
}

fn move_entry<T>(entries: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from >= entries.len() || to >= entries.len() {
        return false;
    }
    if from != to {
        let entry = entries.remove(from);
        entries.insert(to, entry);
    }
    true
}

impl WidgetryTableCellValue {
    pub(crate) fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self.0.as_ref()
    }

    pub fn new<V: Send + Sync + 'static>(value: V) -> Self {
        Self(Arc::new(value))
    }

    pub fn downcast_ref<V: 'static>(&self) -> Option<&V> {
        self.0.downcast_ref()
    }

    pub fn type_id(&self) -> TypeId {
        self.0.as_ref().type_id()
    }
}

impl WidgetryTableHeaderValue {
    pub(crate) fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self.0.as_ref()
    }

    pub fn new<V: Send + Sync + 'static>(value: V) -> Self {
        Self(Arc::new(value))
    }

    pub fn downcast_ref<V: 'static>(&self) -> Option<&V> {
        self.0.downcast_ref()
    }

    pub fn type_id(&self) -> TypeId {
        self.0.as_ref().type_id()
    }
}

impl<T> WidgetryTableColumn<T> {
    pub fn new<S, F>(header: WidgetryTableHeaderValue, schema: S, projection: F) -> Self
    where
        S: Send + Sync + 'static,
        F: Fn(&T, &S) -> WidgetryTableCellValue + Send + Sync + 'static,
    {
        Self {
            header,
            project: Arc::new(move |row| projection(row, &schema)),
        }
    }

    pub fn header(&self) -> &WidgetryTableHeaderValue {
        &self.header
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::ecs::error::Severity;
    use bevy::log::tracing::Level;
    use bevy_widgetry_test_utils::LogCapture;

    #[derive(Clone, Copy)]
    enum Field {
        Name,
        Age,
    }

    fn column(field: Field) -> WidgetryTableColumn<(String, u32)> {
        WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new("header"),
            field,
            |row: &(String, u32), field| match field {
                Field::Name => WidgetryTableCellValue::new(row.0.clone()),
                Field::Age => WidgetryTableCellValue::new(row.1),
            },
        )
    }

    #[test]
    fn row_identity_is_unique_and_axes_are_independent() {
        let mut model = WidgetryTableModel::default();
        let a = model.push_row(10).unwrap();
        let b = model.push_row(20).unwrap();
        assert_ne!(a, b);
        assert_eq!(model.row_count(), 2);
        assert_eq!(model.column_count(), 0);
    }

    #[test]
    fn axis_lookups_follow_mutations_and_reject_removed_ids() {
        let mut model = WidgetryTableModel::<u32>::default();
        let check = |model: &WidgetryTableModel<u32>| {
            assert_eq!(model.row_indices.len(), model.row_count());
            assert_eq!(model.column_indices.len(), model.column_count());
            for index in 0..model.row_count() {
                assert_eq!(model.row_index(model.row_id(index).unwrap()), Some(index));
            }
            for index in 0..model.column_count() {
                assert_eq!(
                    model.column_index(model.column_id(index).unwrap()),
                    Some(index)
                );
            }
        };
        for index in [0, 0, 1, 3, 2] {
            model.insert_row(index, index as u32).unwrap();
            model
                .insert_column(
                    index,
                    WidgetryTableColumn::new(
                        WidgetryTableHeaderValue::new(()),
                        (),
                        |row: &u32, _| WidgetryTableCellValue::new(*row),
                    ),
                )
                .unwrap();
            check(&model);
        }
        for (from, to) in [(0, 4), (3, 0), (2, 2), (5, 0), (0, 5)] {
            model.move_row(from, to);
            model.move_column(from, to);
            check(&model);
        }
        let removed_row = model.row_id(2).unwrap();
        let removed_column = model.column_id(2).unwrap();
        model.remove_row(2);
        model.remove_column(2);
        check(&model);
        assert_eq!(model.row_index(removed_row), None);
        assert_eq!(model.column_index(removed_column), None);
        model.clear_rows();
        model.clear_columns();
        check(&model);
        assert_ne!(model.push_row(0).unwrap(), removed_row);
        assert_ne!(
            model
                .push_column(WidgetryTableColumn::new(
                    WidgetryTableHeaderValue::new(()),
                    (),
                    |row: &u32, _| WidgetryTableCellValue::new(*row),
                ))
                .unwrap(),
            removed_column
        );
        check(&model);
    }

    #[test]
    fn pairs_project_current_values_after_independent_axis_changes() {
        let mut model = WidgetryTableModel::default();
        let a = model.push_row(("Alice".to_owned(), 21)).unwrap();
        let b = model.insert_row(0, ("Bob".to_owned(), 30)).unwrap();
        let name = model.push_column(column(Field::Name)).unwrap();
        let age = model.insert_column(0, column(Field::Age)).unwrap();
        assert_eq!(
            model
                .cell(a, name)
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "Alice"
        );
        assert_eq!(model.cell(b, age).unwrap().downcast_ref::<u32>(), Some(&30));
        assert!(model.move_row(0, 1));
        assert!(model.move_column(0, 1));
        assert_eq!(model.row_id(0), Some(a));
        assert_eq!(model.column_id(1), Some(age));
        model.row_mut(0).unwrap().unwrap().1 = 22;
        assert_eq!(model.row_revision(0), Some(1));
        assert_eq!(model.row_revision(1), Some(0));
        assert_eq!(model.column_revision(1), Some(0));
        assert_eq!(model.cell(a, age).unwrap().downcast_ref::<u32>(), Some(&22));
        model.row_mut(0).unwrap().unwrap();
        assert_eq!(model.row_revision(0), Some(2));
        model
            .set_header(1, WidgetryTableHeaderValue::new(12u32))
            .unwrap();
        assert_eq!(model.header(age).unwrap().downcast_ref::<u32>(), Some(&12));
        assert_eq!(model.column_revision(1), Some(1));
        assert_eq!(model.row_revision(0), Some(2));
        model.set_column(1, column(Field::Name)).unwrap();
        assert_eq!(model.column_id(1), Some(age));
        assert_eq!(model.column_revision(1), Some(2));
        assert_eq!(
            model
                .cell(a, age)
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "Alice"
        );
        assert!(model.cell(a, age).unwrap().downcast_ref::<u32>().is_none());
    }

    #[test]
    fn stale_pairs_never_alias_reinserted_values_and_empty_axes_stay_independent() {
        let mut model = WidgetryTableModel::default();
        let old_row = model.push_row(("A".into(), 1)).unwrap();
        let old_col = model.push_column(column(Field::Age)).unwrap();
        assert_eq!(model.remove_row(0), Some(("A".into(), 1)));
        let row = model.push_row(("A".into(), 1)).unwrap();
        assert_ne!(row, old_row);
        assert!(model.cell(old_row, old_col).is_none());
        assert!(model.remove_column(0).is_some());
        let col = model.push_column(column(Field::Age)).unwrap();
        assert_ne!(col, old_col);
        assert!(model.cell(row, old_col).is_none());
        model.clear_rows();
        assert!(model.header(col).is_some());
        assert!(model.cell(row, col).is_none());
        let final_row = model.push_row(("B".into(), 2)).unwrap();
        assert_ne!(final_row, row);
        model.clear_columns();
        assert_eq!(model.row(0), Some(&("B".into(), 2)));
        let final_col = model.push_column(column(Field::Name)).unwrap();
        assert_ne!(final_col, col);
        assert!(model.cell(final_row, col).is_none());
        assert_eq!(
            model
                .cell(final_row, final_col)
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "B"
        );
    }

    #[test]
    fn invalid_operations_preserve_order_and_counters() {
        let mut model = WidgetryTableModel::<(String, u32)>::default();
        assert!(!model.move_row(0, 0));
        assert!(!model.move_column(0, 0));
        assert!(model.row_mut(0).unwrap().is_none());
        assert!(model.remove_row(0).is_none());
        assert!(model.remove_column(0).is_none());
        assert!(!model.set_column(0, column(Field::Age)).unwrap());
        assert!(
            !model
                .set_header(0, WidgetryTableHeaderValue::new(1u32))
                .unwrap()
        );
        let capture = LogCapture::default();
        for error in capture.run(|| {
            vec![
                model.insert_row(1, ("A".into(), 1)).unwrap_err(),
                model.insert_column(1, column(Field::Age)).unwrap_err(),
            ]
        }) {
            assert_eq!(error.severity(), Severity::Error);
            assert!(error.to_string().contains("index out of bounds"));
        }
        assert_eq!(capture.records().len(), 2);
        assert!(
            capture
                .records()
                .iter()
                .all(|record| record.level == Level::ERROR)
        );
        assert_eq!((model.next_row, model.next_column), (0, 0));
        let id = model.push_row(("A".into(), 1)).unwrap();
        model.push_column(column(Field::Age)).unwrap();
        assert!(model.move_row(0, 0));
        assert!(model.move_column(0, 0));
        assert!(!model.move_row(0, 1));
        assert!(!model.move_column(1, 0));
        assert_eq!(model.row_id(0), Some(id));
        assert_eq!(model.row_id(1), None);
        assert_eq!(model.column_id(1), None);
        assert_eq!(model.row_revision(1), None);
        assert_eq!(model.column_revision(1), None);
    }

    #[test]
    fn exhausted_counters_return_logged_errors_without_mutation() {
        let mut model = WidgetryTableModel::default();
        let row = model.push_row(("A".into(), 1)).unwrap();
        let col = model.push_column(column(Field::Age)).unwrap();
        model.next_row = u64::MAX;
        model.next_column = u64::MAX;
        model.rows[0].revision = u64::MAX;
        model.columns[0].revision = u64::MAX;
        let capture = LogCapture::default();
        let errors = capture.run(|| {
            vec![
                model.push_row(("B".into(), 2)).unwrap_err(),
                model.push_column(column(Field::Name)).unwrap_err(),
                model.row_mut(0).unwrap_err(),
                model.set_column(0, column(Field::Name)).unwrap_err(),
                model
                    .set_header(0, WidgetryTableHeaderValue::new(7u32))
                    .unwrap_err(),
            ]
        });
        assert_eq!(capture.records().len(), 5);
        for error in errors {
            assert_eq!(error.severity(), Severity::Error);
            assert!(error.to_string().contains("exhausted"));
        }
        assert!(
            capture
                .records()
                .iter()
                .all(|record| record.level == Level::ERROR)
        );
        assert_eq!((model.row_count(), model.column_count()), (1, 1));
        assert_eq!(model.row_id(0), Some(row));
        assert_eq!(model.column_id(0), Some(col));
        assert_eq!(
            model.cell(row, col).unwrap().downcast_ref::<u32>(),
            Some(&1)
        );
        assert_eq!(
            model.header(col).unwrap().downcast_ref::<&str>(),
            Some(&"header")
        );
        assert_eq!(model.row_revision(0), Some(u64::MAX));
        assert_eq!(model.column_revision(0), Some(u64::MAX));
        assert_eq!((model.next_row, model.next_column), (u64::MAX, u64::MAX));
    }
}
