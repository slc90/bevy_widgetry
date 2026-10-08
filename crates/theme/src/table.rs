use crate::common::{dark, light};
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
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
