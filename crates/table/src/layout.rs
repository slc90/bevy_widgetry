use crate::WidgetryTableColumnId;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetryTableColumnWidth {
    Fixed(f32),
    Flexible(f32),
}

#[derive(Component, Clone)]
pub struct WidgetryTableLayout {
    pub row_height: f32,
    pub column_header_height: f32,
    pub row_header_width: f32,
    pub default_column_width: WidgetryTableColumnWidth,
    pub min_column_width: f32,
    pub(crate) columns: HashMap<WidgetryTableColumnId, WidgetryTableColumnWidth>,
}

#[derive(Clone)]
pub(crate) struct ColumnGeometry {
    pub(crate) id: WidgetryTableColumnId,
    pub(crate) index: usize,
    pub(crate) left: f32,
    pub(crate) width: f32,
}

#[derive(Component, Clone, Default)]
pub(crate) struct TableGeometry {
    pub(crate) columns: Vec<ColumnGeometry>,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) row_height: f32,
}

impl Default for WidgetryTableLayout {
    fn default() -> Self {
        Self {
            row_height: 28.0,
            column_header_height: 30.0,
            row_header_width: 44.0,
            default_column_width: WidgetryTableColumnWidth::Fixed(120.0),
            min_column_width: 24.0,
            columns: HashMap::new(),
        }
    }
}

impl WidgetryTableLayout {
    pub fn with_column_width(
        mut self,
        column: WidgetryTableColumnId,
        width: WidgetryTableColumnWidth,
    ) -> Self {
        self.columns.insert(column, width);
        self
    }

    pub fn column_widths(&self) -> &HashMap<WidgetryTableColumnId, WidgetryTableColumnWidth> {
        &self.columns
    }

    pub(crate) fn validate(&self) -> Result<(), BevyError> {
        let positive = |value: f32| value.is_finite() && value > 0.0;
        let width_valid = |width| match width {
            WidgetryTableColumnWidth::Fixed(value) | WidgetryTableColumnWidth::Flexible(value) => {
                positive(value)
            }
        };
        if [
            self.row_height,
            self.column_header_height,
            self.row_header_width,
            self.min_column_width,
        ]
        .into_iter()
        .all(positive)
            && width_valid(self.default_column_width)
            && self.columns.values().copied().all(width_valid)
        {
            Ok(())
        } else {
            Err(BevyError::error(
                "Table layout dimensions and width weights must be finite and positive",
            ))
        }
    }

    pub(crate) fn resolve(
        &self,
        ids: Vec<WidgetryTableColumnId>,
        rows: usize,
        viewport: f32,
    ) -> Result<TableGeometry, BevyError> {
        self.validate()?;
        let widths: Vec<_> = ids
            .iter()
            .map(|id| {
                self.columns
                    .get(id)
                    .copied()
                    .unwrap_or(self.default_column_width)
            })
            .collect();
        let mut fixed = 0.0;
        let mut weight = 0.0;
        let mut flexible = 0;
        for width in &widths {
            match width {
                WidgetryTableColumnWidth::Fixed(value) => fixed += value.max(self.min_column_width),
                WidgetryTableColumnWidth::Flexible(value) => {
                    weight += value;
                    flexible += 1;
                }
            }
        }
        let remaining =
            (viewport.max(0.0) - fixed - flexible as f32 * self.min_column_width).max(0.0);
        let mut geometry = TableGeometry {
            row_height: self.row_height,
            height: rows as f32 * self.row_height,
            ..default()
        };
        for (index, (id, width)) in ids.into_iter().zip(widths).enumerate() {
            let width = match width {
                WidgetryTableColumnWidth::Fixed(value) => value.max(self.min_column_width),
                WidgetryTableColumnWidth::Flexible(value) => {
                    self.min_column_width + remaining * (value / weight)
                }
            };
            geometry.columns.push(ColumnGeometry {
                id,
                index,
                left: geometry.width,
                width,
            });
            geometry.width += width;
        }
        if !fixed.is_finite()
            || !weight.is_finite()
            || !geometry.width.is_finite()
            || !geometry.height.is_finite()
        {
            return Err(BevyError::error("Table total geometry must be finite"));
        }
        Ok(geometry)
    }
}

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableHeaderValue, WidgetryTableModel,
    };

    #[test]
    fn fixed_and_flexible_widths_share_one_ordered_geometry() {
        let mut model = WidgetryTableModel::<()>::default();
        let ids: Vec<_> = (0..3)
            .map(|_| {
                model
                    .push_column(WidgetryTableColumn::new(
                        WidgetryTableHeaderValue::new(()),
                        (),
                        |_, _| WidgetryTableCellValue::new(()),
                    ))
                    .unwrap()
            })
            .collect();
        let mut layout = WidgetryTableLayout {
            min_column_width: 20.0,
            ..default()
        };
        layout
            .columns
            .insert(ids[0], WidgetryTableColumnWidth::Fixed(100.0));
        layout
            .columns
            .insert(ids[1], WidgetryTableColumnWidth::Flexible(1.0));
        layout
            .columns
            .insert(ids[2], WidgetryTableColumnWidth::Flexible(3.0));
        let geometry = layout.resolve(ids.clone(), 2, 500.0).unwrap();
        assert_eq!(geometry.width, 500.0);
        assert_eq!(
            geometry
                .columns
                .iter()
                .map(|column| column.width)
                .collect::<Vec<_>>(),
            vec![100.0, 110.0, 290.0]
        );
        assert_eq!(
            geometry
                .columns
                .iter()
                .map(|column| column.left)
                .collect::<Vec<_>>(),
            vec![0.0, 100.0, 210.0]
        );
        assert_eq!(layout.resolve(ids, 0, 50.0).unwrap().width, 140.0);
        assert_eq!(layout.resolve(vec![], 2, 500.0).unwrap().width, 0.0);
    }

    #[test]
    fn invalid_and_overflow_geometry_return_error() {
        for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let layout = WidgetryTableLayout {
                row_height: value,
                ..default()
            };
            assert!(layout.validate().is_err());
            let layout = WidgetryTableLayout {
                default_column_width: WidgetryTableColumnWidth::Flexible(value),
                ..default()
            };
            assert!(layout.validate().is_err());
        }
        let layout = WidgetryTableLayout {
            row_height: f32::MAX,
            ..default()
        };
        let error = layout.resolve(vec![], 2, 0.0).err().unwrap();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    }
}
