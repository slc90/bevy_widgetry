use bevy_widgetry::theme::WidgetryTheme;

mod palette {

    use bevy::prelude::Color;
    pub(super) const WINDOW_BACKGROUND: Color = Color::srgb_u8(227, 217, 241);
    pub(super) const SURFACE: Color = Color::srgb_u8(240, 232, 250);
    pub(super) const ELEVATED_SURFACE: Color = Color::srgb_u8(247, 242, 253);
    pub(super) const HOVER_SURFACE: Color = Color::srgb_u8(232, 221, 246);
    pub(super) const PRESSED_SURFACE: Color = Color::srgb_u8(219, 203, 236);
    pub(super) const DISABLED_SURFACE: Color = Color::srgb_u8(232, 224, 240);
    pub(super) const BORDER: Color = Color::srgb_u8(216, 205, 228);
    pub(super) const SUBTLE_BORDER: Color = Color::srgb_u8(238, 231, 245);
    pub(super) const DISABLED_BORDER: Color = Color::srgb_u8(231, 223, 240);
    pub(super) const TEXT: Color = Color::srgb_u8(61, 49, 74);
    pub(super) const SECONDARY_TEXT: Color = Color::srgb_u8(106, 89, 119);
    pub(super) const TERTIARY_TEXT: Color = Color::srgb_u8(154, 140, 166);
    pub(super) const DISABLED_TEXT: Color = Color::srgb_u8(181, 169, 193);
    pub(super) const INVERSE_TEXT: Color = Color::srgb_u8(255, 255, 255);
    pub(super) const PRIMARY: Color = Color::srgb_u8(140, 107, 193);
    pub(super) const PRIMARY_HOVER: Color = Color::srgb_u8(161, 129, 210);
    pub(super) const PRIMARY_PRESSED: Color = Color::srgb_u8(110, 78, 163);
    pub(super) const FOCUS_BORDER: Color = Color::srgb_u8(140, 107, 193);
    pub(super) const SELECTED_SURFACE: Color = Color::srgb_u8(223, 210, 242);
    pub(super) const SELECTION_BACKGROUND: Color = Color::srgb_u8(217, 202, 239);
    pub(super) const UNFOCUSED_SELECTION_BACKGROUND: Color = Color::srgb_u8(232, 224, 240);
    pub(super) const TOOLTIP_BACKGROUND: Color = Color::srgb_u8(74, 58, 95);
    pub(super) const DANGER_HOVER: Color = Color::srgb_u8(155, 92, 134);
    pub(super) const DANGER_PRESSED: Color = Color::srgb_u8(122, 69, 107);
    pub(super) const TRANSPARENT: Color = Color::NONE;
    pub(super) const IMAGE_TINT: Color = Color::srgb_u8(255, 255, 255);
}

mod button {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{WidgetryButtonColors, WidgetryButtonStateColors};

    pub(super) const KAMURI_VIOLET: WidgetryButtonColors = WidgetryButtonColors {
        normal: WidgetryButtonStateColors {
            background: kamuri_violet::SURFACE,
            border: kamuri_violet::BORDER,
            foreground: kamuri_violet::TEXT,
        },
        hovered: WidgetryButtonStateColors {
            background: kamuri_violet::HOVER_SURFACE,
            border: kamuri_violet::PRIMARY_HOVER,
            foreground: kamuri_violet::TEXT,
        },
        pressed: WidgetryButtonStateColors {
            background: kamuri_violet::PRESSED_SURFACE,
            border: kamuri_violet::PRIMARY_PRESSED,
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryButtonStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            border: kamuri_violet::DISABLED_BORDER,
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    };
}

mod check_box {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryCheckBoxColors, WidgetryCheckBoxInteractionColors, WidgetryCheckBoxStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryCheckBoxColors = WidgetryCheckBoxColors {
        unchecked: WidgetryCheckBoxInteractionColors {
            normal: WidgetryCheckBoxStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::TRANSPARENT,
            },
            hovered: WidgetryCheckBoxStateColors {
                background: kamuri_violet::HOVER_SURFACE,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::TRANSPARENT,
            },
            pressed: WidgetryCheckBoxStateColors {
                background: kamuri_violet::PRESSED_SURFACE,
                border: kamuri_violet::PRIMARY_PRESSED,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::TRANSPARENT,
            },
            disabled: WidgetryCheckBoxStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                mark: kamuri_violet::TRANSPARENT,
            },
        },
        checked: WidgetryCheckBoxInteractionColors {
            normal: WidgetryCheckBoxStateColors {
                background: kamuri_violet::PRIMARY,
                border: kamuri_violet::PRIMARY,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::INVERSE_TEXT,
            },
            hovered: WidgetryCheckBoxStateColors {
                background: kamuri_violet::PRIMARY_HOVER,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::INVERSE_TEXT,
            },
            pressed: WidgetryCheckBoxStateColors {
                background: kamuri_violet::PRIMARY_PRESSED,
                border: kamuri_violet::PRIMARY_PRESSED,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::INVERSE_TEXT,
            },
            disabled: WidgetryCheckBoxStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                mark: kamuri_violet::DISABLED_TEXT,
            },
        },
        indeterminate: WidgetryCheckBoxInteractionColors {
            normal: WidgetryCheckBoxStateColors {
                background: kamuri_violet::PRIMARY,
                border: kamuri_violet::PRIMARY,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::INVERSE_TEXT,
            },
            hovered: WidgetryCheckBoxStateColors {
                background: kamuri_violet::PRIMARY_HOVER,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::INVERSE_TEXT,
            },
            pressed: WidgetryCheckBoxStateColors {
                background: kamuri_violet::PRIMARY_PRESSED,
                border: kamuri_violet::PRIMARY_PRESSED,
                foreground: kamuri_violet::TEXT,
                mark: kamuri_violet::INVERSE_TEXT,
            },
            disabled: WidgetryCheckBoxStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                mark: kamuri_violet::DISABLED_TEXT,
            },
        },
    };
}

