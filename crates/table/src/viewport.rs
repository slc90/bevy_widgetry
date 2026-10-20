use crate::layout::TableGeometry;
use bevy::prelude::*;
use std::ops::Range;

pub(crate) struct VisibleCells {
    pub(crate) rows: Range<usize>,
    frozen: Range<usize>,
    pub(crate) columns: Range<usize>,
}

impl VisibleCells {
    pub(crate) fn new(geometry: &TableGeometry, rows: usize, offset: Vec2, size: Vec2) -> Self {
        if !size.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
            return Self {
                rows: 0..0,
                frozen: 0..0,
                columns: 0..0,
            };
        }
        let first = ((offset.y / geometry.row_height).floor() as usize).min(rows);
        let last = (((offset.y + size.y) / geometry.row_height).ceil() as usize).min(rows);
        let columns = &geometry.columns;
        let frozen_count = columns.partition_point(|column| column.frozen);
        let width = size.x.min(geometry.viewport_width).max(0.0);
        let frozen_end = columns
            .partition_point(|column| column.left < width)
            .min(frozen_count);
        let scrolling = if width <= geometry.frozen_width {
            frozen_count..frozen_count
        } else {
            columns
                .partition_point(|column| {
                    column.left + column.width <= offset.x + geometry.frozen_width
                })
                .max(frozen_count)
                ..columns
                    .partition_point(|column| column.left < offset.x + width)
                    .max(frozen_count)
        };
        Self {
            rows: first..last,
            frozen: 0..frozen_end,
            columns: scrolling,
        }
    }

    pub(crate) fn column_indices(&self) -> impl Iterator<Item = usize> + Clone {
        self.frozen.clone().chain(self.columns.clone())
    }

    pub(crate) fn contains_column(&self, index: usize) -> bool {
        self.frozen.contains(&index) || self.columns.contains(&index)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableColumnWidth,
        WidgetryTableHeaderValue, WidgetryTableLayout, WidgetryTableModel,
    };

    #[test]
    fn boundaries_use_actual_column_geometry() {
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
            row_height: 20.0,
            ..default()
        };
        layout
            .columns
            .insert(ids[0], WidgetryTableColumnWidth::Fixed(40.0));
        layout
            .columns
            .insert(ids[1], WidgetryTableColumnWidth::Fixed(80.0));
        let geometry = layout.resolve(ids, 10, 100.0).unwrap();
        let visible =
            VisibleCells::new(&geometry, 10, Vec2::new(40.0, 20.0), Vec2::new(80.0, 40.0));
        assert_eq!(visible.rows, 1..3);
        assert_eq!(visible.columns, 1..2);
        let visible =
            VisibleCells::new(&geometry, 10, Vec2::new(39.0, 19.0), Vec2::new(82.0, 42.0));
        assert_eq!(visible.rows, 0..4);
        assert_eq!(visible.columns, 0..3);
        for size in [
            Vec2::ZERO,
            Vec2::new(0.0, 50.0),
            Vec2::new(50.0, 0.0),
            Vec2::new(-1.0, 50.0),
            Vec2::splat(f32::NAN),
            Vec2::splat(f32::INFINITY),
        ] {
            let visible = VisibleCells::new(&geometry, 10, Vec2::ZERO, size);
            assert!(visible.rows.is_empty());
            assert!(visible.columns.is_empty());
        }
        let empty = layout.resolve(vec![], 0, 100.0).unwrap();
        let visible = VisibleCells::new(&empty, 0, Vec2::ZERO, Vec2::splat(100.0));
        assert!(visible.rows.is_empty());
        assert!(visible.columns.is_empty());
    }
}
