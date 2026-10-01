use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// 所属 Model 内稳定、删除后永不复用；完整 identity 包含 source Entity。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct WidgetryTableRowId(u64);

/// 与 Row ID 独立分配；只能结合所属 Model/source 解释。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct WidgetryTableColumnId(u64);

/// 二维 projection 产生的 owned 异构值，不持有独立 Cell identity。
#[derive(Clone)]
pub struct WidgetryTableCellValue(Arc<dyn Any + Send + Sync>);

/// Header 的 owned 异构语义值，与 Cell renderer registry 隔离。
#[derive(Clone)]
pub struct WidgetryTableHeaderValue(Arc<dyn Any + Send + Sync>);

/// Column 数据与 schema；不承载 width、selection 或 layout。
pub struct WidgetryTableColumn<T> {
    /// 不限制为 String 的 Header 内容。
    header: WidgetryTableHeaderValue,
    /// closure 捕获只读 schema，每次查询只从当前 Row value 投影。
    project: Arc<dyn Fn(&T) -> WidgetryTableCellValue + Send + Sync>,
}

/// 将 Row 的 identity、revision 与唯一业务 value 绑定，顺序改变不改变 metadata。
struct RowEntry<T> {
    /// model-local logical identity。
    id: WidgetryTableRowId,
    /// mutable access 时推进，初始为零。
    revision: u64,
    /// 外部只能通过 Model API 取得 mutable access。
    value: T,
}

/// Header/schema replacement 使用独立 revision，不改变 Column identity。
struct ColumnEntry<T> {
    /// 与当前 index 分离的 model-local identity。
    id: WidgetryTableColumnId,
    /// Header 与 schema 共用的内容版本。
    revision: u64,
    /// 唯一 Column schema 与 Header 定义。
    value: WidgetryTableColumn<T>,
}

