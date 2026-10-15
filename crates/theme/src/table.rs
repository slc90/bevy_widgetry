use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTableStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTableRegionColors {
    pub normal: WidgetryTableStateColors,
    pub hovered: WidgetryTableStateColors,
    pub selected: WidgetryTableStateColors,
    pub disabled: WidgetryTableStateColors,
    pub focused_border: Color,
    pub disabled_focused_border: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryTableColors {
    pub table: WidgetryTableRegionColors,
    pub column_header: WidgetryTableRegionColors,
    pub row_header: WidgetryTableRegionColors,
    pub corner: WidgetryTableRegionColors,
    pub cell: WidgetryTableRegionColors,
}

pub const LIGHT: WidgetryTableColors = WidgetryTableColors {
    table: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: light::SURFACE,
            border: light::BORDER,
            foreground: light::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: light::HOVER_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: light::SELECTED_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
        focused_border: light::FOCUS_BORDER,
        disabled_focused_border: light::DISABLED_BORDER,
    },
    column_header: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: light::HOVER_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: light::HOVER_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: light::SELECTED_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
        focused_border: light::FOCUS_BORDER,
        disabled_focused_border: light::DISABLED_BORDER,
    },
    row_header: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: light::HOVER_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: light::HOVER_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: light::SELECTED_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
        focused_border: light::FOCUS_BORDER,
        disabled_focused_border: light::DISABLED_BORDER,
    },
    corner: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: light::HOVER_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: light::HOVER_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: light::SELECTED_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
        focused_border: light::FOCUS_BORDER,
        disabled_focused_border: light::DISABLED_BORDER,
    },
    cell: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: light::TRANSPARENT,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: light::HOVER_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: light::SELECTED_SURFACE,
            border: light::SUBTLE_BORDER,
            foreground: light::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: light::DISABLED_SURFACE,
            border: light::DISABLED_BORDER,
            foreground: light::DISABLED_TEXT,
        },
        focused_border: light::FOCUS_BORDER,
        disabled_focused_border: light::DISABLED_BORDER,
    },
};

pub const DARK: WidgetryTableColors = WidgetryTableColors {
    table: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: dark::SURFACE,
            border: dark::BORDER,
            foreground: dark::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: dark::SELECTED_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
        focused_border: dark::FOCUS_BORDER,
        disabled_focused_border: dark::DISABLED_BORDER,
    },
    column_header: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: dark::SELECTED_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
        focused_border: dark::FOCUS_BORDER,
        disabled_focused_border: dark::DISABLED_BORDER,
    },
    row_header: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: dark::SELECTED_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
        focused_border: dark::FOCUS_BORDER,
        disabled_focused_border: dark::DISABLED_BORDER,
    },
    corner: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: dark::SELECTED_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
        focused_border: dark::FOCUS_BORDER,
        disabled_focused_border: dark::DISABLED_BORDER,
    },
    cell: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: dark::TRANSPARENT,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: dark::HOVER_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: dark::SELECTED_SURFACE,
            border: dark::SUBTLE_BORDER,
            foreground: dark::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: dark::DISABLED_SURFACE,
            border: dark::DISABLED_BORDER,
            foreground: dark::DISABLED_TEXT,
        },
        focused_border: dark::FOCUS_BORDER,
        disabled_focused_border: dark::DISABLED_BORDER,
    },
};

pub const PINK_DREAM: WidgetryTableColors = WidgetryTableColors {
    table: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: pink_dream::SURFACE,
            border: pink_dream::BORDER,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: pink_dream::SELECTED_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
        focused_border: pink_dream::FOCUS_BORDER,
        disabled_focused_border: pink_dream::DISABLED_BORDER,
    },
    column_header: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: pink_dream::SELECTED_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
        focused_border: pink_dream::FOCUS_BORDER,
        disabled_focused_border: pink_dream::DISABLED_BORDER,
    },
    row_header: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: pink_dream::SELECTED_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
        focused_border: pink_dream::FOCUS_BORDER,
        disabled_focused_border: pink_dream::DISABLED_BORDER,
    },
    corner: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: pink_dream::SELECTED_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
        focused_border: pink_dream::FOCUS_BORDER,
        disabled_focused_border: pink_dream::DISABLED_BORDER,
    },
    cell: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: pink_dream::TRANSPARENT,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: pink_dream::HOVER_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: pink_dream::SELECTED_SURFACE,
            border: pink_dream::SUBTLE_BORDER,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: pink_dream::DISABLED_SURFACE,
            border: pink_dream::DISABLED_BORDER,
            foreground: pink_dream::DISABLED_TEXT,
        },
        focused_border: pink_dream::FOCUS_BORDER,
        disabled_focused_border: pink_dream::DISABLED_BORDER,
    },
};

