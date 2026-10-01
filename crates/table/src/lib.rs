//! 通过两个内部 Axis 和 RowId × ColumnId projection 表达 Table 数据。

mod model;

pub use model::{
    WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableColumnId, WidgetryTableHeaderValue,
    WidgetryTableModel, WidgetryTableRowId,
};
