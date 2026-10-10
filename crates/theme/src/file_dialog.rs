use crate::button::WidgetryButtonColors;
use crate::check_box::WidgetryCheckBoxColors;
use crate::combo_box::WidgetryComboBoxColors;
use crate::common::{dark, kamuri_violet, light, pink_dream};
use crate::message_box::WidgetryMessageBoxColors;
use crate::scroll_area::WidgetryScrollAreaColors;
use crate::text_field::WidgetryTextFieldColors;
use crate::window::WidgetryWindowColors;
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryFileDialogBodyStateColors {
    pub background: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryFileDialogBodyColors {
    pub normal: WidgetryFileDialogBodyStateColors,
    pub disabled: WidgetryFileDialogBodyStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryFileDialogEntryStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryFileDialogEntryColors {
    pub normal: WidgetryFileDialogEntryStateColors,
    pub selected: WidgetryFileDialogEntryStateColors,
    pub disabled: WidgetryFileDialogEntryStateColors,
    pub active_border: Color,
    pub disabled_active_border: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryFileDialogStatusStateColors {
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryFileDialogStatusColors {
    pub normal: WidgetryFileDialogStatusStateColors,
    pub disabled: WidgetryFileDialogStatusStateColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryFileDialogColors {
    pub window: WidgetryWindowColors,
    pub body: WidgetryFileDialogBodyColors,
    pub entry: WidgetryFileDialogEntryColors,
    pub status: WidgetryFileDialogStatusColors,
    pub sidebar_button: WidgetryButtonColors,
    pub toolbar_button: WidgetryButtonColors,
    pub accept_button: WidgetryButtonColors,
    pub cancel_button: WidgetryButtonColors,
    pub folder_create_button: WidgetryButtonColors,
    pub folder_cancel_button: WidgetryButtonColors,
    pub overwrite_accept_button: WidgetryButtonColors,
    pub overwrite_cancel_button: WidgetryButtonColors,
    pub path_field: WidgetryTextFieldColors,
    pub search_field: WidgetryTextFieldColors,
    pub filename_field: WidgetryTextFieldColors,
    pub folder_name_field: WidgetryTextFieldColors,
    pub filter: WidgetryComboBoxColors,
    pub sort: WidgetryComboBoxColors,
    pub hidden_option: WidgetryCheckBoxColors,
    pub system_option: WidgetryCheckBoxColors,
    pub sidebar_scroll: WidgetryScrollAreaColors,
    pub entries_scroll: WidgetryScrollAreaColors,
    pub confirmation: WidgetryMessageBoxColors,
}

pub const LIGHT: WidgetryFileDialogColors = WidgetryFileDialogColors {
    window: crate::window::LIGHT,
    body: WidgetryFileDialogBodyColors {
        normal: WidgetryFileDialogBodyStateColors {
            background: light::WINDOW_BACKGROUND,
            foreground: light::TEXT,
        },
        disabled: WidgetryFileDialogBodyStateColors {
            background: light::WINDOW_BACKGROUND,
            foreground: light::DISABLED_TEXT,
        },
    },
    entry: WidgetryFileDialogEntryColors {
        normal: WidgetryFileDialogEntryStateColors {
            background: light::TRANSPARENT,
            border: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        selected: WidgetryFileDialogEntryStateColors {
            background: light::SELECTED_SURFACE,
            border: light::TRANSPARENT,
            foreground: light::TEXT,
        },
        disabled: WidgetryFileDialogEntryStateColors {
            background: light::TRANSPARENT,
            border: light::TRANSPARENT,
            foreground: light::DISABLED_TEXT,
        },
        active_border: light::FOCUS_BORDER,
        disabled_active_border: light::TRANSPARENT,
    },
    status: WidgetryFileDialogStatusColors {
        normal: WidgetryFileDialogStatusStateColors {
            foreground: light::SECONDARY_TEXT,
        },
        disabled: WidgetryFileDialogStatusStateColors {
            foreground: light::DISABLED_TEXT,
        },
    },
    sidebar_button: crate::button::LIGHT,
    toolbar_button: crate::button::LIGHT,
    accept_button: crate::button::LIGHT,
    cancel_button: crate::button::LIGHT,
    folder_create_button: crate::button::LIGHT,
    folder_cancel_button: crate::button::LIGHT,
    overwrite_accept_button: crate::button::LIGHT,
    overwrite_cancel_button: crate::button::LIGHT,
    path_field: crate::text_field::LIGHT,
    search_field: crate::text_field::LIGHT,
    filename_field: crate::text_field::LIGHT,
    folder_name_field: crate::text_field::LIGHT,
    filter: crate::combo_box::LIGHT,
    sort: crate::combo_box::LIGHT,
    hidden_option: crate::check_box::LIGHT,
    system_option: crate::check_box::LIGHT,
    sidebar_scroll: crate::scroll_area::LIGHT,
    entries_scroll: crate::scroll_area::LIGHT,
    confirmation: crate::message_box::LIGHT,
};

pub const DARK: WidgetryFileDialogColors = WidgetryFileDialogColors {
    window: crate::window::DARK,
    body: WidgetryFileDialogBodyColors {
        normal: WidgetryFileDialogBodyStateColors {
            background: dark::WINDOW_BACKGROUND,
            foreground: dark::TEXT,
        },
        disabled: WidgetryFileDialogBodyStateColors {
            background: dark::WINDOW_BACKGROUND,
            foreground: dark::DISABLED_TEXT,
        },
    },
    entry: WidgetryFileDialogEntryColors {
        normal: WidgetryFileDialogEntryStateColors {
            background: dark::TRANSPARENT,
            border: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        selected: WidgetryFileDialogEntryStateColors {
            background: dark::SELECTED_SURFACE,
            border: dark::TRANSPARENT,
            foreground: dark::TEXT,
        },
        disabled: WidgetryFileDialogEntryStateColors {
            background: dark::TRANSPARENT,
            border: dark::TRANSPARENT,
            foreground: dark::DISABLED_TEXT,
        },
        active_border: dark::FOCUS_BORDER,
        disabled_active_border: dark::TRANSPARENT,
    },
    status: WidgetryFileDialogStatusColors {
        normal: WidgetryFileDialogStatusStateColors {
            foreground: dark::SECONDARY_TEXT,
        },
        disabled: WidgetryFileDialogStatusStateColors {
            foreground: dark::DISABLED_TEXT,
        },
    },
    sidebar_button: crate::button::DARK,
    toolbar_button: crate::button::DARK,
    accept_button: crate::button::DARK,
    cancel_button: crate::button::DARK,
    folder_create_button: crate::button::DARK,
    folder_cancel_button: crate::button::DARK,
    overwrite_accept_button: crate::button::DARK,
    overwrite_cancel_button: crate::button::DARK,
    path_field: crate::text_field::DARK,
    search_field: crate::text_field::DARK,
    filename_field: crate::text_field::DARK,
    folder_name_field: crate::text_field::DARK,
    filter: crate::combo_box::DARK,
    sort: crate::combo_box::DARK,
    hidden_option: crate::check_box::DARK,
    system_option: crate::check_box::DARK,
    sidebar_scroll: crate::scroll_area::DARK,
    entries_scroll: crate::scroll_area::DARK,
    confirmation: crate::message_box::DARK,
};

pub const PINK_DREAM: WidgetryFileDialogColors = WidgetryFileDialogColors {
    window: crate::window::PINK_DREAM,
    body: WidgetryFileDialogBodyColors {
        normal: WidgetryFileDialogBodyStateColors {
            background: pink_dream::WINDOW_BACKGROUND,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryFileDialogBodyStateColors {
            background: pink_dream::WINDOW_BACKGROUND,
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    entry: WidgetryFileDialogEntryColors {
        normal: WidgetryFileDialogEntryStateColors {
            background: pink_dream::TRANSPARENT,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        selected: WidgetryFileDialogEntryStateColors {
            background: pink_dream::SELECTED_SURFACE,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::TEXT,
        },
        disabled: WidgetryFileDialogEntryStateColors {
            background: pink_dream::TRANSPARENT,
            border: pink_dream::TRANSPARENT,
            foreground: pink_dream::DISABLED_TEXT,
        },
        active_border: pink_dream::FOCUS_BORDER,
        disabled_active_border: pink_dream::TRANSPARENT,
    },
    status: WidgetryFileDialogStatusColors {
        normal: WidgetryFileDialogStatusStateColors {
            foreground: pink_dream::SECONDARY_TEXT,
        },
        disabled: WidgetryFileDialogStatusStateColors {
            foreground: pink_dream::DISABLED_TEXT,
        },
    },
    sidebar_button: crate::button::PINK_DREAM,
    toolbar_button: crate::button::PINK_DREAM,
    accept_button: crate::button::PINK_DREAM,
    cancel_button: crate::button::PINK_DREAM,
    folder_create_button: crate::button::PINK_DREAM,
    folder_cancel_button: crate::button::PINK_DREAM,
    overwrite_accept_button: crate::button::PINK_DREAM,
    overwrite_cancel_button: crate::button::PINK_DREAM,
    path_field: crate::text_field::PINK_DREAM,
    search_field: crate::text_field::PINK_DREAM,
    filename_field: crate::text_field::PINK_DREAM,
    folder_name_field: crate::text_field::PINK_DREAM,
    filter: crate::combo_box::PINK_DREAM,
    sort: crate::combo_box::PINK_DREAM,
    hidden_option: crate::check_box::PINK_DREAM,
    system_option: crate::check_box::PINK_DREAM,
    sidebar_scroll: crate::scroll_area::PINK_DREAM,
    entries_scroll: crate::scroll_area::PINK_DREAM,
    confirmation: crate::message_box::PINK_DREAM,
};

pub const KAMURI_VIOLET: WidgetryFileDialogColors = WidgetryFileDialogColors {
    window: crate::window::KAMURI_VIOLET,
    body: WidgetryFileDialogBodyColors {
        normal: WidgetryFileDialogBodyStateColors {
            background: kamuri_violet::WINDOW_BACKGROUND,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryFileDialogBodyStateColors {
            background: kamuri_violet::WINDOW_BACKGROUND,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    entry: WidgetryFileDialogEntryColors {
        normal: WidgetryFileDialogEntryStateColors {
            background: kamuri_violet::TRANSPARENT,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        selected: WidgetryFileDialogEntryStateColors {
            background: kamuri_violet::SELECTED_SURFACE,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryFileDialogEntryStateColors {
            background: kamuri_violet::TRANSPARENT,
            border: kamuri_violet::TRANSPARENT,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
        active_border: kamuri_violet::FOCUS_BORDER,
        disabled_active_border: kamuri_violet::TRANSPARENT,
    },
    status: WidgetryFileDialogStatusColors {
        normal: WidgetryFileDialogStatusStateColors {
            foreground: kamuri_violet::SECONDARY_TEXT,
        },
        disabled: WidgetryFileDialogStatusStateColors {
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    },
    sidebar_button: crate::button::KAMURI_VIOLET,
    toolbar_button: crate::button::KAMURI_VIOLET,
    accept_button: crate::button::KAMURI_VIOLET,
    cancel_button: crate::button::KAMURI_VIOLET,
    folder_create_button: crate::button::KAMURI_VIOLET,
    folder_cancel_button: crate::button::KAMURI_VIOLET,
    overwrite_accept_button: crate::button::KAMURI_VIOLET,
    overwrite_cancel_button: crate::button::KAMURI_VIOLET,
    path_field: crate::text_field::KAMURI_VIOLET,
    search_field: crate::text_field::KAMURI_VIOLET,
    filename_field: crate::text_field::KAMURI_VIOLET,
    folder_name_field: crate::text_field::KAMURI_VIOLET,
    filter: crate::combo_box::KAMURI_VIOLET,
    sort: crate::combo_box::KAMURI_VIOLET,
    hidden_option: crate::check_box::KAMURI_VIOLET,
    system_option: crate::check_box::KAMURI_VIOLET,
    sidebar_scroll: crate::scroll_area::KAMURI_VIOLET,
    entries_scroll: crate::scroll_area::KAMURI_VIOLET,
    confirmation: crate::message_box::KAMURI_VIOLET,
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
            (PINK_DREAM.body.normal.background, 0xF6DCE9FFu32),
            (PINK_DREAM.body.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.body.disabled.background, 0xF6DCE9FFu32),
            (PINK_DREAM.body.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.entry.normal.background, 0x00000000u32),
            (PINK_DREAM.entry.normal.border, 0x00000000u32),
            (PINK_DREAM.entry.normal.foreground, 0x422B3CFFu32),
            (PINK_DREAM.entry.selected.background, 0xF5D3E6FFu32),
            (PINK_DREAM.entry.selected.border, 0x00000000u32),
            (PINK_DREAM.entry.selected.foreground, 0x422B3CFFu32),
            (PINK_DREAM.entry.disabled.background, 0x00000000u32),
            (PINK_DREAM.entry.disabled.border, 0x00000000u32),
            (PINK_DREAM.entry.disabled.foreground, 0xAC98A3FFu32),
            (PINK_DREAM.entry.active_border, 0xB4437DFFu32),
            (PINK_DREAM.entry.disabled_active_border, 0x00000000u32),
            (PINK_DREAM.status.normal.foreground, 0x705566FFu32),
            (PINK_DREAM.status.disabled.foreground, 0xAC98A3FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(PINK_DREAM.window, crate::window::PINK_DREAM);
        assert_eq!(PINK_DREAM.sidebar_button, crate::button::PINK_DREAM);
        assert_eq!(PINK_DREAM.toolbar_button, crate::button::PINK_DREAM);
        assert_eq!(PINK_DREAM.accept_button, crate::button::PINK_DREAM);
        assert_eq!(PINK_DREAM.cancel_button, crate::button::PINK_DREAM);
        assert_eq!(PINK_DREAM.folder_create_button, crate::button::PINK_DREAM);
        assert_eq!(PINK_DREAM.folder_cancel_button, crate::button::PINK_DREAM);
        assert_eq!(
            PINK_DREAM.overwrite_accept_button,
            crate::button::PINK_DREAM
        );
        assert_eq!(
            PINK_DREAM.overwrite_cancel_button,
            crate::button::PINK_DREAM
        );
        assert_eq!(PINK_DREAM.path_field, crate::text_field::PINK_DREAM);
        assert_eq!(PINK_DREAM.search_field, crate::text_field::PINK_DREAM);
        assert_eq!(PINK_DREAM.filename_field, crate::text_field::PINK_DREAM);
        assert_eq!(PINK_DREAM.folder_name_field, crate::text_field::PINK_DREAM);
        assert_eq!(PINK_DREAM.filter, crate::combo_box::PINK_DREAM);
        assert_eq!(PINK_DREAM.sort, crate::combo_box::PINK_DREAM);
        assert_eq!(PINK_DREAM.hidden_option, crate::check_box::PINK_DREAM);
        assert_eq!(PINK_DREAM.system_option, crate::check_box::PINK_DREAM);
        assert_eq!(PINK_DREAM.sidebar_scroll, crate::scroll_area::PINK_DREAM);
        assert_eq!(PINK_DREAM.entries_scroll, crate::scroll_area::PINK_DREAM);
        assert_eq!(PINK_DREAM.confirmation, crate::message_box::PINK_DREAM);
    }

    #[test]
    fn kamuri_violet_matches_approved_palette() {
        let slots = [
            (KAMURI_VIOLET.body.normal.background, 0xE3D9F1FFu32),
            (KAMURI_VIOLET.body.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.body.disabled.background, 0xE3D9F1FFu32),
            (KAMURI_VIOLET.body.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.entry.normal.background, 0x00000000u32),
            (KAMURI_VIOLET.entry.normal.border, 0x00000000u32),
            (KAMURI_VIOLET.entry.normal.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.entry.selected.background, 0xDFD2F2FFu32),
            (KAMURI_VIOLET.entry.selected.border, 0x00000000u32),
            (KAMURI_VIOLET.entry.selected.foreground, 0x3D314AFFu32),
            (KAMURI_VIOLET.entry.disabled.background, 0x00000000u32),
            (KAMURI_VIOLET.entry.disabled.border, 0x00000000u32),
            (KAMURI_VIOLET.entry.disabled.foreground, 0xB5A9C1FFu32),
            (KAMURI_VIOLET.entry.active_border, 0x8C6BC1FFu32),
            (KAMURI_VIOLET.entry.disabled_active_border, 0x00000000u32),
            (KAMURI_VIOLET.status.normal.foreground, 0x72617FFFu32),
            (KAMURI_VIOLET.status.disabled.foreground, 0xB5A9C1FFu32),
        ];
        for (actual, expected) in slots {
            assert_eq!(actual.to_srgba().to_u8_array(), expected.to_be_bytes());
        }
        assert_eq!(KAMURI_VIOLET.window, crate::window::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.sidebar_button, crate::button::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.toolbar_button, crate::button::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.accept_button, crate::button::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.cancel_button, crate::button::KAMURI_VIOLET);
        assert_eq!(
            KAMURI_VIOLET.folder_create_button,
            crate::button::KAMURI_VIOLET
        );
        assert_eq!(
            KAMURI_VIOLET.folder_cancel_button,
            crate::button::KAMURI_VIOLET
        );
        assert_eq!(
            KAMURI_VIOLET.overwrite_accept_button,
            crate::button::KAMURI_VIOLET
        );
        assert_eq!(
            KAMURI_VIOLET.overwrite_cancel_button,
            crate::button::KAMURI_VIOLET
        );
        assert_eq!(KAMURI_VIOLET.path_field, crate::text_field::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.search_field, crate::text_field::KAMURI_VIOLET);
        assert_eq!(
            KAMURI_VIOLET.filename_field,
            crate::text_field::KAMURI_VIOLET
        );
        assert_eq!(
            KAMURI_VIOLET.folder_name_field,
            crate::text_field::KAMURI_VIOLET
        );
        assert_eq!(KAMURI_VIOLET.filter, crate::combo_box::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.sort, crate::combo_box::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.hidden_option, crate::check_box::KAMURI_VIOLET);
        assert_eq!(KAMURI_VIOLET.system_option, crate::check_box::KAMURI_VIOLET);
        assert_eq!(
            KAMURI_VIOLET.sidebar_scroll,
            crate::scroll_area::KAMURI_VIOLET
        );
        assert_eq!(
            KAMURI_VIOLET.entries_scroll,
            crate::scroll_area::KAMURI_VIOLET
        );
        assert_eq!(
            KAMURI_VIOLET.confirmation,
            crate::message_box::KAMURI_VIOLET
        );
    }

    #[test]
    fn light_matches_reference() {
        let slots = [
            (LIGHT.body.normal.background, [255, 255, 255, 255]),
            (LIGHT.body.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.body.disabled.background, [255, 255, 255, 255]),
            (LIGHT.body.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.entry.normal.background, [0, 0, 0, 0]),
            (LIGHT.entry.normal.border, [0, 0, 0, 0]),
            (LIGHT.entry.normal.foreground, [31, 31, 31, 255]),
            (LIGHT.entry.selected.background, [230, 244, 255, 255]),
            (LIGHT.entry.selected.border, [0, 0, 0, 0]),
            (LIGHT.entry.selected.foreground, [31, 31, 31, 255]),
            (LIGHT.entry.disabled.background, [0, 0, 0, 0]),
            (LIGHT.entry.disabled.border, [0, 0, 0, 0]),
            (LIGHT.entry.disabled.foreground, [191, 191, 191, 255]),
            (LIGHT.entry.active_border, [22, 119, 255, 255]),
            (LIGHT.entry.disabled_active_border, [0, 0, 0, 0]),
            (LIGHT.status.normal.foreground, [89, 89, 89, 255]),
            (LIGHT.status.disabled.foreground, [191, 191, 191, 255]),
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
        assert_eq!(LIGHT.window, crate::window::LIGHT);
        assert_eq!(LIGHT.sidebar_button, crate::button::LIGHT);
        assert_eq!(LIGHT.toolbar_button, crate::button::LIGHT);
        assert_eq!(LIGHT.accept_button, crate::button::LIGHT);
        assert_eq!(LIGHT.cancel_button, crate::button::LIGHT);
        assert_eq!(LIGHT.folder_create_button, crate::button::LIGHT);
        assert_eq!(LIGHT.folder_cancel_button, crate::button::LIGHT);
        assert_eq!(LIGHT.overwrite_accept_button, crate::button::LIGHT);
        assert_eq!(LIGHT.overwrite_cancel_button, crate::button::LIGHT);
        assert_eq!(LIGHT.path_field, crate::text_field::LIGHT);
        assert_eq!(LIGHT.search_field, crate::text_field::LIGHT);
        assert_eq!(LIGHT.filename_field, crate::text_field::LIGHT);
        assert_eq!(LIGHT.folder_name_field, crate::text_field::LIGHT);
        assert_eq!(LIGHT.filter, crate::combo_box::LIGHT);
        assert_eq!(LIGHT.sort, crate::combo_box::LIGHT);
        assert_eq!(LIGHT.hidden_option, crate::check_box::LIGHT);
        assert_eq!(LIGHT.system_option, crate::check_box::LIGHT);
        assert_eq!(LIGHT.sidebar_scroll, crate::scroll_area::LIGHT);
        assert_eq!(LIGHT.entries_scroll, crate::scroll_area::LIGHT);
        assert_eq!(LIGHT.confirmation, crate::message_box::LIGHT);
    }
    #[test]
    fn dark_matches_reference() {
        let slots = [
            (DARK.body.normal.background, [20, 20, 20, 255]),
            (DARK.body.normal.foreground, [220, 220, 220, 255]),
            (DARK.body.disabled.background, [20, 20, 20, 255]),
            (DARK.body.disabled.foreground, [89, 89, 89, 255]),
            (DARK.entry.normal.background, [0, 0, 0, 0]),
            (DARK.entry.normal.border, [0, 0, 0, 0]),
            (DARK.entry.normal.foreground, [220, 220, 220, 255]),
            (DARK.entry.selected.background, [17, 26, 44, 255]),
            (DARK.entry.selected.border, [0, 0, 0, 0]),
            (DARK.entry.selected.foreground, [220, 220, 220, 255]),
            (DARK.entry.disabled.background, [0, 0, 0, 0]),
            (DARK.entry.disabled.border, [0, 0, 0, 0]),
            (DARK.entry.disabled.foreground, [89, 89, 89, 255]),
            (DARK.entry.active_border, [60, 137, 232, 255]),
            (DARK.entry.disabled_active_border, [0, 0, 0, 0]),
            (DARK.status.normal.foreground, [173, 173, 173, 255]),
            (DARK.status.disabled.foreground, [89, 89, 89, 255]),
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
        assert_eq!(DARK.window, crate::window::DARK);
        assert_eq!(DARK.sidebar_button, crate::button::DARK);
        assert_eq!(DARK.toolbar_button, crate::button::DARK);
        assert_eq!(DARK.accept_button, crate::button::DARK);
        assert_eq!(DARK.cancel_button, crate::button::DARK);
        assert_eq!(DARK.folder_create_button, crate::button::DARK);
        assert_eq!(DARK.folder_cancel_button, crate::button::DARK);
        assert_eq!(DARK.overwrite_accept_button, crate::button::DARK);
        assert_eq!(DARK.overwrite_cancel_button, crate::button::DARK);
        assert_eq!(DARK.path_field, crate::text_field::DARK);
        assert_eq!(DARK.search_field, crate::text_field::DARK);
        assert_eq!(DARK.filename_field, crate::text_field::DARK);
        assert_eq!(DARK.folder_name_field, crate::text_field::DARK);
        assert_eq!(DARK.filter, crate::combo_box::DARK);
        assert_eq!(DARK.sort, crate::combo_box::DARK);
        assert_eq!(DARK.hidden_option, crate::check_box::DARK);
        assert_eq!(DARK.system_option, crate::check_box::DARK);
        assert_eq!(DARK.sidebar_scroll, crate::scroll_area::DARK);
        assert_eq!(DARK.entries_scroll, crate::scroll_area::DARK);
        assert_eq!(DARK.confirmation, crate::message_box::DARK);
    }
}
