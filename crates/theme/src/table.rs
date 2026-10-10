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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::color::ColorToPacked;

    #[test]
    fn pink_dream_matches_approved_palette() {
        let slots = [
            (PINK_DREAM.table.normal.background, 0xFCEAF3FFu32),
            (PINK_DREAM.table.normal.border, 0xE7CCD9FFu32),
            (PINK_DREAM.table.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.table.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.table.hovered.border, 0xF3E3ECFFu32),
            (PINK_DREAM.table.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.table.selected.background, 0xF5D3E6FFu32),
            (PINK_DREAM.table.selected.border, 0xF3E3ECFFu32),
            (PINK_DREAM.table.selected.foreground, 0x422B3CFFu32),
            (PINK_DREAM.table.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.table.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.table.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.table.focused_border, 0xB4437DFFu32),
            (PINK_DREAM.table.disabled_focused_border, 0xE5D9E0FFu32),
            (PINK_DREAM.column_header.normal.background, 0xF9DFEDFFu32),
            (PINK_DREAM.column_header.normal.border, 0xF3E3ECFFu32),
            (PINK_DREAM.column_header.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.column_header.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.column_header.hovered.border, 0xF3E3ECFFu32),
            (PINK_DREAM.column_header.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.column_header.selected.background, 0xF5D3E6FFu32),
            (PINK_DREAM.column_header.selected.border, 0xF3E3ECFFu32),
            (PINK_DREAM.column_header.selected.foreground, 0x422B3CFFu32),
            (PINK_DREAM.column_header.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.column_header.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.column_header.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.column_header.focused_border, 0xB4437DFFu32),
            (
                PINK_DREAM.column_header.disabled_focused_border,
                0xE5D9E0FFu32,
            ),
            (PINK_DREAM.row_header.normal.background, 0xF9DFEDFFu32),
            (PINK_DREAM.row_header.normal.border, 0xF3E3ECFFu32),
            (PINK_DREAM.row_header.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.row_header.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.row_header.hovered.border, 0xF3E3ECFFu32),
            (PINK_DREAM.row_header.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.row_header.selected.background, 0xF5D3E6FFu32),
            (PINK_DREAM.row_header.selected.border, 0xF3E3ECFFu32),
            (PINK_DREAM.row_header.selected.foreground, 0x422B3CFFu32),
            (PINK_DREAM.row_header.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.row_header.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.row_header.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.row_header.focused_border, 0xB4437DFFu32),
            (PINK_DREAM.row_header.disabled_focused_border, 0xE5D9E0FFu32),
            (PINK_DREAM.corner.normal.background, 0xF9DFEDFFu32),
            (PINK_DREAM.corner.normal.border, 0xF3E3ECFFu32),
            (PINK_DREAM.corner.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.corner.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.corner.hovered.border, 0xF3E3ECFFu32),
            (PINK_DREAM.corner.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.corner.selected.background, 0xF5D3E6FFu32),
            (PINK_DREAM.corner.selected.border, 0xF3E3ECFFu32),
            (PINK_DREAM.corner.selected.foreground, 0x422B3CFFu32),
            (PINK_DREAM.corner.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.corner.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.corner.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.corner.focused_border, 0xB4437DFFu32),
            (PINK_DREAM.corner.disabled_focused_border, 0xE5D9E0FFu32),
            (PINK_DREAM.cell.normal.background, 0x00000000u32),
            (PINK_DREAM.cell.normal.border, 0xF3E3ECFFu32),
            (PINK_DREAM.cell.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.cell.hovered.background, 0xF9DFEDFFu32),
            (PINK_DREAM.cell.hovered.border, 0xF3E3ECFFu32),
            (PINK_DREAM.cell.hovered.foreground, 0x422B3CFFu32),
            (PINK_DREAM.cell.selected.background, 0xF5D3E6FFu32),
            (PINK_DREAM.cell.selected.border, 0xF3E3ECFFu32),
            (PINK_DREAM.cell.selected.foreground, 0x422B3CFFu32),
            (PINK_DREAM.cell.disabled.background, 0xF1E1E9FFu32),
            (PINK_DREAM.cell.disabled.border, 0xE5D9E0FFu32),
            (PINK_DREAM.cell.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.cell.focused_border, 0xB4437DFFu32),
            (PINK_DREAM.cell.disabled_focused_border, 0xE5D9E0FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.table.normal.background, 0xF0E8FAFFu32),
            (KAMURI_VIOLET.table.normal.border, 0xD8CDE4FFu32),
            (KAMURI_VIOLET.table.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.table.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.table.hovered.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.table.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.table.selected.background, 0xDFD2F2FFu32),
            (KAMURI_VIOLET.table.selected.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.table.selected.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.table.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.table.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.table.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.table.focused_border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.table.disabled_focused_border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.column_header.normal.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.column_header.normal.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.column_header.normal.foreground, 0x3D314AFFu32),
            (
                KAMURI_VIOLET.column_header.hovered.background,
                0xE8DDF6FFu32,
            ),
            (KAMURI_VIOLET.column_header.hovered.border, 0xEEE7F5FFu32),
            (
                KAMURI_VIOLET.column_header.hovered.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.column_header.selected.background,
                0xDFD2F2FFu32,
            ),
            (KAMURI_VIOLET.column_header.selected.border, 0xEEE7F5FFu32),
            (
                KAMURI_VIOLET.column_header.selected.foreground,
                0x3D314AFFu32,
            ),
            (
                KAMURI_VIOLET.column_header.disabled.background,
                0xE8E0F0FFu32,
            ),
            (KAMURI_VIOLET.column_header.disabled.border, 0xE7DFF0FFu32),
            (
                KAMURI_VIOLET.column_header.disabled.foreground,
                0xB5A9C1FFu32,
            ),
            (KAMURI_VIOLET.column_header.focused_border, 0x8C6BC1FFu32),
            (
                KAMURI_VIOLET.column_header.disabled_focused_border,
                0xE7DFF0FFu32,
            ),
            (KAMURI_VIOLET.row_header.normal.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.row_header.normal.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.row_header.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.row_header.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.row_header.hovered.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.row_header.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.row_header.selected.background, 0xDFD2F2FFu32),
            (KAMURI_VIOLET.row_header.selected.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.row_header.selected.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.row_header.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.row_header.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.row_header.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.row_header.focused_border, 0x8C6BC1FFu32),
            (
                KAMURI_VIOLET.row_header.disabled_focused_border,
                0xE7DFF0FFu32,
            ),
            (KAMURI_VIOLET.corner.normal.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.corner.normal.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.corner.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.corner.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.corner.hovered.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.corner.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.corner.selected.background, 0xDFD2F2FFu32),
            (KAMURI_VIOLET.corner.selected.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.corner.selected.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.corner.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.corner.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.corner.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.corner.focused_border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.corner.disabled_focused_border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.cell.normal.background, 0x00000000u32),
            (KAMURI_VIOLET.cell.normal.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.cell.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.cell.hovered.background, 0xE8DDF6FFu32),
            (KAMURI_VIOLET.cell.hovered.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.cell.hovered.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.cell.selected.background, 0xDFD2F2FFu32),
            (KAMURI_VIOLET.cell.selected.border, 0xEEE7F5FFu32),
            (KAMURI_VIOLET.cell.selected.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.cell.disabled.background, 0xE8E0F0FFu32),
            (KAMURI_VIOLET.cell.disabled.border, 0xE7DFF0FFu32),
            (KAMURI_VIOLET.cell.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.cell.focused_border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.cell.disabled_focused_border, 0xE7DFF0FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.table.normal.background, [255, 255, 255, 255]),
            (LIGHT.table.normal.border, [217, 217, 217, 255]),
            (LIGHT.table.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.table.hovered.background, [245, 245, 245, 255]),
            (LIGHT.table.hovered.border, [240, 240, 240, 255]),
            (LIGHT.table.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.table.selected.background, [230, 244, 255, 255]),
            (LIGHT.table.selected.border, [240, 240, 240, 255]),
            (LIGHT.table.selected.foreground, [31, 31, 31, 255]),
            (LIGHT.table.disabled.background, [245, 245, 245, 255]),
            (LIGHT.table.disabled.border, [217, 217, 217, 255]),
            (LIGHT.table.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.table.focused_border, [22, 119, 255, 255]),
            (LIGHT.table.disabled_focused_border, [217, 217, 217, 255]),
            (LIGHT.column_header.normal.background, [245, 245, 245, 255]),
            (LIGHT.column_header.normal.border, [240, 240, 240, 255]),
            (LIGHT.column_header.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.column_header.hovered.background, [245, 245, 245, 255]),
            (LIGHT.column_header.hovered.border, [240, 240, 240, 255]),
            (LIGHT.column_header.hovered.foreground, [31, 31, 31, 255]),
            (
                LIGHT.column_header.selected.background,
                [230, 244, 255, 255],
            ),
            (LIGHT.column_header.selected.border, [240, 240, 240, 255]),
            (LIGHT.column_header.selected.foreground, [31, 31, 31, 255]),
            (
                LIGHT.column_header.disabled.background,
                [245, 245, 245, 255],
            ),
            (LIGHT.column_header.disabled.border, [217, 217, 217, 255]),
            (
                LIGHT.column_header.disabled.foreground,
                [191, 191, 191, 255],
            ),
            (LIGHT.column_header.focused_border, [22, 119, 255, 255]),
            (
                LIGHT.column_header.disabled_focused_border,
                [217, 217, 217, 255],
            ),
            (LIGHT.row_header.normal.background, [245, 245, 245, 255]),
            (LIGHT.row_header.normal.border, [240, 240, 240, 255]),
            (LIGHT.row_header.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.row_header.hovered.background, [245, 245, 245, 255]),
            (LIGHT.row_header.hovered.border, [240, 240, 240, 255]),
            (LIGHT.row_header.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.row_header.selected.background, [230, 244, 255, 255]),
            (LIGHT.row_header.selected.border, [240, 240, 240, 255]),
            (LIGHT.row_header.selected.foreground, [31, 31, 31, 255]),
            (LIGHT.row_header.disabled.background, [245, 245, 245, 255]),
            (LIGHT.row_header.disabled.border, [217, 217, 217, 255]),
            (LIGHT.row_header.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.row_header.focused_border, [22, 119, 255, 255]),
            (
                LIGHT.row_header.disabled_focused_border,
                [217, 217, 217, 255],
            ),
            (LIGHT.corner.normal.background, [245, 245, 245, 255]),
            (LIGHT.corner.normal.border, [240, 240, 240, 255]),
            (LIGHT.corner.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.corner.hovered.background, [245, 245, 245, 255]),
            (LIGHT.corner.hovered.border, [240, 240, 240, 255]),
            (LIGHT.corner.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.corner.selected.background, [230, 244, 255, 255]),
            (LIGHT.corner.selected.border, [240, 240, 240, 255]),
            (LIGHT.corner.selected.foreground, [31, 31, 31, 255]),
            (LIGHT.corner.disabled.background, [245, 245, 245, 255]),
            (LIGHT.corner.disabled.border, [217, 217, 217, 255]),
            (LIGHT.corner.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.corner.focused_border, [22, 119, 255, 255]),
            (LIGHT.corner.disabled_focused_border, [217, 217, 217, 255]),
            (LIGHT.cell.normal.background, [0, 0, 0, 0]),
            (LIGHT.cell.normal.border, [240, 240, 240, 255]),
            (LIGHT.cell.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.cell.hovered.background, [245, 245, 245, 255]),
            (LIGHT.cell.hovered.border, [240, 240, 240, 255]),
            (LIGHT.cell.hovered.foreground, [31, 31, 31, 255]),
            (LIGHT.cell.selected.background, [230, 244, 255, 255]),
            (LIGHT.cell.selected.border, [240, 240, 240, 255]),
            (LIGHT.cell.selected.foreground, [31, 31, 31, 255]),
            (LIGHT.cell.disabled.background, [245, 245, 245, 255]),
            (LIGHT.cell.disabled.border, [217, 217, 217, 255]),
            (LIGHT.cell.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.cell.focused_border, [22, 119, 255, 255]),
            (LIGHT.cell.disabled_focused_border, [217, 217, 217, 255]),
        ];
        for (color, rgba) in slots {
            let actual = color.to_srgba();
            let expected = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3]).to_srgba();
            assert_eq!(actual, expected);
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
    #[test]
    fn dark_matches_reference() {
        let slots = [
            (DARK.table.normal.background, [20, 20, 20, 255]),
            (DARK.table.normal.border, [66, 66, 66, 255]),
            (DARK.table.normal.foreground, [220, 220, 220, 255]),
            (DARK.table.hovered.background, [38, 38, 38, 255]),
            (DARK.table.hovered.border, [48, 48, 48, 255]),
            (DARK.table.hovered.foreground, [220, 220, 220, 255]),
            (DARK.table.selected.background, [17, 26, 44, 255]),
            (DARK.table.selected.border, [48, 48, 48, 255]),
            (DARK.table.selected.foreground, [220, 220, 220, 255]),
            (DARK.table.disabled.background, [31, 31, 31, 255]),
            (DARK.table.disabled.border, [66, 66, 66, 255]),
            (DARK.table.disabled.foreground, [89, 89, 89, 255]),
            (DARK.table.focused_border, [60, 137, 232, 255]),
            (DARK.table.disabled_focused_border, [66, 66, 66, 255]),
            (DARK.column_header.normal.background, [38, 38, 38, 255]),
            (DARK.column_header.normal.border, [48, 48, 48, 255]),
            (DARK.column_header.normal.foreground, [220, 220, 220, 255]),
            (DARK.column_header.hovered.background, [38, 38, 38, 255]),
            (DARK.column_header.hovered.border, [48, 48, 48, 255]),
            (DARK.column_header.hovered.foreground, [220, 220, 220, 255]),
            (DARK.column_header.selected.background, [17, 26, 44, 255]),
            (DARK.column_header.selected.border, [48, 48, 48, 255]),
            (DARK.column_header.selected.foreground, [220, 220, 220, 255]),
            (DARK.column_header.disabled.background, [31, 31, 31, 255]),
            (DARK.column_header.disabled.border, [66, 66, 66, 255]),
            (DARK.column_header.disabled.foreground, [89, 89, 89, 255]),
            (DARK.column_header.focused_border, [60, 137, 232, 255]),
            (
                DARK.column_header.disabled_focused_border,
                [66, 66, 66, 255],
            ),
            (DARK.row_header.normal.background, [38, 38, 38, 255]),
            (DARK.row_header.normal.border, [48, 48, 48, 255]),
            (DARK.row_header.normal.foreground, [220, 220, 220, 255]),
            (DARK.row_header.hovered.background, [38, 38, 38, 255]),
            (DARK.row_header.hovered.border, [48, 48, 48, 255]),
            (DARK.row_header.hovered.foreground, [220, 220, 220, 255]),
            (DARK.row_header.selected.background, [17, 26, 44, 255]),
            (DARK.row_header.selected.border, [48, 48, 48, 255]),
            (DARK.row_header.selected.foreground, [220, 220, 220, 255]),
            (DARK.row_header.disabled.background, [31, 31, 31, 255]),
            (DARK.row_header.disabled.border, [66, 66, 66, 255]),
            (DARK.row_header.disabled.foreground, [89, 89, 89, 255]),
            (DARK.row_header.focused_border, [60, 137, 232, 255]),
            (DARK.row_header.disabled_focused_border, [66, 66, 66, 255]),
            (DARK.corner.normal.background, [38, 38, 38, 255]),
            (DARK.corner.normal.border, [48, 48, 48, 255]),
            (DARK.corner.normal.foreground, [220, 220, 220, 255]),
            (DARK.corner.hovered.background, [38, 38, 38, 255]),
            (DARK.corner.hovered.border, [48, 48, 48, 255]),
            (DARK.corner.hovered.foreground, [220, 220, 220, 255]),
            (DARK.corner.selected.background, [17, 26, 44, 255]),
            (DARK.corner.selected.border, [48, 48, 48, 255]),
            (DARK.corner.selected.foreground, [220, 220, 220, 255]),
            (DARK.corner.disabled.background, [31, 31, 31, 255]),
            (DARK.corner.disabled.border, [66, 66, 66, 255]),
            (DARK.corner.disabled.foreground, [89, 89, 89, 255]),
            (DARK.corner.focused_border, [60, 137, 232, 255]),
            (DARK.corner.disabled_focused_border, [66, 66, 66, 255]),
            (DARK.cell.normal.background, [0, 0, 0, 0]),
            (DARK.cell.normal.border, [48, 48, 48, 255]),
            (DARK.cell.normal.foreground, [220, 220, 220, 255]),
            (DARK.cell.hovered.background, [38, 38, 38, 255]),
            (DARK.cell.hovered.border, [48, 48, 48, 255]),
            (DARK.cell.hovered.foreground, [220, 220, 220, 255]),
            (DARK.cell.selected.background, [17, 26, 44, 255]),
            (DARK.cell.selected.border, [48, 48, 48, 255]),
            (DARK.cell.selected.foreground, [220, 220, 220, 255]),
            (DARK.cell.disabled.background, [31, 31, 31, 255]),
            (DARK.cell.disabled.border, [66, 66, 66, 255]),
            (DARK.cell.disabled.foreground, [89, 89, 89, 255]),
            (DARK.cell.focused_border, [60, 137, 232, 255]),
            (DARK.cell.disabled_focused_border, [66, 66, 66, 255]),
        ];
        for (color, rgba) in slots {
            let actual = color.to_srgba();
            let expected = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3]).to_srgba();
            assert_eq!(actual, expected);
            assert!(
                [actual.red, actual.green, actual.blue, actual.alpha]
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
}
