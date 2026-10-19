use crate::{
    WidgetryTable, WidgetryTableCell, WidgetryTableColumnId, WidgetryTableModel,
    WidgetryTableRowId, WidgetryTableState, WidgetryTableViewState,
};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum WidgetryTableSelection {
    #[default]
    None,
    Row(WidgetryTableRowId),
    Column(WidgetryTableColumnId),
    Cell {
        row: WidgetryTableRowId,
        column: WidgetryTableColumnId,
    },
    Rows {
        anchor: WidgetryTableRowId,
        end: WidgetryTableRowId,
    },
    Columns {
        anchor: WidgetryTableColumnId,
        end: WidgetryTableColumnId,
    },
    Cells {
        anchor: WidgetryTableCell,
        end: WidgetryTableCell,
    },
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct WidgetryTableSelectionRange {
    row_start: usize,
    row_end: usize,
    column_start: usize,
    column_end: usize,
}

impl WidgetryTableSelectionRange {
    pub fn rows(self) -> Range<usize> {
        self.row_start..self.row_end
    }
    pub fn columns(self) -> Range<usize> {
        self.column_start..self.column_end
    }
    pub fn contains(self, row: usize, column: usize) -> bool {
        self.rows().contains(&row) && self.columns().contains(&column)
    }
}

impl<T: Send + Sync + 'static> WidgetryTable<T> {
    pub fn selection_range(
        world: &World,
        root: Entity,
    ) -> Result<Option<WidgetryTableSelectionRange>, BevyError> {
        (|| {
            let source = world
                .get::<Self>(root)
                .ok_or_else(|| BevyError::error("Table range query requires a live matching View"))?
                .source();
            let model = world.get::<WidgetryTableModel<T>>(source).ok_or_else(|| {
                BevyError::error("Table range query requires a live matching Model")
            })?;
            let state = world
                .get::<WidgetryTableState>(root)
                .ok_or_else(|| BevyError::error("Table range query requires selection state"))?;
            let view = world
                .get::<WidgetryTableViewState>(root)
                .ok_or_else(|| BevyError::error("Table range query requires View projection"))?;
            if !crate::interaction::valid(model, view, state.selection()) {
                return Ok(None);
            }
            Ok(state
                .selection()
                .resolve(view, model.column_count(), |column| {
                    model.column_index(column)
                }))
        })()
        .inspect_err(|error| widgetry_error!(?root, %error, "Table 选区查询失败"))
    }
}

impl WidgetryTableSelection {
    pub(crate) fn normalize(self, empty: bool) -> Self {
        match self {
            Self::All if empty => Self::None,
            Self::Rows { anchor, end } if anchor == end => Self::Row(anchor),
            Self::Columns { anchor, end } if anchor == end => Self::Column(anchor),
            Self::Cells { anchor, end } if anchor == end => Self::Cell {
                row: anchor.row,
                column: anchor.column,
            },
            _ => self,
        }
    }

    pub(crate) fn extend(self, target: Self) -> Self {
        match (self, target) {
            (Self::Row(anchor) | Self::Rows { anchor, .. }, Self::Row(end)) => {
                Self::Rows { anchor, end }
            }
            (Self::Column(anchor) | Self::Columns { anchor, .. }, Self::Column(end)) => {
                Self::Columns { anchor, end }
            }
            (
                Self::Cell { row, column },
                Self::Cell {
                    row: end_row,
                    column: end_column,
                },
            ) => Self::Cells {
                anchor: WidgetryTableCell { row, column },
                end: WidgetryTableCell {
                    row: end_row,
                    column: end_column,
                },
            },
            (Self::Cells { anchor, .. }, Self::Cell { row, column }) => Self::Cells {
                anchor,
                end: WidgetryTableCell { row, column },
            },
            _ => target,
        }
        .normalize(false)
    }

    pub(crate) fn resolve(
        self,
        view: &WidgetryTableViewState,
        column_count: usize,
        column_index: impl Fn(WidgetryTableColumnId) -> Option<usize>,
    ) -> Option<WidgetryTableSelectionRange> {
        if view.is_empty() || column_count == 0 {
            return None;
        }
        let row = |id| view.display_index(id);
        let span = |a: usize, b: usize| (a.min(b), a.max(b) + 1);
        let ((row_start, row_end), (column_start, column_end)) = match self {
            Self::None => return None,
            Self::All => ((0, view.len()), (0, column_count)),
            Self::Row(id) => {
                let index = row(id)?;
                ((index, index + 1), (0, column_count))
            }
            Self::Column(id) => {
                let index = column_index(id)?;
                ((0, view.len()), (index, index + 1))
            }
            Self::Cell { row: id, column } => {
                let r = row(id)?;
                let c = column_index(column)?;
                ((r, r + 1), (c, c + 1))
            }
            Self::Rows { anchor, end } => (span(row(anchor)?, row(end)?), (0, column_count)),
            Self::Columns { anchor, end } => (
                (0, view.len()),
                span(column_index(anchor)?, column_index(end)?),
            ),
            Self::Cells { anchor, end } => (
                span(row(anchor.row)?, row(end.row)?),
                span(column_index(anchor.column)?, column_index(end.column)?),
            ),
        };
        Some(WidgetryTableSelectionRange {
            row_start,
            row_end,
            column_start,
            column_end,
        })
    }
}
