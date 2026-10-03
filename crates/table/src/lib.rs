//! 提供异构二维 Table，支持 stable Row/Column identity、virtualization、selection、navigation 和 Column resize。

mod interaction;
mod layout;
mod model;
mod projection;
mod registration;
mod resize;
mod style;
mod view;
mod viewport;

pub use interaction::{
    WidgetryTableEvent, WidgetryTableEventKind, WidgetryTableSelection, WidgetryTableState,
};
pub use layout::{WidgetryTableColumnWidth, WidgetryTableLayout};
pub use model::{
    WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableColumnId, WidgetryTableHeaderValue,
    WidgetryTableModel, WidgetryTableRowId,
};
pub use registration::{
    WidgetryTableAppExt, WidgetryTableCellRenderer, WidgetryTableCellRendererRegistry,
    WidgetryTableHeaderRenderer, WidgetryTableHeaderRendererRegistry, WidgetryTablePlugin,
};
pub use style::{WidgetryTableRegionStyle, WidgetryTableStyle};
pub use view::{
    WidgetryTable, WidgetryTableBody, WidgetryTableCell, WidgetryTableColumnHeader,
    WidgetryTableColumnHeaders, WidgetryTableCorner, WidgetryTableProps, WidgetryTableRowHeader,
    WidgetryTableRowHeaders,
};
