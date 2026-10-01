use crate::{WidgetryTableColumnId, WidgetryTableLayout, WidgetryTableRowId, WidgetryTableStyle};
use bevy::prelude::*;
use bevy::ui::{GridPlacement, ScrollPosition};
use bevy::ui_widgets::ScrollArea;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_log::widgetry_error;
use std::marker::PhantomData;

/// BSN 二维 Table identity；先通过 WidgetryTableAppExt 注册业务 T 与实际 Cell/Header value renderer。
/// source 创建后固定，生命周期内必须持有 WidgetryTableModel<T>；源数据始终由调用方拥有。
/// 调用方在 root Node 提供有界宽高。layout/style 是可修改的独立 Component，不保留 props 副本。
/// Body 支持两轴 wheel/trackpad 与原生 ScrollPosition，Header 只同步对应轴，无 scrollbar gutter。
/// Row Header 显示从 1 开始的当前行号，Corner 为空。所有 Content 都属于 Table subtree。
#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTableProps)]
#[require(TableDiagnostics)]
pub struct WidgetryTable<T: Send + Sync + 'static> {
    source: Entity,
    marker: PhantomData<fn() -> T>,
}

/// 一次性 source/layout/style 初始化；后续读取长期 Component。
pub struct WidgetryTableProps {
    pub source: Entity,
    pub layout: WidgetryTableLayout,
    pub style: WidgetryTableStyle,
}

/// 持有 Table 自身的持续异常边界，不依赖 update 次数。
#[derive(Component, Default)]
pub(crate) struct TableDiagnostics(pub(crate) FailureState);

/// 持有两轴原生 ScrollPosition 的 Body viewport。
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableBody;

/// 固定纵向位置、只镜像 Body 水平 scroll 的 Header viewport。
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableColumnHeaders;

/// 固定横向位置、只镜像 Body 垂直 scroll 的 Header viewport。
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableRowHeaders;

/// 固定且为空的 Corner，不承担全选操作。
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableCorner;

/// physical Cell 的当前 model-local logical pair，Content 子 entity 通过此边界路由。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryTableCell {
    pub row: WidgetryTableRowId,
    pub column: WidgetryTableColumnId,
}

/// physical Column Header 当前指向的 logical identity。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryTableColumnHeader {
    pub column: WidgetryTableColumnId,
}

/// physical Row Header 当前指向的 logical identity；index 仅为当前顺序的 projection。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryTableRowHeader {
    pub row: WidgetryTableRowId,
    pub index: usize,
}

/// 为绝对定位的 direct Cell/Header children 保留完整 canvas 几何。
#[derive(Component, Default, Clone)]
pub(crate) struct TableCanvas;

impl Default for WidgetryTableProps {
    fn default() -> Self {
        Self {
            source: Entity::PLACEHOLDER,
            layout: default(),
            style: default(),
        }
    }
}

impl<T: Send + Sync + 'static> WidgetryTable<T> {
    /// 当前 source identity，构造后固定。
    pub fn source(&self) -> Entity {
        self.source
    }

    /// fallible template 校验必填配置，使用四区 Grid 与三个独立 clipped canvas。
    fn scene(props: WidgetryTableProps) -> impl Scene {
        let source = props.source;
        let layout = props.layout;
        let style = props.style;
        let header_height = layout.column_header_height;
        let header_width = layout.row_header_width;
        bsn! {
            template(move |_| {
                if source == Entity::PLACEHOLDER {
                    widgetry_error!("Table 构造必须提供 source");
                    return Err(logged_error("WidgetryTable requires source"));
                }
                if let Err(error) = layout.validate() {
                    widgetry_error!(%error, "Table 构造 layout 无效");
                    return Err(logged_error("WidgetryTable layout must be finite and positive"));
                }
                Ok(layout.clone())
            })
            template(move |_| Ok(style.clone()))
            WidgetryTable::<T> { source: {source}, marker: PhantomData }
            BackgroundColor::default() BorderColor::default()
            Node {
                display: Display::Grid, min_width: px(0), min_height: px(0), overflow: Overflow::clip(),
                grid_template_columns: vec![RepeatedGridTrack::px(1, header_width), RepeatedGridTrack::flex(1, 1.0)],
                grid_template_rows: vec![RepeatedGridTrack::px(1, header_height), RepeatedGridTrack::flex(1, 1.0)],
            }
            Children [
                (WidgetryTableCorner BackgroundColor::default() BorderColor::default()
                    Node { grid_column: GridPlacement::start(1), grid_row: GridPlacement::start(1), overflow: Overflow::clip() }),
                (WidgetryTableColumnHeaders ScrollPosition::default()
                    Node { grid_column: GridPlacement::start(2), grid_row: GridPlacement::start(1), min_width: px(0), min_height: px(0), overflow: Overflow::scroll_x(), scrollbar_width: 0.0 }
                    Children [(TableCanvas Node { flex_shrink: 0.0 })]),
                (WidgetryTableRowHeaders ScrollPosition::default()
                    Node { grid_column: GridPlacement::start(1), grid_row: GridPlacement::start(2), min_width: px(0), min_height: px(0), overflow: Overflow::scroll_y(), scrollbar_width: 0.0 }
                    Children [(TableCanvas Node { flex_shrink: 0.0 })]),
                (WidgetryTableBody ScrollArea
                    Node { grid_column: GridPlacement::start(2), grid_row: GridPlacement::start(2), min_width: px(0), min_height: px(0), overflow: Overflow::scroll(), scrollbar_width: 0.0 }
                    Children [(TableCanvas Node { flex_shrink: 0.0 })]),
            ]
        }
    }
}