/// 多个 View 可共享的独立 ECS 数据 source；外部不操作内部 Axis。
/// ID 仅在此 Component 生命周期内有效，不支持整块替换后继续使用旧 identity。
#[derive(Component)]
pub struct WidgetryTableModel<T: Send + Sync + 'static> {
    /// 有序 Row Axis，private Vec 防止绕过 revision/identity contract。
    rows: Vec<RowEntry<T>>,
    /// 有序 Column Axis，与 Row 独立增删改移。
    columns: Vec<ColumnEntry<T>>,
    /// stable ID 的当前 index；mutation 只修复顺序变化的区间，查询不扫描整个 Axis。
    row_indices: HashMap<WidgetryTableRowId, usize>,
    /// 与 Row 独立维护的 Column ID lookup。
    column_indices: HashMap<WidgetryTableColumnId, usize>,
    /// clear/remove 不回退 counter。
    next_row: u64,
    /// Column 不共享 Row 的 counter。
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
    /// 当前 Row 数量。
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// 当前 Column 数量。
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// 末尾追加 Row，耗尽时返回 Error 且不改变 Model。
    pub fn push_row(&mut self, value: T) -> Result<WidgetryTableRowId, BevyError> {
        self.insert_row(self.rows.len(), value)
    }

    /// 在 index 前插入；index == row_count 允许追加，越界拒绝且不消耗 ID。
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

    /// 当前 Row index 的稳定 ID；越界返回 None。
    pub fn row_id(&self, index: usize) -> Option<WidgetryTableRowId> {
        self.rows.get(index).map(|entry| entry.id)
    }

    /// 根据稳定 ID 查找当前 Row index；stale ID 返回 None。
    pub fn row_index(&self, id: WidgetryTableRowId) -> Option<usize> {
        self.row_indices.get(&id).copied()
    }

    /// 当前 Row value 的只读访问；越界返回 None。
    pub fn row(&self, index: usize) -> Option<&T> {
        self.rows.get(index).map(|entry| &entry.value)
    }

    /// 每次成功 mutable access 都推进该 Row revision，即使调用方不修改 value。
    /// 越界为 Ok(None)，revision 耗尽为 Error 且不开放 mutable access。
    pub fn row_mut(&mut self, index: usize) -> Result<Option<&mut T>, BevyError> {
        let Some(entry) = self.rows.get_mut(index) else {
            return Ok(None);
        };
        advance_revision(&mut entry.revision, "Row revision")?;
        Ok(Some(&mut entry.value))
    }

    /// 读取当前 Row 的内容版本；初始为零。
    pub fn row_revision(&self, index: usize) -> Option<u64> {
        self.rows.get(index).map(|entry| entry.revision)
    }

    /// 将 Row 移至最终 index，保留 ID/revision/value；越界返回 false，同位置为有效 no-op。
    pub fn move_row(&mut self, from: usize, to: usize) -> bool {
        let moved = move_entry(&mut self.rows, from, to);
        if moved && from != to {
            self.index_rows(from.min(to), from.max(to) + 1);
        }
        moved
    }

    /// 删除 Row 并永久使其 ID 失效；越界返回 None。
    pub fn remove_row(&mut self, index: usize) -> Option<T> {
        if index >= self.rows.len() {
            return None;
        }
        let entry = self.rows.remove(index);
        self.row_indices.remove(&entry.id);
        self.index_rows(index, self.rows.len());
        Some(entry.value)
    }

    /// 清空 Row Axis，保留 Column 和 ID counter。
    pub fn clear_rows(&mut self) {
        self.rows.clear();
        self.row_indices.clear();
    }

    /// 追加 Column，初始 revision 为零，不改变 Row Axis。
    pub fn push_column(
        &mut self,
        column: WidgetryTableColumn<T>,
    ) -> Result<WidgetryTableColumnId, BevyError> {
        self.insert_column(self.columns.len(), column)
    }

    /// 在 index 前插入 Column；末尾合法，越界或 ID 耗尽不改变 Model。
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

    /// 当前 Column index 的稳定 ID；越界返回 None。
    pub fn column_id(&self, index: usize) -> Option<WidgetryTableColumnId> {
        self.columns.get(index).map(|entry| entry.id)
    }

    /// 根据稳定 ID 查找当前 Column index；stale ID 返回 None。
    pub fn column_index(&self, id: WidgetryTableColumnId) -> Option<usize> {
        self.column_indices.get(&id).copied()
    }

    /// Column 定义的只读访问，不允许绕过版本管理修改 schema。
    pub fn column(&self, index: usize) -> Option<&WidgetryTableColumn<T>> {
        self.columns.get(index).map(|entry| &entry.value)
    }

    /// 读取 Column 内容版本，Row mutable access 不改变它。
    pub fn column_revision(&self, index: usize) -> Option<u64> {
        self.columns.get(index).map(|entry| entry.revision)
    }

    /// 原位置替换 Header/schema，保留 Column ID 并推进该 Column revision。
    /// 越界为 Ok(false)；版本耗尽拒绝修改。
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

    /// 只替换 Header，保留 schema/ID，并推进该 Column revision；越界为 Ok(false)。
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

    /// Column 移至最终 index，不改变其 identity、内容或 revision。
    pub fn move_column(&mut self, from: usize, to: usize) -> bool {
        let moved = move_entry(&mut self.columns, from, to);
        if moved && from != to {
            self.index_columns(from.min(to), from.max(to) + 1);
        }
        moved
    }

    /// 删除 Column 并永久使其 ID 失效；越界返回 None。
    pub fn remove_column(&mut self, index: usize) -> Option<WidgetryTableColumn<T>> {
        if index >= self.columns.len() {
            return None;
        }
        let entry = self.columns.remove(index);
        self.column_indices.remove(&entry.id);
        self.index_columns(index, self.columns.len());
        Some(entry.value)
    }

    /// 清空 Column Axis，不改变 Row 或回退 counter。
    pub fn clear_columns(&mut self) {
        self.columns.clear();
        self.column_indices.clear();
    }

    /// 按 logical ID pair 查询当前 Row × schema 的 owned 异构值。
    /// 任一 ID missing/stale 时返回 None；不同 Model 可能分配同值 ID，调用方负责 source provenance。
    pub fn cell(
        &self,
        row: WidgetryTableRowId,
        column: WidgetryTableColumnId,
    ) -> Option<WidgetryTableCellValue> {
        self.cell_at(self.row_index(row)?, self.column_index(column)?)
    }

    /// View 已持有当前 ordered index，直接访问 Axis，避免可见 pair 再走 ID lookup。
    pub(crate) fn cell_at(&self, row: usize, column: usize) -> Option<WidgetryTableCellValue> {
        let row = self.rows.get(row)?;
        let column = self.columns.get(column)?;
        Some((column.value.project)(&row.value))
    }

    /// 按稳定 Column ID 读取 Header，Row Axis 为空时仍然可用。
    pub fn header(&self, column: WidgetryTableColumnId) -> Option<&WidgetryTableHeaderValue> {
        self.column(self.column_index(column)?)
            .map(WidgetryTableColumn::header)
    }

    /// insert/remove/move 后仅修复受影响的 Row index；末尾追加保持常数开销。
    fn index_rows(&mut self, start: usize, end: usize) {
        for index in start..end {
            self.row_indices.insert(self.rows[index].id, index);
        }
    }

    /// Column 顺序独立变化，不触碰 Row lookup 或内容 revision。
    fn index_columns(&mut self, start: usize, end: usize) {
        for index in start..end {
            self.column_indices.insert(self.columns[index].id, index);
        }
    }
}

