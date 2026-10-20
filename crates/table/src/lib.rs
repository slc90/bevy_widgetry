#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod clipboard;
mod column_reorder;
pub use clipboard::{
    WidgetryTableCopyAppExt, WidgetryTableCopyFormatter, WidgetryTableCopyFormatterRegistry,
};
pub use column_reorder::WidgetryTableColumnReordering;
mod editing;
mod header;
pub use editing::{
    WidgetryTableCellEditor, WidgetryTableEditRequestId, WidgetryTableEditRequested,
    WidgetryTableEditResult, WidgetryTableEditState, WidgetryTableEditing,
    WidgetryTableEditorAppExt, WidgetryTableEditorMode, WidgetryTableEditorPopup,
    WidgetryTableEditorRegistry, WidgetryTableSubmitEdit,
};
mod interaction;
mod selection;
mod selection_drag;
pub use header::{
    WidgetryTableHeaderAction, WidgetryTableHeaderActionKind, WidgetryTableHeaderActions,
};
pub use selection::{WidgetryTableSelection, WidgetryTableSelectionRange};
mod layout;
mod model;
mod projection;
mod registration;
mod resize;
mod style;
mod view;
mod view_state;
mod viewport;

pub use view_state::{
    WidgetryTableSort, WidgetryTableSortDirection, WidgetryTableViewState, WidgetryTableViewUpdate,
};

pub use interaction::{WidgetryTableEvent, WidgetryTableEventKind, WidgetryTableState};
pub use layout::{WidgetryTableColumnWidth, WidgetryTableLayout};
pub use model::{
    WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableColumnId, WidgetryTableHeaderValue,
    WidgetryTableModel, WidgetryTableRowId,
};
pub use registration::{
    WidgetryTableAppExt, WidgetryTableCellRenderer, WidgetryTableCellRendererRegistry,
    WidgetryTableHeaderRenderer, WidgetryTableHeaderRendererRegistry, WidgetryTablePlugin,
    WidgetryTableSystems,
};
pub use style::{WidgetryTableRegionStyle, WidgetryTableStyle};
pub use view::{
    WidgetryTable, WidgetryTableBody, WidgetryTableCell, WidgetryTableColumnHeader,
    WidgetryTableColumnHeaders, WidgetryTableCorner, WidgetryTableFrozenBody,
    WidgetryTableFrozenColumnHeaders, WidgetryTableProps, WidgetryTableRowHeader,
    WidgetryTableRowHeaders,
};

mod colors;
pub use colors::*;