mod collapsible {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::WidgetryCollapsibleColors;

    macro_rules! palette {
        ($name:ident, $base:ident) => {
            pub(super) const $name: WidgetryCollapsibleColors = WidgetryCollapsibleColors {
                background: $base::SURFACE,
                header_background: $base::SURFACE,
                content_background: $base::SURFACE,
                border: $base::BORDER,
                separator: $base::BORDER,
            };
        };
    }

    palette!(KAMURI_VIOLET, kamuri_violet);
}

mod combo_box {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryComboBoxColors, WidgetryComboBoxFieldColors, WidgetryComboBoxFieldStateColors,
        WidgetryComboBoxPopupColors, WidgetryComboBoxPopupStateColors, WidgetryListViewColors,
        WidgetryListViewContainerColors, WidgetryListViewContainerStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryComboBoxColors = WidgetryComboBoxColors {
        field: WidgetryComboBoxFieldColors {
            normal: WidgetryComboBoxFieldStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
                indicator_foreground: kamuri_violet::TERTIARY_TEXT,
            },
            hovered: WidgetryComboBoxFieldStateColors {
                background: kamuri_violet::HOVER_SURFACE,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                indicator_foreground: kamuri_violet::TERTIARY_TEXT,
            },
            pressed: WidgetryComboBoxFieldStateColors {
                background: kamuri_violet::PRESSED_SURFACE,
                border: kamuri_violet::PRIMARY_PRESSED,
                foreground: kamuri_violet::TEXT,
                indicator_foreground: kamuri_violet::TERTIARY_TEXT,
            },
            open: WidgetryComboBoxFieldStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::FOCUS_BORDER,
                foreground: kamuri_violet::TEXT,
                indicator_foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryComboBoxFieldStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                indicator_foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        popup: WidgetryComboBoxPopupColors {
            normal: WidgetryComboBoxPopupStateColors {
                background: kamuri_violet::ELEVATED_SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryComboBoxPopupStateColors {
                background: kamuri_violet::ELEVATED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        list: WidgetryListViewColors {
            container: WidgetryListViewContainerColors {
                normal: WidgetryListViewContainerStateColors {
                    background: kamuri_violet::ELEVATED_SURFACE,
                    border: kamuri_violet::BORDER,
                    foreground: kamuri_violet::TEXT,
                },
                focused: WidgetryListViewContainerStateColors {
                    background: kamuri_violet::ELEVATED_SURFACE,
                    border: kamuri_violet::FOCUS_BORDER,
                    foreground: kamuri_violet::TEXT,
                },
                disabled: WidgetryListViewContainerStateColors {
                    background: kamuri_violet::ELEVATED_SURFACE,
                    border: kamuri_violet::DISABLED_BORDER,
                    foreground: kamuri_violet::DISABLED_TEXT,
                },
            },
            item: super::list_view::KAMURI_VIOLET.item,
        },
    };
}

mod date_picker {
    use super::palette as kamuri_violet;
    use bevy::prelude::Color;
    use bevy_widgetry::theme::WidgetryDatePickerColors;

    macro_rules! palette {
        ($name:ident, $base:ident, $selected:expr, $hovered:expr, $pressed:expr) => {
            pub(super) const $name: WidgetryDatePickerColors = WidgetryDatePickerColors {
                background: $base::SURFACE,
                border: $base::BORDER,
                foreground: $base::TEXT,
                adjacent: $base::TERTIARY_TEXT,
                disabled: $base::DISABLED_TEXT,
                selected: $selected,
                hovered: $hovered,
                pressed: $pressed,
            };
        };
    }

    palette!(
        KAMURI_VIOLET,
        kamuri_violet,
        Color::srgb_u8(224, 224, 224),
        Color::srgb_u8(245, 245, 245),
        Color::srgb_u8(235, 235, 235)
    );
}

mod file_dialog {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryFileDialogBodyColors, WidgetryFileDialogBodyStateColors, WidgetryFileDialogColors,
        WidgetryFileDialogEntryColors, WidgetryFileDialogEntryStateColors,
        WidgetryFileDialogStatusColors, WidgetryFileDialogStatusStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryFileDialogColors = WidgetryFileDialogColors {
        window: super::window::KAMURI_VIOLET,
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
        sidebar_button: super::button::KAMURI_VIOLET,
        toolbar_button: super::button::KAMURI_VIOLET,
        accept_button: super::button::KAMURI_VIOLET,
        cancel_button: super::button::KAMURI_VIOLET,
        folder_create_button: super::button::KAMURI_VIOLET,
        folder_cancel_button: super::button::KAMURI_VIOLET,
        overwrite_accept_button: super::button::KAMURI_VIOLET,
        overwrite_cancel_button: super::button::KAMURI_VIOLET,
        path_field: super::text_field::KAMURI_VIOLET,
        search_field: super::text_field::KAMURI_VIOLET,
        filename_field: super::text_field::KAMURI_VIOLET,
        folder_name_field: super::text_field::KAMURI_VIOLET,
        filter: super::combo_box::KAMURI_VIOLET,
        sort: super::combo_box::KAMURI_VIOLET,
        hidden_option: super::check_box::KAMURI_VIOLET,
        system_option: super::check_box::KAMURI_VIOLET,
        sidebar_scroll: super::scroll_area::KAMURI_VIOLET,
        entries_scroll: super::scroll_area::KAMURI_VIOLET,
        confirmation: super::message_box::KAMURI_VIOLET,
    };
}

mod hold_button {
    use bevy::prelude::Color;
    use bevy_widgetry::theme::{WidgetryHoldButtonColors, WidgetryHoldButtonStateColors};

    pub(super) const KAMURI_VIOLET: WidgetryHoldButtonColors = WidgetryHoldButtonColors {
        normal: WidgetryHoldButtonStateColors {
            background: Color::srgb_u8(240, 232, 250),
            liquid: Color::srgb_u8(140, 110, 179),
            wave: Color::srgb_u8(180, 155, 210),
            border: Color::srgb_u8(174, 148, 201),
            foreground: Color::srgb_u8(48, 31, 66),
        },
        hovered: WidgetryHoldButtonStateColors {
            background: Color::srgb_u8(247, 239, 255),
            liquid: Color::srgb_u8(140, 110, 179),
            wave: Color::srgb_u8(180, 155, 210),
            border: Color::srgb_u8(181, 155, 208),
            foreground: Color::srgb_u8(48, 31, 66),
        },
        pressed: WidgetryHoldButtonStateColors {
            background: Color::srgb_u8(232, 224, 242),
            liquid: Color::srgb_u8(140, 110, 179),
            wave: Color::srgb_u8(180, 155, 210),
            border: Color::srgb_u8(166, 140, 193),
            foreground: Color::srgb_u8(48, 31, 66),
        },
        disabled: WidgetryHoldButtonStateColors {
            background: Color::srgb_u8(240, 232, 250),
            liquid: Color::srgb_u8(190, 171, 214),
            wave: Color::srgb_u8(210, 194, 230),
            border: Color::srgb_u8(207, 190, 226),
            foreground: Color::srgb_u8(144, 132, 158),
        },
        success_border: Color::srgb_u8(211, 183, 252),
    };
}

mod icon {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{WidgetryIconColors, WidgetryIconStateColors};

    pub(super) const KAMURI_VIOLET: WidgetryIconColors = WidgetryIconColors {
        normal: WidgetryIconStateColors {
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryIconStateColors {
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    };
}

mod link_button {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::WidgetryLinkButtonColors;

    pub(super) const KAMURI_VIOLET: WidgetryLinkButtonColors = WidgetryLinkButtonColors {
        normal: kamuri_violet::PRIMARY,
        hovered: kamuri_violet::PRIMARY_HOVER,
        pressed: kamuri_violet::PRIMARY_PRESSED,
        disabled: kamuri_violet::DISABLED_TEXT,
    };
}

mod list_view {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryListViewColors, WidgetryListViewContainerColors,
        WidgetryListViewContainerStateColors, WidgetryListViewItemColors,
        WidgetryListViewItemStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryListViewColors = WidgetryListViewColors {
        container: WidgetryListViewContainerColors {
            normal: WidgetryListViewContainerStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
            },
            focused: WidgetryListViewContainerStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::FOCUS_BORDER,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryListViewContainerStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        item: WidgetryListViewItemColors {
            normal: WidgetryListViewItemStateColors {
                background: kamuri_violet::TRANSPARENT,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            hovered: WidgetryListViewItemStateColors {
                background: kamuri_violet::HOVER_SURFACE,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            pressed: WidgetryListViewItemStateColors {
                background: kamuri_violet::PRESSED_SURFACE,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            selected: WidgetryListViewItemStateColors {
                background: kamuri_violet::SELECTED_SURFACE,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryListViewItemStateColors {
                background: kamuri_violet::TRANSPARENT,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
            active_border: kamuri_violet::FOCUS_BORDER,
            disabled_active_border: kamuri_violet::TRANSPARENT,
        },
    };
}

mod message_box {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryMessageBoxBodyColors, WidgetryMessageBoxBodyStateColors, WidgetryMessageBoxColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryMessageBoxColors = WidgetryMessageBoxColors {
        window: super::window::KAMURI_VIOLET,
        body: WidgetryMessageBoxBodyColors {
            normal: WidgetryMessageBoxBodyStateColors {
                background: kamuri_violet::ELEVATED_SURFACE,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryMessageBoxBodyStateColors {
                background: kamuri_violet::ELEVATED_SURFACE,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        action_button: super::button::KAMURI_VIOLET,
    };
}

mod progress_bar {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::WidgetryProgressBarColors;

    pub(super) const KAMURI_VIOLET: WidgetryProgressBarColors = WidgetryProgressBarColors {
        track: kamuri_violet::SURFACE,
        fill: kamuri_violet::PRIMARY,
        border: kamuri_violet::BORDER,
    };
}

mod radio_group {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryRadioGroupColors, WidgetryRadioGroupContainerColors,
        WidgetryRadioGroupContainerStateColors, WidgetryRadioOptionColors,
        WidgetryRadioOptionInteractionColors, WidgetryRadioOptionStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryRadioGroupColors = WidgetryRadioGroupColors {
        container: WidgetryRadioGroupContainerColors {
            normal: WidgetryRadioGroupContainerStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
            },
            focused: WidgetryRadioGroupContainerStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::FOCUS_BORDER,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryRadioGroupContainerStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        option: WidgetryRadioOptionColors {
            unchecked: WidgetryRadioOptionInteractionColors {
                normal: WidgetryRadioOptionStateColors {
                    background: kamuri_violet::SURFACE,
                    border: kamuri_violet::BORDER,
                    foreground: kamuri_violet::TEXT,
                    dot: kamuri_violet::TRANSPARENT,
                },
                hovered: WidgetryRadioOptionStateColors {
                    background: kamuri_violet::SURFACE,
                    border: kamuri_violet::PRIMARY_HOVER,
                    foreground: kamuri_violet::TEXT,
                    dot: kamuri_violet::TRANSPARENT,
                },
                disabled: WidgetryRadioOptionStateColors {
                    background: kamuri_violet::DISABLED_SURFACE,
                    border: kamuri_violet::DISABLED_BORDER,
                    foreground: kamuri_violet::DISABLED_TEXT,
                    dot: kamuri_violet::TRANSPARENT,
                },
            },
            checked: WidgetryRadioOptionInteractionColors {
                normal: WidgetryRadioOptionStateColors {
                    background: kamuri_violet::SURFACE,
                    border: kamuri_violet::PRIMARY,
                    foreground: kamuri_violet::TEXT,
                    dot: kamuri_violet::PRIMARY,
                },
                hovered: WidgetryRadioOptionStateColors {
                    background: kamuri_violet::SURFACE,
                    border: kamuri_violet::PRIMARY_HOVER,
                    foreground: kamuri_violet::TEXT,
                    dot: kamuri_violet::PRIMARY_HOVER,
                },
                disabled: WidgetryRadioOptionStateColors {
                    background: kamuri_violet::DISABLED_SURFACE,
                    border: kamuri_violet::DISABLED_BORDER,
                    foreground: kamuri_violet::DISABLED_TEXT,
                    dot: kamuri_violet::DISABLED_TEXT,
                },
            },
        },
    };
}

mod scroll_area {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryScrollAreaColors, WidgetryScrollAxisColors, WidgetryScrollThumbColors,
        WidgetryScrollThumbStateColors, WidgetryScrollTrackColors, WidgetryScrollTrackStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryScrollAreaColors = WidgetryScrollAreaColors {
        horizontal: WidgetryScrollAxisColors {
            track: WidgetryScrollTrackColors {
                normal: WidgetryScrollTrackStateColors {
                    background: kamuri_violet::TRANSPARENT,
                },
                disabled: WidgetryScrollTrackStateColors {
                    background: kamuri_violet::TRANSPARENT,
                },
            },
            thumb: WidgetryScrollThumbColors {
                normal: WidgetryScrollThumbStateColors {
                    background: kamuri_violet::BORDER,
                },
                hovered: WidgetryScrollThumbStateColors {
                    background: kamuri_violet::PRIMARY_HOVER,
                },
                dragged: WidgetryScrollThumbStateColors {
                    background: kamuri_violet::PRIMARY_PRESSED,
                },
                disabled: WidgetryScrollThumbStateColors {
                    background: kamuri_violet::DISABLED_BORDER,
                },
            },
        },
        vertical: WidgetryScrollAxisColors {
            track: WidgetryScrollTrackColors {
                normal: WidgetryScrollTrackStateColors {
                    background: kamuri_violet::TRANSPARENT,
                },
                disabled: WidgetryScrollTrackStateColors {
                    background: kamuri_violet::TRANSPARENT,
                },
            },
            thumb: WidgetryScrollThumbColors {
                normal: WidgetryScrollThumbStateColors {
                    background: kamuri_violet::BORDER,
                },
                hovered: WidgetryScrollThumbStateColors {
                    background: kamuri_violet::PRIMARY_HOVER,
                },
                dragged: WidgetryScrollThumbStateColors {
                    background: kamuri_violet::PRIMARY_PRESSED,
                },
                disabled: WidgetryScrollThumbStateColors {
                    background: kamuri_violet::DISABLED_BORDER,
                },
            },
        },
    };
}

mod segmented {
    use super::palette as kamuri_violet;
    use bevy::prelude::Color;
    use bevy_widgetry::theme::{
        WidgetrySegmentedColors, WidgetrySegmentedContainerColors,
        WidgetrySegmentedContainerStateColors, WidgetrySegmentedOptionColors,
        WidgetrySegmentedOptionInteractionColors, WidgetrySegmentedOptionStateColors,
    };

    const fn colors(
        container: Color,
        selection: Color,
        hover: Color,
        disabled_surface: Color,
        disabled_selection: Color,
        foreground: Color,
        disabled_foreground: Color,
        focus: Color,
    ) -> WidgetrySegmentedColors {
        WidgetrySegmentedColors {
            container: WidgetrySegmentedContainerColors {
                normal: WidgetrySegmentedContainerStateColors {
                    background: container,
                    border: Color::NONE,
                },
                focused: WidgetrySegmentedContainerStateColors {
                    background: container,
                    border: focus,
                },
                disabled: WidgetrySegmentedContainerStateColors {
                    background: disabled_surface,
                    border: Color::NONE,
                },
            },
            option: WidgetrySegmentedOptionColors {
                unchecked: WidgetrySegmentedOptionInteractionColors {
                    normal: WidgetrySegmentedOptionStateColors {
                        background: Color::NONE,
                        foreground,
                    },
                    hovered: WidgetrySegmentedOptionStateColors {
                        background: hover,
                        foreground,
                    },
                    disabled: WidgetrySegmentedOptionStateColors {
                        background: Color::NONE,
                        foreground: disabled_foreground,
                    },
                },
                checked: WidgetrySegmentedOptionInteractionColors {
                    normal: WidgetrySegmentedOptionStateColors {
                        background: selection,
                        foreground,
                    },
                    hovered: WidgetrySegmentedOptionStateColors {
                        background: selection,
                        foreground,
                    },
                    disabled: WidgetrySegmentedOptionStateColors {
                        background: disabled_selection,
                        foreground: disabled_foreground,
                    },
                },
            },
        }
    }

    pub(super) const KAMURI_VIOLET: WidgetrySegmentedColors = colors(
        kamuri_violet::HOVER_SURFACE,
        kamuri_violet::ELEVATED_SURFACE,
        kamuri_violet::PRESSED_SURFACE,
        kamuri_violet::DISABLED_SURFACE,
        kamuri_violet::PRESSED_SURFACE,
        kamuri_violet::TEXT,
        kamuri_violet::DISABLED_TEXT,
        kamuri_violet::FOCUS_BORDER,
    );
}

mod slider {
    use super::palette as kamuri_violet;
    use bevy::prelude::Color;
    use bevy_widgetry::theme::{WidgetrySliderColors, WidgetrySliderStateColors};

    pub(super) const KAMURI_VIOLET: WidgetrySliderColors = WidgetrySliderColors {
        normal: WidgetrySliderStateColors {
            track: kamuri_violet::BORDER,
            fill: kamuri_violet::PRIMARY,
            thumb: kamuri_violet::PRIMARY,
            focus_ring: kamuri_violet::PRIMARY,
        },
        hovered: WidgetrySliderStateColors {
            track: kamuri_violet::BORDER,
            fill: kamuri_violet::PRIMARY_HOVER,
            thumb: kamuri_violet::PRIMARY_HOVER,
            focus_ring: kamuri_violet::PRIMARY_HOVER,
        },
        pressed: WidgetrySliderStateColors {
            track: kamuri_violet::BORDER,
            fill: kamuri_violet::PRIMARY_PRESSED,
            thumb: kamuri_violet::PRIMARY_PRESSED,
            focus_ring: kamuri_violet::PRIMARY_PRESSED,
        },
        disabled: WidgetrySliderStateColors {
            track: kamuri_violet::DISABLED_BORDER,
            fill: kamuri_violet::DISABLED_TEXT,
            thumb: kamuri_violet::DISABLED_TEXT,
            focus_ring: Color::NONE,
        },
    };
}

mod splitter {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::WidgetrySplitterColors;

    pub(super) const KAMURI_VIOLET: WidgetrySplitterColors = WidgetrySplitterColors {
        normal: kamuri_violet::BORDER,
        hovered: kamuri_violet::PRIMARY_HOVER,
        dragging: kamuri_violet::PRIMARY,
        disabled: kamuri_violet::SUBTLE_BORDER,
    };
}

mod table {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryTableColors, WidgetryTableRegionColors, WidgetryTableStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryTableColors = WidgetryTableColors {
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
}

mod tabs {
    use super::palette as kamuri_violet;
    use bevy::prelude::Color;
    use bevy_widgetry::theme::{
        WidgetryTabColors, WidgetryTabInteractionColors, WidgetryTabStateColors, WidgetryTabsColors,
    };

    macro_rules! palette {
        ($name:ident, $base:ident) => {
            pub(super) const $name: WidgetryTabsColors = WidgetryTabsColors {
                header: WidgetryTabColors {
                    unselected: WidgetryTabInteractionColors {
                        normal: state($base::HOVER_SURFACE, $base::BORDER, $base::TEXT),
                        hovered: state($base::PRESSED_SURFACE, $base::BORDER, $base::TEXT),
                        pressed: state($base::SELECTED_SURFACE, $base::BORDER, $base::TEXT),
                        disabled: state(
                            $base::DISABLED_SURFACE,
                            $base::DISABLED_BORDER,
                            $base::DISABLED_TEXT,
                        ),
                    },
                    selected: WidgetryTabInteractionColors {
                        normal: state($base::ELEVATED_SURFACE, $base::PRIMARY, $base::PRIMARY),
                        hovered: state(
                            $base::HOVER_SURFACE,
                            $base::PRIMARY_HOVER,
                            $base::PRIMARY_HOVER,
                        ),
                        pressed: state(
                            $base::PRESSED_SURFACE,
                            $base::PRIMARY_PRESSED,
                            $base::PRIMARY_PRESSED,
                        ),
                        disabled: state(
                            $base::SELECTED_SURFACE,
                            $base::DISABLED_BORDER,
                            $base::DISABLED_TEXT,
                        ),
                    },
                    focus_border: $base::FOCUS_BORDER,
                },
                bar_background: $base::SURFACE,
                panel_background: $base::ELEVATED_SURFACE,
                separator: $base::BORDER,
            };
        };
    }

    const fn state(background: Color, border: Color, foreground: Color) -> WidgetryTabStateColors {
        WidgetryTabStateColors {
            background,
            border,
            foreground,
        }
    }

    palette!(KAMURI_VIOLET, kamuri_violet);
}

mod text {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{WidgetryTextColors, WidgetryTextStateColors};

    pub(super) const KAMURI_VIOLET: WidgetryTextColors = WidgetryTextColors {
        normal: WidgetryTextStateColors {
            foreground: kamuri_violet::TEXT,
        },
        disabled: WidgetryTextStateColors {
            foreground: kamuri_violet::DISABLED_TEXT,
        },
    };
}

mod text_field {
    use super::palette as kamuri_violet;
    use bevy::prelude::Color;
    use bevy_widgetry::theme::{
        WidgetryTextFieldColors, WidgetryTextFieldInteractionColors, WidgetryTextFieldStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryTextFieldColors = WidgetryTextFieldColors {
        hint_warning: Color::srgb_u8(116, 91, 0),
        editable: WidgetryTextFieldInteractionColors {
            normal: WidgetryTextFieldStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
                caret: kamuri_violet::TEXT,
                selection_background: kamuri_violet::SELECTION_BACKGROUND,
                unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                selection_foreground: kamuri_violet::TEXT,
            },
            hovered: WidgetryTextFieldStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                caret: kamuri_violet::TEXT,
                selection_background: kamuri_violet::SELECTION_BACKGROUND,
                unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                selection_foreground: kamuri_violet::TEXT,
            },
            focused: WidgetryTextFieldStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::FOCUS_BORDER,
                foreground: kamuri_violet::TEXT,
                caret: kamuri_violet::TEXT,
                selection_background: kamuri_violet::SELECTION_BACKGROUND,
                unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                selection_foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryTextFieldStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                caret: kamuri_violet::DISABLED_TEXT,
                selection_background: kamuri_violet::SELECTION_BACKGROUND,
                unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                selection_foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        read_only: WidgetryTextFieldInteractionColors {
            normal: WidgetryTextFieldStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
                caret: kamuri_violet::TEXT,
                selection_background: kamuri_violet::SELECTION_BACKGROUND,
                unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                selection_foreground: kamuri_violet::TEXT,
            },
            hovered: WidgetryTextFieldStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::PRIMARY_HOVER,
                foreground: kamuri_violet::TEXT,
                caret: kamuri_violet::TEXT,
                selection_background: kamuri_violet::SELECTION_BACKGROUND,
                unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                selection_foreground: kamuri_violet::TEXT,
            },
            focused: WidgetryTextFieldStateColors {
                background: kamuri_violet::SURFACE,
                border: kamuri_violet::FOCUS_BORDER,
                foreground: kamuri_violet::TEXT,
                caret: kamuri_violet::TEXT,
                selection_background: kamuri_violet::SELECTION_BACKGROUND,
                unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                selection_foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryTextFieldStateColors {
                background: kamuri_violet::DISABLED_SURFACE,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
                caret: kamuri_violet::DISABLED_TEXT,
                selection_background: kamuri_violet::SELECTION_BACKGROUND,
                unfocused_selection_background: kamuri_violet::UNFOCUSED_SELECTION_BACKGROUND,
                selection_foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
    };
}

mod tooltip {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryTooltipColors, WidgetryTooltipPopupColors, WidgetryTooltipStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryTooltipColors = WidgetryTooltipColors {
        popup: WidgetryTooltipPopupColors {
            normal: WidgetryTooltipStateColors {
                background: kamuri_violet::TOOLTIP_BACKGROUND,
                border: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::INVERSE_TEXT,
            },
        },
    };
}

mod tree {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryTreeColors, WidgetryTreeExpanderColors, WidgetryTreeExpanderInteractionColors,
        WidgetryTreeExpanderStateColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryTreeColors = WidgetryTreeColors {
        container: super::list_view::KAMURI_VIOLET.container,
        item: super::list_view::KAMURI_VIOLET.item,
        expander: WidgetryTreeExpanderColors {
            collapsed: WidgetryTreeExpanderInteractionColors {
                normal: WidgetryTreeExpanderStateColors {
                    background: kamuri_violet::TRANSPARENT,
                    border: kamuri_violet::TRANSPARENT,
                    foreground: kamuri_violet::TEXT,
                },
                hovered: WidgetryTreeExpanderStateColors {
                    background: kamuri_violet::HOVER_SURFACE,
                    border: kamuri_violet::TRANSPARENT,
                    foreground: kamuri_violet::TEXT,
                },
                pressed: WidgetryTreeExpanderStateColors {
                    background: kamuri_violet::PRESSED_SURFACE,
                    border: kamuri_violet::TRANSPARENT,
                    foreground: kamuri_violet::TEXT,
                },
                disabled: WidgetryTreeExpanderStateColors {
                    background: kamuri_violet::TRANSPARENT,
                    border: kamuri_violet::TRANSPARENT,
                    foreground: kamuri_violet::DISABLED_TEXT,
                },
            },
            expanded: WidgetryTreeExpanderInteractionColors {
                normal: WidgetryTreeExpanderStateColors {
                    background: kamuri_violet::TRANSPARENT,
                    border: kamuri_violet::TRANSPARENT,
                    foreground: kamuri_violet::TEXT,
                },
                hovered: WidgetryTreeExpanderStateColors {
                    background: kamuri_violet::HOVER_SURFACE,
                    border: kamuri_violet::TRANSPARENT,
                    foreground: kamuri_violet::TEXT,
                },
                pressed: WidgetryTreeExpanderStateColors {
                    background: kamuri_violet::PRESSED_SURFACE,
                    border: kamuri_violet::TRANSPARENT,
                    foreground: kamuri_violet::TEXT,
                },
                disabled: WidgetryTreeExpanderStateColors {
                    background: kamuri_violet::TRANSPARENT,
                    border: kamuri_violet::TRANSPARENT,
                    foreground: kamuri_violet::DISABLED_TEXT,
                },
            },
        },
    };
}

mod waveform {
    use super::palette as kamuri_violet;
    use bevy::prelude::Color;
    use bevy_widgetry::theme::{WidgetryWaveformColors, WidgetryWaveformStateColors};

    const KAMURI_VIOLET_NORMAL_PALETTE: &[Color] = &[
        Color::srgb_u8(110, 78, 163),
        Color::srgb_u8(169, 83, 135),
        Color::srgb_u8(54, 117, 132),
        Color::srgb_u8(153, 113, 47),
    ];

    const KAMURI_VIOLET_DISABLED_PALETTE: &[Color] = &[
        Color::srgb_u8(174, 164, 189),
        Color::srgb_u8(190, 182, 201),
        Color::srgb_u8(208, 202, 217),
        Color::srgb_u8(223, 218, 231),
    ];

    pub(super) const KAMURI_VIOLET: WidgetryWaveformColors = WidgetryWaveformColors {
        normal: WidgetryWaveformStateColors {
            background: kamuri_violet::SURFACE,
            palette: KAMURI_VIOLET_NORMAL_PALETTE,
        },
        disabled: WidgetryWaveformStateColors {
            background: kamuri_violet::DISABLED_SURFACE,
            palette: KAMURI_VIOLET_DISABLED_PALETTE,
        },
    };
}

mod window {
    use super::palette as kamuri_violet;
    use bevy_widgetry::theme::{
        WidgetryWindowButtonColors, WidgetryWindowButtonStateColors, WidgetryWindowColors,
        WidgetryWindowStateColors, WidgetryWindowSurfaceColors,
    };

    pub(super) const KAMURI_VIOLET: WidgetryWindowColors = WidgetryWindowColors {
        frame: WidgetryWindowSurfaceColors {
            normal: WidgetryWindowStateColors {
                background: kamuri_violet::WINDOW_BACKGROUND,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryWindowStateColors {
                background: kamuri_violet::WINDOW_BACKGROUND,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        title_bar: WidgetryWindowSurfaceColors {
            normal: WidgetryWindowStateColors {
                background: kamuri_violet::TRANSPARENT,
                border: kamuri_violet::BORDER,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryWindowStateColors {
                background: kamuri_violet::TRANSPARENT,
                border: kamuri_violet::DISABLED_BORDER,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        minimize: WidgetryWindowButtonColors {
            normal: WidgetryWindowButtonStateColors {
                background: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            hovered: WidgetryWindowButtonStateColors {
                background: kamuri_violet::HOVER_SURFACE,
                foreground: kamuri_violet::TEXT,
            },
            pressed: WidgetryWindowButtonStateColors {
                background: kamuri_violet::PRESSED_SURFACE,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryWindowButtonStateColors {
                background: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        maximize: WidgetryWindowButtonColors {
            normal: WidgetryWindowButtonStateColors {
                background: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            hovered: WidgetryWindowButtonStateColors {
                background: kamuri_violet::HOVER_SURFACE,
                foreground: kamuri_violet::TEXT,
            },
            pressed: WidgetryWindowButtonStateColors {
                background: kamuri_violet::PRESSED_SURFACE,
                foreground: kamuri_violet::TEXT,
            },
            disabled: WidgetryWindowButtonStateColors {
                background: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        close: WidgetryWindowButtonColors {
            normal: WidgetryWindowButtonStateColors {
                background: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::TEXT,
            },
            hovered: WidgetryWindowButtonStateColors {
                background: kamuri_violet::DANGER_HOVER,
                foreground: kamuri_violet::INVERSE_TEXT,
            },
            pressed: WidgetryWindowButtonStateColors {
                background: kamuri_violet::DANGER_PRESSED,
                foreground: kamuri_violet::INVERSE_TEXT,
            },
            disabled: WidgetryWindowButtonStateColors {
                background: kamuri_violet::TRANSPARENT,
                foreground: kamuri_violet::DISABLED_TEXT,
            },
        },
        image_tint: kamuri_violet::IMAGE_TINT,
    };
}

pub(super) const KAMURI_VIOLET_THEME: WidgetryTheme = WidgetryTheme {
    text: self::text::KAMURI_VIOLET,
    icon: self::icon::KAMURI_VIOLET,
    button: self::button::KAMURI_VIOLET,
    hold_button: self::hold_button::KAMURI_VIOLET,
    link_button: self::link_button::KAMURI_VIOLET,
    check_box: self::check_box::KAMURI_VIOLET,
    radio_group: self::radio_group::KAMURI_VIOLET,
    segmented: self::segmented::KAMURI_VIOLET,
    tabs: self::tabs::KAMURI_VIOLET,
    collapsible: self::collapsible::KAMURI_VIOLET,
    date_picker: self::date_picker::KAMURI_VIOLET,
    slider: self::slider::KAMURI_VIOLET,
    splitter: self::splitter::KAMURI_VIOLET,
    progress_bar: self::progress_bar::KAMURI_VIOLET,
    text_field: self::text_field::KAMURI_VIOLET,
    combo_box: self::combo_box::KAMURI_VIOLET,
    scroll_area: self::scroll_area::KAMURI_VIOLET,
    list_view: self::list_view::KAMURI_VIOLET,
    tree: self::tree::KAMURI_VIOLET,
    table: self::table::KAMURI_VIOLET,
    tooltip: self::tooltip::KAMURI_VIOLET,
    window: self::window::KAMURI_VIOLET,
    message_box: self::message_box::KAMURI_VIOLET,
    file_dialog: self::file_dialog::KAMURI_VIOLET,
    waveform: self::waveform::KAMURI_VIOLET,
};

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::prelude::Color;

    #[test]
    fn combo_box_kamuri_violet_reuses_widget_colors() {
        assert_eq!(
            super::combo_box::KAMURI_VIOLET.list.item,
            super::list_view::KAMURI_VIOLET.item
        );
    }

    #[test]
    fn file_dialog_kamuri_violet_reuses_widget_colors() {
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.window,
            super::window::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.sidebar_button,
            super::button::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.toolbar_button,
            super::button::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.accept_button,
            super::button::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.cancel_button,
            super::button::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.folder_create_button,
            super::button::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.folder_cancel_button,
            super::button::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.overwrite_accept_button,
            super::button::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.overwrite_cancel_button,
            super::button::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.path_field,
            super::text_field::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.search_field,
            super::text_field::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.filename_field,
            super::text_field::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.folder_name_field,
            super::text_field::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.filter,
            super::combo_box::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.sort,
            super::combo_box::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.hidden_option,
            super::check_box::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.system_option,
            super::check_box::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.sidebar_scroll,
            super::scroll_area::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.entries_scroll,
            super::scroll_area::KAMURI_VIOLET
        );
        assert_eq!(
            super::file_dialog::KAMURI_VIOLET.confirmation,
            super::message_box::KAMURI_VIOLET
        );
    }

    #[test]
    fn message_box_kamuri_violet_reuses_widget_colors() {
        assert_eq!(
            super::message_box::KAMURI_VIOLET.window,
            super::window::KAMURI_VIOLET
        );
        assert_eq!(
            super::message_box::KAMURI_VIOLET.action_button,
            super::button::KAMURI_VIOLET
        );
    }

    #[test]
    fn tree_kamuri_violet_reuses_widget_colors() {
        assert_eq!(
            super::tree::KAMURI_VIOLET.container,
            super::list_view::KAMURI_VIOLET.container
        );
        assert_eq!(
            super::tree::KAMURI_VIOLET.item,
            super::list_view::KAMURI_VIOLET.item
        );
    }

    #[test]
    fn normal_text_contrast_matches_reference_budget() {
        const MIN_TEXT_CONTRAST: f32 = 4.5;

        for mode in [KAMURI_VIOLET_THEME] {
            let theme = mode;
            let text = theme.text.normal.foreground;
            for background in [
                theme.window.frame.normal.background,
                theme.button.normal.background,
                theme.button.hovered.background,
                theme.button.pressed.background,
                theme.combo_box.popup.normal.background,
                theme.list_view.item.selected.background,
                theme.text_field.editable.normal.selection_background,
                theme
                    .text_field
                    .editable
                    .normal
                    .unfocused_selection_background,
                theme.table.column_header.normal.background,
                theme.message_box.body.normal.background,
            ] {
                let ratio = contrast(text, background);
                assert!(
                    ratio >= MIN_TEXT_CONTRAST,
                    "{mode:?} 普通文字对比度 {ratio} 低于 4.5，background={background:?}"
                );
            }
            let ratio = contrast(
                theme.tooltip.popup.normal.foreground,
                theme.tooltip.popup.normal.background,
            );
            assert!(
                ratio >= MIN_TEXT_CONTRAST,
                "{mode:?} Tooltip 文字对比度 {ratio} 低于 4.5"
            );
            let ratio = contrast(
                theme.file_dialog.status.normal.foreground,
                theme.file_dialog.body.normal.background,
            );
            assert!(
                ratio >= MIN_TEXT_CONTRAST,
                "{mode:?} FileDialog status 文字对比度 {ratio} 低于 4.5"
            );
        }
    }

    #[test]
    fn new_theme_marks_focus_and_danger_icons_have_non_text_contrast() {
        const MIN_GRAPHIC_CONTRAST: f32 = 3.0;

        let themes = [&KAMURI_VIOLET_THEME];
        for theme in themes {
            for state in [theme.check_box.checked, theme.check_box.indeterminate] {
                for state in [state.normal, state.hovered, state.pressed] {
                    assert!(contrast(state.mark, state.background) >= MIN_GRAPHIC_CONTRAST);
                }
            }
            for state in [theme.window.close.hovered, theme.window.close.pressed] {
                assert!(contrast(state.foreground, state.background) >= MIN_GRAPHIC_CONTRAST);
            }
            assert!(
                contrast(
                    theme.text_field.editable.focused.border,
                    theme.text_field.editable.focused.background
                ) >= MIN_GRAPHIC_CONTRAST
            );
            for foreground in theme.waveform.normal.palette {
                assert!(
                    contrast(*foreground, theme.waveform.normal.background) >= MIN_GRAPHIC_CONTRAST
                );
            }
        }
    }

    fn contrast(foreground: Color, background: Color) -> f32 {
        let luminance = |color: Color| {
            let color = color.to_linear();
            {
                let [red_weight, green_weight, blue_weight] = [0.2126, 0.7152, 0.0722];
                red_weight * color.red + green_weight * color.green + blue_weight * color.blue
            }
        };
        let foreground = luminance(foreground);
        let background = luminance(background);
        {
            let luminance_offset = 0.05;
            (foreground.max(background) + luminance_offset)
                / (foreground.min(background) + luminance_offset)
        }
    }

    #[test]
    fn selected_headers_remain_distinct_when_enabled_and_disabled() {
        let palettes = [super::tabs::KAMURI_VIOLET];
        for palette in palettes {
            assert_ne!(
                palette.header.selected.normal.background,
                palette.header.unselected.normal.background
            );
            assert_ne!(
                palette.header.selected.disabled.background,
                palette.header.unselected.disabled.background
            );
            assert_ne!(
                palette.header.selected.normal.foreground,
                palette.header.selected.disabled.foreground
            );
            assert_eq!(
                palette.header.selected.normal.background,
                palette.panel_background
            );
        }
    }

    #[test]
    fn every_palette_keeps_checked_blocks_distinct_when_enabled_or_disabled() {
        let palettes = [super::segmented::KAMURI_VIOLET];
        for palette in palettes {
            assert_ne!(
                palette.option.checked.normal.background,
                palette.container.normal.background
            );
            assert_ne!(
                palette.option.checked.normal.background,
                palette.option.unchecked.normal.background
            );
            assert_ne!(
                palette.option.checked.disabled.background,
                palette.option.unchecked.disabled.background
            );
            assert_ne!(
                palette.container.focused.border,
                palette.container.normal.border
            );
        }
    }

    #[test]
    fn warning_text_has_readable_contrast_on_each_theme_background() {
        const MIN_TEXT_CONTRAST: f32 = 4.5;
        fn luminance(color: Color) -> f32 {
            let linear = color.to_linear();
            {
                let [red_weight, green_weight, blue_weight] = [0.2126, 0.7152, 0.0722];
                red_weight * linear.red + green_weight * linear.green + blue_weight * linear.blue
            }
        }
        let palettes = [(
            super::text_field::KAMURI_VIOLET,
            super::palette::WINDOW_BACKGROUND,
        )];
        for (theme, background) in palettes {
            for surface in [
                background,
                theme.editable.normal.background,
                theme.editable.disabled.background,
            ] {
                let a = luminance(theme.hint_warning);
                let b = luminance(surface);
                let contrast = {
                    let luminance_offset = 0.05;
                    (a.max(b) + luminance_offset) / (a.min(b) + luminance_offset)
                };
                assert!(
                    contrast >= MIN_TEXT_CONTRAST,
                    "warning contrast {contrast} on {surface:?}"
                );
            }
        }
    }
}
