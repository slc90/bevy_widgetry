const ROW_HEADER_COLUMN: i16 = 1;
const FROZEN_COLUMN: i16 = 2;
const SCROLLING_COLUMN: i16 = 3;
const HEADER_ROW: i16 = 1;
const BODY_ROW: i16 = 2;

use crate::{WidgetryTableColumnId, WidgetryTableLayout, WidgetryTableRowId, WidgetryTableStyle};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::{GridPlacement, ScrollPosition};
use bevy::ui_widgets::ScrollArea;
use std::marker::PhantomData;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTableProps)]
#[require(crate::error::LayoutDiagnostics, TableDiagnostics, crate::WidgetryTableState, crate::WidgetryTableViewState, crate::style::TableStyle, crate::colors::ColorState = crate::colors::ColorState::new::<T>())]
pub struct WidgetryTable<T: Send + Sync + 'static> {
    source: Entity,
    marker: PhantomData<fn() -> T>,
}

pub struct WidgetryTableProps {
    pub column_reordering: crate::WidgetryTableColumnReordering,
    pub editing: crate::WidgetryTableEditing,
    pub header_actions: crate::WidgetryTableHeaderActions,
    pub colors: crate::WidgetryTableColorOverrides,
    pub source: Entity,
    pub layout: WidgetryTableLayout,
    pub style: WidgetryTableStyle,
}

#[derive(Component, Default)]
pub(crate) struct TableDiagnostics(pub(crate) crate::runtime_error::RuntimeDiagnostics);

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableBody;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableFrozenBody;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct WidgetryTableFrozenColumnHeaders;

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
            column_reordering: default(),
            editing: default(),
            header_actions: default(),
            colors: default(),
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
    ) -> Result<bool, crate::TableFailure> {
        crate::interaction::set_selection::<T>(world, root, selection).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("table.set_selection")
                    .field("selection", format_args!("{selection:?}")),
            )
        })
    }
    pub fn set_focused_cell(
        world: &mut World,
        root: Entity,
        cell: Option<crate::WidgetryTableCell>,
    ) -> Result<bool, crate::TableFailure> {
        crate::interaction::set_focused_cell::<T>(world, root, cell).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("table.set_focused_cell")
                    .field("cell", format_args!("{cell:?}")),
            )
        })
    }
    pub fn set_column_width(
        world: &mut World,
        root: Entity,
        column: WidgetryTableColumnId,
        width: f32,
    ) -> Result<bool, crate::TableFailure> {
        crate::resize::set_width::<T>(world, root, column, width).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("table.set_column_width")
                    .field("column", format_args!("{column:?}"))
                    .field("width", width),
            )
        })
    }

    fn scene(props: WidgetryTableProps) -> impl Scene {
        let column_reordering = props.column_reordering;
        let editing = props.editing;
        let header_actions = props.header_actions;
        let source = props.source;
        let layout = props.layout;
        let style = props.style;
        let header_height = layout.column_header_height;
        let header_width = layout.row_header_width;
        bsn! {
            template(move |_| Ok(column_reordering))
            template(move |_| Ok(editing.clone()))
            bevy_widgetry_core::foreground::ResolvedForeground
            template(move |_| props.colors.clone().initial::<T>().map_err(crate::TableFailure::into_bevy_error))
            template(move |ctx| {
                if source == Entity::PLACEHOLDER {
                    return Err(bevy_widgetry_core::scene_source_error!("WidgetryTable requires source"; "Table 构造必须提供 source"));
                }
                layout.validate().map_err(|fault|crate::error::layout_failure(fault,ctx.entity.id(),Some(source),"table.scene.layout").into_bevy_error())?;
                Ok(layout.clone())
            })
            template(move |_| Ok(style.clone()))
            template(move |_| Ok(header_actions.clone()))
            WidgetryTable::<T> { source: {source}, marker: PhantomData }
            TabIndex(-1)
            BackgroundColor::default() BorderColor::default()
            Node {
                display: Display::Grid, min_width: px(0), min_height: px(0), overflow: Overflow::clip(),
                grid_template_columns: vec![RepeatedGridTrack::px(1, header_width), RepeatedGridTrack::px(1,0.0), RepeatedGridTrack::flex(1, 1.0)],
                grid_template_rows: vec![RepeatedGridTrack::px(1, header_height), RepeatedGridTrack::flex(1, 1.0)],
            }
            Children [
                WidgetryTableCorner BackgroundColor::default() BorderColor::default()
                    Node { grid_column: GridPlacement::start(ROW_HEADER_COLUMN), grid_row: GridPlacement::start(HEADER_ROW), overflow: Overflow::clip() }--
                WidgetryTableColumnHeaders ScrollPosition::default()
                    Node { grid_column: GridPlacement::start(SCROLLING_COLUMN), grid_row: GridPlacement::start(HEADER_ROW), min_width: px(0), min_height: px(0), overflow: Overflow::scroll_x(), scrollbar_width: 0.0 }
                    Children [TableCanvas LayoutConfig { use_rounding: false } Node { flex_shrink: 0.0 }]--
                WidgetryTableRowHeaders ScrollPosition::default()
                    Node { grid_column: GridPlacement::start(ROW_HEADER_COLUMN), grid_row: GridPlacement::start(BODY_ROW), min_width: px(0), min_height: px(0), overflow: Overflow::scroll_y(), scrollbar_width: 0.0 }
                    Children [TableCanvas LayoutConfig { use_rounding: false } Node { flex_shrink: 0.0 }]--
                WidgetryTableBody ScrollArea
                    Node { grid_column: GridPlacement::start(SCROLLING_COLUMN), grid_row: GridPlacement::start(BODY_ROW), min_width: px(0), min_height: px(0), overflow: Overflow::scroll(), scrollbar_width: 0.0 }
                    Children [TableCanvas LayoutConfig { use_rounding: false } Node { flex_shrink: 0.0 }]
                -- WidgetryTableFrozenColumnHeaders
                    Node {grid_column:GridPlacement::start(FROZEN_COLUMN),grid_row:GridPlacement::start(HEADER_ROW),min_width:px(0),min_height:px(0),overflow:Overflow::clip()}
                    Children [TableCanvas LayoutConfig{use_rounding:false} Node{flex_shrink:0.0}]
                -- WidgetryTableFrozenBody ScrollPosition::default()
                    Node {grid_column:GridPlacement::start(FROZEN_COLUMN),grid_row:GridPlacement::start(BODY_ROW),min_width:px(0),min_height:px(0),overflow:Overflow{x:OverflowAxis::Clip,y:OverflowAxis::Scroll},scrollbar_width:0.0}
                    Children [TableCanvas LayoutConfig{use_rounding:false} Node{flex_shrink:0.0}]
            ]
        }
    }
}
