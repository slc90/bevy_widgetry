//! 提供用于展示行数据与多种类型 Cell 内容的二维 Table。
//! 适用于需要 Row、Column 或 Cell selection，以及可配置列宽的表格界面。
//!
//! Model 保存 generic Row 数据，每个 Column 通过 projection 从 Row 生成对应的 Cell 值。
//! Cell 和 Column Header 支持异构数据，并分别通过按值类型注册的 renderer 生成 BSN 内容。
//! Model 支持 Row 与 Column 的插入、移除、移动和内容更新，并提供 stable ID 与 revision。
//! Table 展示 body、Column Headers、Row Headers 与 corner，Header 随对应方向的内容滚动保持对齐。
//! 两个方向均按可见范围生成 Cell 与 Header，支持滚动浏览超出 viewport 的数据。
//! layout 可配置 row height、Header 尺寸、默认列宽、逐列宽度和最小列宽。
//! 列宽支持 Fixed 与按权重分配可用空间的 Flexible 两种配置。
//! pointer 可选择 Row、Column 或 Cell，方向键可移动 focused_cell 并使目标进入可见范围。
//! 可通过程序化 API 设置或清空 selection，并分别调整 focused_cell 与列宽。
//! Column Header 的 resize handle 支持 drag 调宽，并发出 Start、Resized、End 或 Cancel 通知。
//! 提供按区域配置的 style，以及随 theme、selection、focus 与 disabled 更新的外观。
//!
//! 使用前注册 Row 类型和实际使用的 Cell、Header 值类型 renderer，并提供匹配的 Model source。
//! Row 与 Column ID 在同一 Model 内保持稳定，移动后仍指向原数据，删除后不复用。
//! selection 与 focused_cell 分别表达已选择的数据和 keyboard navigation 的当前位置。
//! selection 变化先提交真实 state 再通知，同值保持静默，Model 自动 repair 保持静默。
//! InteractionDisabled 阻止用户 selection、navigation 和 resize，程序化 state 更新仍由公开 API 提供。
//! resize 过程中目标失效或操作被取消时结束当前 gesture，并以 Cancel 表达结果。

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
