use bevy::prelude::*;

/// 区域 shell 的可配置外观；None color 使用当前 theme，不修改业务 Content 的 Node。
#[derive(Clone, Default)]
pub struct WidgetryTableRegionStyle {
    /// normal background；Cell 默认透明。
    pub background: Option<Color>,
    /// normal border color。
    pub border_color: Option<Color>,
    /// 传播到未显式覆盖颜色的 Content。
    pub foreground: Option<Color>,
    /// hover覆盖selected，None使用theme item_background_hovered。
    pub hovered_background: Option<Color>,
    /// logical selection的背景，None使用theme item_background_selected。
    pub selected_background: Option<Color>,
    /// 仅真实root input focus的cursor Cell使用此border，None使用theme control_border_active。
    pub focused_border_color: Option<Color>,
    /// disabled background 独立于 normal。
    pub disabled_background: Option<Color>,
    /// disabled border color。
    pub disabled_border_color: Option<Color>,
    /// disabled foreground。
    pub disabled_foreground: Option<Color>,
    /// shell 内部间距。
    pub padding: UiRect,
    /// shell border，不参与 Column width authority。
    pub border: UiRect,
}

/// 运行期独立区域 style；renderer 只生成 Content，Table 持续维护 shell。
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