/// 在 mutation 之前验证插入边界，失败保留 counter 与集合。
fn validate_insert(index: usize, len: usize, axis: &str) -> Result<(), BevyError> {
    if index <= len {
        return Ok(());
    }
    widgetry_error!(index, len, axis, "Table Model 插入 index 越界");
    Err(BevyError::error(format!(
        "Table {axis} insertion index out of bounds"
    )))
}

/// 单调分配 ID，拒绝溢出，避免 identity 回绕复用。
fn allocate(counter: &mut u64, kind: &str) -> Result<u64, BevyError> {
    let id = *counter;
    let Some(next) = counter.checked_add(1) else {
        widgetry_error!(kind, "Table Model counter 已耗尽");
        return Err(BevyError::error(format!("Table {kind} exhausted")));
    };
    *counter = next;
    Ok(id)
}

/// 内容变更之前推进 revision，耗尽时不允许任何对应业务 mutation。
fn advance_revision(revision: &mut u64, kind: &str) -> Result<(), BevyError> {
    allocate(revision, kind).map(|_| ())
}

/// 两个内部 Axis 共用最终 index 的 move 语义，不暴露 Axis API。
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
    /// 供内部 renderer dispatch 使用，不允许 mutable access。
    pub(crate) fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self.0.as_ref()
    }

    /// 接收任意 owned Send + Sync 业务值，无需 Clone/Reflect/Component。
    pub fn new<V: Send + Sync + 'static>(value: V) -> Self {
        Self(Arc::new(value))
    }

    /// 精确类型的只读访问，类型不匹配返回 None。
    pub fn downcast_ref<V: 'static>(&self) -> Option<&V> {
        self.0.downcast_ref()
    }

    /// renderer 使用的实际业务 type，不是 wrapper 的 type。
    pub fn type_id(&self) -> TypeId {
        self.0.as_ref().type_id()
    }
}

impl WidgetryTableHeaderValue {
    /// Header registry 的 type-erased 只读输入。
    pub(crate) fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self.0.as_ref()
    }

    /// Header 独立于 Cell 内容，可接收自定义 owned 语义值。
    pub fn new<V: Send + Sync + 'static>(value: V) -> Self {
        Self(Arc::new(value))
    }

    /// 精确类型的只读访问，类型不匹配返回 None。
    pub fn downcast_ref<V: 'static>(&self) -> Option<&V> {
        self.0.downcast_ref()
    }

    /// Header renderer 使用的实际业务 type。
    pub fn type_id(&self) -> TypeId {
        self.0.as_ref().type_id()
    }
}

impl<T> WidgetryTableColumn<T> {
    /// 将 owned schema 与 typed projection 一次性封装；查询使用当前 Row value。
    /// projection 返回 owned CellValue，允许每个 ID pair 投影不同 value type。
    /// schema 变更通过 Model::set_column 替换定义，closure 不应依赖未纳入 Row/Column revision 的外部 mutable state。
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

    /// 不转换为 String 的 Header 业务值。
    pub fn header(&self) -> &WidgetryTableHeaderValue {
        &self.header
    }
}

// 测试 module 的断言验证业务 contract，生产代码仍禁止主动 panic。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::ecs::error::Severity;
    use bevy::log::tracing::Level;
    use bevy_widgetry_test_utils::LogCapture;

    /// 业务 schema 选择不同 field，同一个 Row 可投影成不同 Cell type。
    #[derive(Clone, Copy)]
    enum Field {
        Name,
        Age,
    }

    /// 用 typed schema 建立异构 projection，不从 Row hierarchy 生成 Cell。
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

    /// 两次追加必须生成不同 Row identity，Row Axis 不改变 Column Axis。
    #[test]
    fn row_identity_is_unique_and_axes_are_independent() {
        let mut model = WidgetryTableModel::default();
        let a = model.push_row(10).unwrap();
        let b = model.push_row(20).unwrap();
        assert_ne!(a, b);
        assert_eq!(model.row_count(), 2);
        assert_eq!(model.column_count(), 0);
    }

    /// 连续两轴 insert/move/remove/clear 后 lookup 必须精确反映当前顺序，旧 ID 永不复用。
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

    /// ID pair 区分 Row 与 schema；两轴 move 后仍查询原业务数据，mutation 只改变对应 revision。
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

    /// remove/clear 永久作废 ID；空 Row 不隐藏 Header，空 Column 不删除业务 Row。
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

    /// 空态与越界 mutation 保持 Model，插入拒绝记录 ERROR 并返回 Error severity。
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

    /// 两个 ID counter 与两个 revision 的耗尽均拒绝 mutation，原 identity/内容/版本保持不变。
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
