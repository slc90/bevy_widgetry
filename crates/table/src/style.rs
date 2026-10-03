use bevy::prelude::*;

#[derive(Clone, Default)]
pub struct WidgetryTableRegionStyle {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub foreground: Option<Color>,
    pub hovered_background: Option<Color>,
    pub selected_background: Option<Color>,
    pub focused_border_color: Option<Color>,
    pub disabled_background: Option<Color>,
    pub disabled_border_color: Option<Color>,
    pub disabled_foreground: Option<Color>,
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
            ..default()
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