pub const KAMURI_VIOLET: WidgetryTableColors = WidgetryTableColors {
    table: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: kamuri_violet::SELECTED_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
        focused_border: kamuri_violet::FOCUS_BORDER,
        disabled_focused_border: kamuri_violet::DISABLED_BORDER,
    },
    column_header: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: kamuri_violet::SELECTED_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
        focused_border: kamuri_violet::FOCUS_BORDER,
        disabled_focused_border: kamuri_violet::DISABLED_BORDER,
    },
    row_header: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: kamuri_violet::SELECTED_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
        focused_border: kamuri_violet::FOCUS_BORDER,
        disabled_focused_border: kamuri_violet::DISABLED_BORDER,
    },
    corner: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: kamuri_violet::SELECTED_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
        focused_border: kamuri_violet::FOCUS_BORDER,
        disabled_focused_border: kamuri_violet::DISABLED_BORDER,
    },
    cell: WidgetryTableRegionColors {
        normal: WidgetryTableStateColors {
            background: kamuri_violet::TRANSPARENT,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryTableStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        selected: WidgetryTableStateColors {
            background: kamuri_violet::SELECTED_SURFACE,
            border: kamuri_violet::SUBTLE_BORDER,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryTableStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
        focused_border: kamuri_violet::FOCUS_BORDER,
        disabled_focused_border: kamuri_violet::DISABLED_BORDER,
    },
};

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn light_colors_are_finite() {
        let colors = [
            LIGHT.table.normal.background,
            LIGHT.table.normal.border,
            LIGHT.table.normal.foreground,
            LIGHT.table.hovered.background,
            LIGHT.table.hovered.border,
            LIGHT.table.hovered.foreground,
            LIGHT.table.selected.background,
            LIGHT.table.selected.border,
            LIGHT.table.selected.foreground,
            LIGHT.table.disabled.background,
            LIGHT.table.disabled.border,
            LIGHT.table.disabled.foreground,
            LIGHT.table.focused_border,
            LIGHT.table.disabled_focused_border,
            LIGHT.column_header.normal.background,
            LIGHT.column_header.normal.border,
            LIGHT.column_header.normal.foreground,
            LIGHT.column_header.hovered.background,
            LIGHT.column_header.hovered.border,
            LIGHT.column_header.hovered.foreground,
            LIGHT.column_header.selected.background,
            LIGHT.column_header.selected.border,
            LIGHT.column_header.selected.foreground,
            LIGHT.column_header.disabled.background,
            LIGHT.column_header.disabled.border,
            LIGHT.column_header.disabled.foreground,
            LIGHT.column_header.focused_border,
            LIGHT.column_header.disabled_focused_border,
            LIGHT.row_header.normal.background,
            LIGHT.row_header.normal.border,
            LIGHT.row_header.normal.foreground,
            LIGHT.row_header.hovered.background,
            LIGHT.row_header.hovered.border,
            LIGHT.row_header.hovered.foreground,
            LIGHT.row_header.selected.background,
            LIGHT.row_header.selected.border,
            LIGHT.row_header.selected.foreground,
            LIGHT.row_header.disabled.background,
            LIGHT.row_header.disabled.border,
            LIGHT.row_header.disabled.foreground,
            LIGHT.row_header.focused_border,
            LIGHT.row_header.disabled_focused_border,
            LIGHT.corner.normal.background,
            LIGHT.corner.normal.border,
            LIGHT.corner.normal.foreground,
            LIGHT.corner.hovered.background,
            LIGHT.corner.hovered.border,
            LIGHT.corner.hovered.foreground,
            LIGHT.corner.selected.background,
            LIGHT.corner.selected.border,
            LIGHT.corner.selected.foreground,
            LIGHT.corner.disabled.background,
            LIGHT.corner.disabled.border,
            LIGHT.corner.disabled.foreground,
            LIGHT.corner.focused_border,
            LIGHT.corner.disabled_focused_border,
            LIGHT.cell.normal.background,
            LIGHT.cell.normal.border,
            LIGHT.cell.normal.foreground,
            LIGHT.cell.hovered.background,
            LIGHT.cell.hovered.border,
            LIGHT.cell.hovered.foreground,
            LIGHT.cell.selected.background,
            LIGHT.cell.selected.border,
            LIGHT.cell.selected.foreground,
            LIGHT.cell.disabled.background,
            LIGHT.cell.disabled.border,
            LIGHT.cell.disabled.foreground,
            LIGHT.cell.focused_border,
            LIGHT.cell.disabled_focused_border,
        ];
        for color in colors {
            let actual = color.to_srgba();
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
    #[test]
    fn dark_colors_are_finite() {
        let colors = [
            DARK.table.normal.background,
            DARK.table.normal.border,
            DARK.table.normal.foreground,
            DARK.table.hovered.background,
            DARK.table.hovered.border,
            DARK.table.hovered.foreground,
            DARK.table.selected.background,
            DARK.table.selected.border,
            DARK.table.selected.foreground,
            DARK.table.disabled.background,
            DARK.table.disabled.border,
            DARK.table.disabled.foreground,
            DARK.table.focused_border,
            DARK.table.disabled_focused_border,
            DARK.column_header.normal.background,
            DARK.column_header.normal.border,
            DARK.column_header.normal.foreground,
            DARK.column_header.hovered.background,
            DARK.column_header.hovered.border,
            DARK.column_header.hovered.foreground,
            DARK.column_header.selected.background,
            DARK.column_header.selected.border,
            DARK.column_header.selected.foreground,
            DARK.column_header.disabled.background,
            DARK.column_header.disabled.border,
            DARK.column_header.disabled.foreground,
            DARK.column_header.focused_border,
            DARK.column_header.disabled_focused_border,
            DARK.row_header.normal.background,
            DARK.row_header.normal.border,
            DARK.row_header.normal.foreground,
            DARK.row_header.hovered.background,
            DARK.row_header.hovered.border,
            DARK.row_header.hovered.foreground,
            DARK.row_header.selected.background,
            DARK.row_header.selected.border,
            DARK.row_header.selected.foreground,
            DARK.row_header.disabled.background,
            DARK.row_header.disabled.border,
            DARK.row_header.disabled.foreground,
            DARK.row_header.focused_border,
            DARK.row_header.disabled_focused_border,
            DARK.corner.normal.background,
            DARK.corner.normal.border,
            DARK.corner.normal.foreground,
            DARK.corner.hovered.background,
            DARK.corner.hovered.border,
            DARK.corner.hovered.foreground,
            DARK.corner.selected.background,
            DARK.corner.selected.border,
            DARK.corner.selected.foreground,
            DARK.corner.disabled.background,
            DARK.corner.disabled.border,
            DARK.corner.disabled.foreground,
            DARK.corner.focused_border,
            DARK.corner.disabled_focused_border,
            DARK.cell.normal.background,
            DARK.cell.normal.border,
            DARK.cell.normal.foreground,
            DARK.cell.hovered.background,
            DARK.cell.hovered.border,
            DARK.cell.hovered.foreground,
            DARK.cell.selected.background,
            DARK.cell.selected.border,
            DARK.cell.selected.foreground,
            DARK.cell.disabled.background,
            DARK.cell.disabled.border,
            DARK.cell.disabled.foreground,
            DARK.cell.focused_border,
            DARK.cell.disabled_focused_border,
        ];
        for color in colors {
            let actual = color.to_srgba();
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
}
