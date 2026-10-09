use bevy::prelude::*;

#[derive(Clone, Default)]
pub struct WidgetryTableRegionStyle {
    pub padding: UiRect,
    pub border: UiRect,
}

#[derive(Component, Clone)]
pub struct WidgetryTableStyle {
    pub table: WidgetryTableRegionStyle,
    pub column_header: WidgetryTableRegionStyle,
    pub row_header: WidgetryTableRegionStyle,
    pub corner: WidgetryTableRegionStyle,
    pub cell: WidgetryTableRegionStyle,
}

impl Default for WidgetryTableStyle {
    fn default() -> Self {
        let item = WidgetryTableRegionStyle {
            padding: UiRect::horizontal(px(6)),
            border: UiRect::all(px(1)),
        };
        Self {
            table: WidgetryTableRegionStyle {
                border: UiRect::all(px(1)),
                ..default()
            },
            column_header: item.clone(),
            row_header: item.clone(),
            corner: item.clone(),
            cell: item,
        }
    }
}
