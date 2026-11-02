use crate::{
    WidgetryTableCell, WidgetryTableColumnId, WidgetryTableColumnWidth, WidgetryTableRowId,
    WidgetryTableSelection,
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::WidgetryFailure;
use std::{error::Error, fmt};

pub type TableFailure = WidgetryFailure<TableError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableRegion {
    Table,
    Body,
    FrozenBody,
    ColumnHeaders,
    FrozenColumnHeaders,
    RowHeaders,
    Corner,
    Cell,
    FrozenCell,
    ColumnHeader,
    FrozenColumnHeader,
    RowHeader,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableRendererRole {
    Cell,
    Header,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableRendererFault {
    MissingRegistry,
    MissingRenderer,
    TypeMismatch { expected: std::any::TypeId },
    GenerationExhausted { generation: u64 },
    MissingRegistration,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableRuntimeFault {
    Missing {
        entity: Entity,
        component: &'static str,
    },
    Ownership {
        component: &'static str,
        entity: Entity,
        expected: Entity,
        actual: Option<Entity>,
    },
    Renderer {
        role: TableRendererRole,
        value_type: std::any::TypeId,
        reason: TableRendererFault,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableAxis {
    Row,
    Column,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableIdentity {
    Row(WidgetryTableRowId),
    Column(WidgetryTableColumnId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableLayoutField {
    RowHeight,
    ColumnHeaderHeight,
    RowHeaderWidth,
    MinColumnWidth,
    DefaultColumnWidth,
    ColumnWidth,
}
#[derive(Clone, Debug)]
pub enum TableLayoutFault {
    Dimension {
        field: TableLayoutField,
        column: Option<WidgetryTableColumnId>,
        value: f32,
        width: Option<WidgetryTableColumnWidth>,
    },
    Geometry {
        column: Option<WidgetryTableColumnId>,
        rows: usize,
        columns: usize,
        viewport: f32,
        fixed: f32,
        weight: f32,
        width: f32,
        height: f32,
        row_height: f32,
        min_column_width: f32,
    },
}
impl fmt::Display for TableLayoutFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Dimension { .. } => "Table 布局尺寸或宽度权重必须有限且为正",
            Self::Geometry { .. } => "Table 总布局尺寸溢出",
        })
    }
}
impl Error for TableLayoutFault {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableViewRejection {
    UnknownRow {
        row: WidgetryTableRowId,
        index: usize,
    },
    DuplicateRow {
        row: WidgetryTableRowId,
        index: usize,
    },
    UnknownSortColumn {
        column: WidgetryTableColumnId,
        direction: crate::WidgetryTableSortDirection,
    },
    UnknownFilterColumn {
        column: WidgetryTableColumnId,
    },
    UnknownColumn {
        column: WidgetryTableColumnId,
    },
    ColumnIndex {
        column: WidgetryTableColumnId,
        index: usize,
        len: usize,
    },
    Selection {
        selection: WidgetryTableSelection,
    },
    FocusedCell {
        cell: Option<WidgetryTableCell>,
    },
}
#[derive(Debug)]
pub enum TableError {
    Runtime {
        fault: TableRuntimeFault,
    },
    InvalidColors {
        view: Entity,
    },
    Core {
        source: Box<bevy_widgetry_core::diagnostics::CoreError>,
    },
    InsertionIndex {
        axis: TableAxis,
        index: usize,
        len: usize,
        next_id: u64,
        structure_version: u64,
    },
    IdExhausted {
        axis: TableAxis,
        index: usize,
        len: usize,
        next_id: u64,
        structure_version: u64,
    },
    RevisionExhausted {
        identity: TableIdentity,
        index: usize,
        len: usize,
        revision: u64,
        next_id: u64,
        structure_version: u64,
    },
    Layout {
        fault: Box<TableLayoutFault>,
    },
    MissingPart {
        view: Entity,
        source: Option<Entity>,
        entity: Entity,
        component: &'static str,
    },
    ViewRequest {
        view: Entity,
        source: Entity,
        rows: usize,
        reason: TableViewRejection,
    },
    ColumnWidth {
        view: Entity,
        source: Option<Entity>,
        column: WidgetryTableColumnId,
        width: f32,
    },
}
impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Runtime { .. } => "Table 运行结构或 renderer 失效",
            Self::InvalidColors { .. } => "Table 配色目标失效或属于托管部件",
            Self::Core { .. } => "Table 配色校验失败",
            Self::InsertionIndex { .. } => "Table Model 插入 index 越界",
            Self::IdExhausted { .. } => "Table Model ID 已耗尽",
            Self::RevisionExhausted { .. } => "Table Model revision 已耗尽",
            Self::Layout { .. } => "Table 布局请求失败",
            Self::MissingPart { .. } => "Table 请求所需部件失效",
            Self::ViewRequest { .. } => "Table View 请求包含失效或重复身份",
            Self::ColumnWidth { .. } => "Table 列宽必须有限且为正",
        })
    }
}
impl Error for TableError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source.as_ref()),
            Self::Layout { fault } => Some(fault.as_ref()),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: TableError, operation: &'static str) -> TableFailure {
    let frame = bevy_widgetry_diagnostics::ErrorFrame::new(operation)
        .field("failure", format_args!("{kind:?}"));
    TableFailure::new(kind, frame)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LayoutKey {
    Dimension {
        field: TableLayoutField,
        column: Option<WidgetryTableColumnId>,
        value: u32,
        flexible: bool,
    },
    Geometry {
        column: Option<WidgetryTableColumnId>,
        rows: usize,
        columns: usize,
        values: [u32; 7],
    },
}
impl TableLayoutFault {
    pub(crate) fn key(&self) -> LayoutKey {
        match self {
            Self::Dimension {
                field,
                column,
                value,
                width,
            } => LayoutKey::Dimension {
                field: *field,
                column: *column,
                value: value.to_bits(),
                flexible: matches!(width, Some(WidgetryTableColumnWidth::Flexible(_))),
            },
            Self::Geometry {
                column,
                rows,
                columns,
                viewport,
                fixed,
                weight,
                width,
                height,
                row_height,
                min_column_width,
            } => LayoutKey::Geometry {
                column: *column,
                rows: *rows,
                columns: *columns,
                values: [
                    viewport.to_bits(),
                    fixed.to_bits(),
                    weight.to_bits(),
                    width.to_bits(),
                    height.to_bits(),
                    row_height.to_bits(),
                    min_column_width.to_bits(),
                ],
            },
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct LayoutDiagnostics(
    bevy_widgetry_diagnostics::bevy::FailureState<(Entity, LayoutKey)>,
);
#[track_caller]
pub(crate) fn layout_failure(
    fault: TableLayoutFault,
    view: Entity,
    source: Option<Entity>,
    operation: &'static str,
) -> TableFailure {
    let frame = bevy_widgetry_diagnostics::ErrorFrame::new(operation)
        .field("view", view)
        .field("source", format_args!("{source:?}"))
        .field("failure", format_args!("{fault:?}"));
    TableFailure::new(
        TableError::Layout {
            fault: Box::new(fault),
        },
        frame,
    )
}
pub(crate) fn observe_layout<V>(
    world: &mut World,
    view: Entity,
    source: Entity,
    result: Result<V, TableLayoutFault>,
) -> Result<V, BevyError> {
    let Some(mut state) = world.get_mut::<LayoutDiagnostics>(view) else {
        return result.map_err(|fault| {
            layout_failure(fault, view, Some(source), "table.layout").into_bevy_error()
        });
    };
    match result {
        Ok(value) => {
            state.0.recover(|| {
                bevy_widgetry_diagnostics::ErrorFrame::new("table.layout")
                    .field("view", view)
                    .field("source", source)
            });
            Ok(value)
        }
        Err(fault) => Err(state
            .0
            .failure((source, fault.key()), || {
                std::sync::Arc::new(layout_failure(fault, view, Some(source), "table.layout"))
            })
            .into_bevy_error()),
    }
}

#[track_caller]
pub(crate) fn missing<T: Send + Sync + 'static>(
    world: &World,
    view: Entity,
    entity: Entity,
    component: &'static str,
) -> TableFailure {
    failure(
        TableError::MissingPart {
            view,
            source: world
                .get::<crate::WidgetryTable<T>>(view)
                .map(crate::WidgetryTable::source),
            entity,
            component,
        },
        "table.request",
    )
}
#[track_caller]
pub(crate) fn request(
    view: Entity,
    source: Entity,
    rows: usize,
    reason: TableViewRejection,
) -> TableFailure {
    failure(
        TableError::ViewRequest {
            view,
            source,
            rows,
            reason,
        },
        "table.view_request",
    )
}

#[track_caller]
pub(crate) fn required<V>(
    value: Option<V>,
    view: Entity,
    source: Option<Entity>,
    entity: Entity,
    component: &'static str,
) -> Result<V, TableFailure> {
    value.ok_or_else(|| {
        failure(
            TableError::MissingPart {
                view,
                source,
                entity,
                component,
            },
            "table.request",
        )
    })
}
