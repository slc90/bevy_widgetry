use crate::layout::TableGeometry;
use bevy::prelude::*;
use std::ops::Range;

/// 半开 viewport 的二维相交范围；不包含仅接触边界的 Cell，也不添加 overscan。
pub(crate) struct VisibleCells {
    pub(crate) rows: Range<usize>,
    pub(crate) columns: Range<usize>,
}

impl VisibleCells {
    /// 零、负数或非有限 viewport 不生成 Cell；scroll 已由消费边界统一 clamp。
    pub(crate) fn new(geometry: &TableGeometry, rows: usize, offset: Vec2, size: Vec2) -> Self {
        if !size.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
            return Self {
                rows: 0..0,
                columns: 0..0,
            };
        }
        let first = ((offset.y / geometry.row_height).floor() as usize).min(rows);
        let last = (((offset.y + size.y) / geometry.row_height).ceil() as usize).min(rows);
        let columns = &geometry.columns;
        Self {
            rows: first..last,
            columns: columns.partition_point(|column| column.left + column.width <= offset.x)
                ..columns.partition_point(|column| column.left < offset.x + size.x),
        }
    }
}

// unit test 的断言保护边界算法，生产代码仍禁止主动 panic。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableColumnWidth,
        WidgetryTableHeaderValue, WidgetryTableLayout, WidgetryTableModel,
    };

    /// 半开区间覆盖 exact、partial、空 Axis、零与非有限 viewport；变宽 Column 依据实际几何查询。
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
