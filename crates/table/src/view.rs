use crate::{WidgetryTableColumnId, WidgetryTableLayout, WidgetryTableRowId, WidgetryTableStyle};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::{GridPlacement, ScrollPosition};
use bevy::ui_widgets::ScrollArea;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_log::widgetry_error;
use std::marker::PhantomData;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTableProps)]
#[require(TableDiagnostics, crate::WidgetryTableState)]
pub struct WidgetryTable<T: Send + Sync + 'static> {
    source: Entity,
    marker: PhantomData<fn() -> T>,
}

pub struct WidgetryTableProps {
    pub source: Entity,
    pub layout: WidgetryTableLayout,
    pub style: WidgetryTableStyle,
}

#[derive(Component, Default)]
pub(crate) struct TableDiagnostics(pub(crate) FailureState);

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableBody;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableColumnHeaders;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableRowHeaders;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableCorner;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableCell {
    pub row: WidgetryTableRowId,
    pub column: WidgetryTableColumnId,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableColumnHeader {
    pub column: WidgetryTableColumnId,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableRowHeader {
    pub row: WidgetryTableRowId,
    pub index: usize,
}

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
    pub fn source(&self) -> Entity {
        self.source
    }

    pub fn set_selection(
        world: &mut World,
        root: Entity,
        selection: crate::WidgetryTableSelection,
    ) -> Result<bool, BevyError> {
        crate::interaction::set_selection::<T>(world, root, selection)
            .inspect_err(|error| widgetry_error!(?root,%error,"Table 程序化selection失败"))
    }

    pub fn set_focused_cell(
        world: &mut World,
        root: Entity,
        cell: Option<crate::WidgetryTableCell>,
    ) -> Result<bool, BevyError> {
        crate::interaction::set_focused_cell::<T>(world, root, cell)
            .inspect_err(|error| widgetry_error!(?root, %error, "Table 程序化cursor失败"))
    }

    pub fn set_column_width(
        world: &mut World,
        root: Entity,
        column: WidgetryTableColumnId,
        width: f32,
    ) -> Result<bool, BevyError> {
        crate::resize::set_width::<T>(world, root, column, width).inspect_err(
            |error| widgetry_error!(?root, ?column, width, %error, "Table 程序化Column width失败"),
        )
    }

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
            TabIndex::default()
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
                    Children [(TableCanvas LayoutConfig { use_rounding: false } Node { flex_shrink: 0.0 })]),
                (WidgetryTableRowHeaders ScrollPosition::default()
                    Node { grid_column: GridPlacement::start(1), grid_row: GridPlacement::start(2), min_width: px(0), min_height: px(0), overflow: Overflow::scroll_y(), scrollbar_width: 0.0 }
                    Children [(TableCanvas LayoutConfig { use_rounding: false } Node { flex_shrink: 0.0 })]),
                (WidgetryTableBody ScrollArea
                    Node { grid_column: GridPlacement::start(2), grid_row: GridPlacement::start(2), min_width: px(0), min_height: px(0), overflow: Overflow::scroll(), scrollbar_width: 0.0 }
                    Children [(TableCanvas LayoutConfig { use_rounding: false } Node { flex_shrink: 0.0 })]),
            ]
        }
    }
}
