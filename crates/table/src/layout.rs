use crate::WidgetryTableColumnId;
use bevy::prelude::*;
use std::collections::HashMap;

/// Column width 属于 View，Model 不保存任何几何数据。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetryTableColumnWidth {
    /// logical px；实际 width 至少为 min_column_width。
    Fixed(f32),
    /// 正权重；先保留最小宽度，再按权重分配额外 viewport width；空间不足时允许水平 scroll。
    Flexible(f32),
}

/// View 几何配置；公开尺寸/default policy 是展示输入，长度与权重必须为有限正数。
/// per-column override 仅只读查询，初始化用 with_column_width；运行期通过 Table::set_column_width 更新。
#[derive(Component, Clone)]
pub struct WidgetryTableLayout {
    /// 固定 Row 高度。
    pub row_height: f32,
    /// 不参与纵向 scroll 的 Column Header 高度。
    pub column_header_height: f32,
    /// 不参与横向 scroll 的 Row Header 宽度。
    pub row_header_width: f32,
    /// 未独立配置的 Column 使用此策略。
    pub default_column_width: WidgetryTableColumnWidth,
    /// 所有 Column 的最小实际宽度。
    pub min_column_width: f32,
    /// 以当前 source-local ColumnId 保存 per-view width；新 Column 使用默认值。
    pub(crate) columns: HashMap<WidgetryTableColumnId, WidgetryTableColumnWidth>,
}

/// 当前已求解的单个 Column 几何，Header 和 Body 共用同一数据。
#[derive(Clone)]
pub(crate) struct ColumnGeometry {
    pub(crate) id: WidgetryTableColumnId,
    pub(crate) index: usize,
    pub(crate) left: f32,
    pub(crate) width: f32,
}

/// 有序二维 canvas 的完整范围，供当前 projection 与后续 virtualization 使用。
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
    /// 一次性配置 source-local Column 的初始 Fixed/Flexible policy；Scene 构造时验证数值。
    /// 不用于已挂载 View 的运行期更新，不发送通知。
    pub fn with_column_width(
        mut self,
        column: WidgetryTableColumnId,
        width: WidgetryTableColumnWidth,
    ) -> Self {
        self.columns.insert(column, width);
        self
    }

    /// 查询显式 override；未配置的 Column 使用 default_column_width。
    /// policy 不等于已完成 layout 的尺寸；Flexible 与 viewport 自动求解不发 resize 通知。
    pub fn column_widths(&self) -> &HashMap<WidgetryTableColumnId, WidgetryTableColumnWidth> {
        &self.columns
    }

    /// 构造和每次 runtime 消费都验证同一个数值 contract；日志由消费边界负责。
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

    /// fixed 优先，flexible 分配扣除 fixed 后的剩余空间；求解前验证累计值不会溢出。
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

// unit test 的断言验证数值 contract，生产代码仍禁止主动 panic。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableHeaderValue, WidgetryTableModel,
    };

    /// fixed 先占用空间，flexible 先保留最小宽度再按权重分配额外空间；不足时允许 canvas 超过 viewport。
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

    /// 非有限/非正尺寸、权重和累计范围拒绝求解，空 Axis 与零 viewport 仍保持合法布局。
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
