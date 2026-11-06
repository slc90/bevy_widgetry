use crate::{ThemeError, ThemeFailure, WidgetryTheme};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

trait ValidateColors {
    fn validate(&self, path: &str) -> Result<(), ThemeFailure>;
}

impl ValidateColors for Color {
    fn validate(&self, path: &str) -> Result<(), ThemeFailure> {
        let rgba = self.to_linear();
        if [rgba.red, rgba.green, rgba.blue, rgba.alpha]
            .iter()
            .all(|v| v.is_finite())
            && (0.0..=1.0).contains(&rgba.alpha)
        {
            return Ok(());
        }
        Err(ThemeFailure::new(
            ThemeError::InvalidColor {
                field: path.to_owned(),
                color: *self,
            },
            ErrorFrame::new("theme.validate").field("field", path),
        ))
    }
}

impl ValidateColors for &[Color] {
    fn validate(&self, path: &str) -> Result<(), ThemeFailure> {
        if self.len() < 2
            || self
                .iter()
                .zip(self.iter().cycle().skip(1))
                .any(|(a, b)| a.to_linear() == b.to_linear())
        {
            return Err(ThemeFailure::new(
                ThemeError::InvalidPalette {
                    field: path.to_owned(),
                },
                ErrorFrame::new("theme.validate").field("field", path),
            ));
        }
        for (index, color) in self.iter().enumerate() {
            color.validate(&format!("{path}[{index}]"))?;
        }
        Ok(())
    }
}

macro_rules! validate_fields {
    ($ty:ty { $($field:ident),+ $(,)? }) => {
        impl ValidateColors for $ty {
            fn validate(&self, path: &str) -> Result<(), ThemeFailure> {
                $(self.$field.validate(&format!("{path}.{}", stringify!($field)))?;)+
                Ok(())
            }
        }
    };
}

pub(super) fn validate(theme: &WidgetryTheme) -> Result<(), ThemeFailure> {
    theme.validate("theme")
}

validate_fields!(crate::WidgetryButtonStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryButtonColors {
    normal,
    hovered,
    pressed,
    disabled
});
validate_fields!(crate::WidgetryCheckBoxStateColors {
    background,
    border,
    foreground,
    mark
});
validate_fields!(crate::WidgetryCheckBoxInteractionColors {
    normal,
    hovered,
    pressed,
    disabled
});
validate_fields!(crate::WidgetryCheckBoxColors {
    unchecked,
    checked,
    indeterminate
});
validate_fields!(crate::WidgetryCollapsibleColors {
    background,
    header_background,
    content_background,
    border,
    separator
});
validate_fields!(crate::WidgetryComboBoxFieldStateColors {
    background,
    border,
    foreground,
    indicator_foreground
});
validate_fields!(crate::WidgetryComboBoxFieldColors {
    normal,
    hovered,
    pressed,
    open,
    disabled
});
validate_fields!(crate::WidgetryComboBoxPopupStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryComboBoxPopupColors { normal, disabled });
validate_fields!(crate::WidgetryComboBoxColors { field, popup, list });
validate_fields!(crate::WidgetryDatePickerColors {
    background,
    border,
    foreground,
    adjacent,
    disabled,
    selected,
    hovered,
    pressed
});
validate_fields!(crate::WidgetryFileDialogBodyStateColors {
    background,
    foreground
});
validate_fields!(crate::WidgetryFileDialogBodyColors { normal, disabled });
validate_fields!(crate::WidgetryFileDialogEntryStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryFileDialogEntryColors {
    normal,
    selected,
    disabled,
    active_border,
    disabled_active_border
});
validate_fields!(crate::WidgetryFileDialogStatusStateColors { foreground });
validate_fields!(crate::WidgetryFileDialogStatusColors { normal, disabled });
validate_fields!(crate::WidgetryFileDialogColors {
    window,
    body,
    entry,
    status,
    sidebar_button,
    toolbar_button,
    accept_button,
    cancel_button,
    folder_create_button,
    folder_cancel_button,
    overwrite_accept_button,
    overwrite_cancel_button,
    path_field,
    search_field,
    filename_field,
    folder_name_field,
    filter,
    sort,
    hidden_option,
    system_option,
    sidebar_scroll,
    entries_scroll,
    confirmation
});
validate_fields!(crate::WidgetryHoldButtonStateColors {
    background,
    liquid,
    wave,
    border,
    foreground
});
validate_fields!(crate::WidgetryHoldButtonColors {
    normal,
    hovered,
    pressed,
    disabled,
    success_border
});
validate_fields!(crate::WidgetryIconStateColors { foreground });
validate_fields!(crate::WidgetryIconColors { normal, disabled });
validate_fields!(crate::WidgetryLinkButtonColors {
    normal,
    hovered,
    pressed,
    disabled
});
validate_fields!(crate::WidgetryListViewContainerStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryListViewContainerColors {
    normal,
    focused,
    disabled
});
validate_fields!(crate::WidgetryListViewItemStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryListViewItemColors {
    normal,
    hovered,
    pressed,
    selected,
    disabled,
    active_border,
    disabled_active_border
});
validate_fields!(crate::WidgetryListViewColors { container, item });
validate_fields!(crate::WidgetryMessageBoxBodyStateColors {
    background,
    foreground
});
validate_fields!(crate::WidgetryMessageBoxBodyColors { normal, disabled });
validate_fields!(crate::WidgetryMessageBoxColors {
    window,
    body,
    action_button
});
validate_fields!(crate::WidgetryProgressBarColors {
    track,
    fill,
    border
});
validate_fields!(crate::WidgetryRadioGroupContainerStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryRadioGroupContainerColors {
    normal,
    focused,
    disabled
});
validate_fields!(crate::WidgetryRadioOptionStateColors {
    background,
    border,
    foreground,
    dot
});
validate_fields!(crate::WidgetryRadioOptionInteractionColors {
    normal,
    hovered,
    disabled
});
validate_fields!(crate::WidgetryRadioOptionColors { unchecked, checked });
validate_fields!(crate::WidgetryRadioGroupColors { container, option });
validate_fields!(crate::WidgetryScrollTrackStateColors { background });
validate_fields!(crate::WidgetryScrollTrackColors { normal, disabled });
validate_fields!(crate::WidgetryScrollThumbStateColors { background });
validate_fields!(crate::WidgetryScrollThumbColors {
    normal,
    hovered,
    dragged,
    disabled
});
validate_fields!(crate::WidgetryScrollAxisColors { track, thumb });
validate_fields!(crate::WidgetryScrollAreaColors {
    horizontal,
    vertical
});
validate_fields!(crate::WidgetrySegmentedContainerStateColors { background, border });
validate_fields!(crate::WidgetrySegmentedContainerColors {
    normal,
    focused,
    disabled
});
validate_fields!(crate::WidgetrySegmentedOptionStateColors {
    background,
    foreground
});
validate_fields!(crate::WidgetrySegmentedOptionInteractionColors {
    normal,
    hovered,
    disabled
});
validate_fields!(crate::WidgetrySegmentedOptionColors { unchecked, checked });
validate_fields!(crate::WidgetrySegmentedColors { container, option });
validate_fields!(crate::WidgetrySliderStateColors {
    track,
    fill,
    thumb,
    focus_ring
});
validate_fields!(crate::WidgetrySliderColors {
    normal,
    hovered,
    pressed,
    disabled
});
validate_fields!(crate::WidgetrySplitterColors {
    normal,
    hovered,
    dragging,
    disabled
});
validate_fields!(crate::WidgetryTableStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryTableRegionColors {
    normal,
    hovered,
    selected,
    disabled,
    focused_border,
    disabled_focused_border
});
validate_fields!(crate::WidgetryTableColors {
    table,
    column_header,
    row_header,
    corner,
    cell
});
validate_fields!(crate::WidgetryTabStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryTabInteractionColors {
    normal,
    hovered,
    pressed,
    disabled
});
validate_fields!(crate::WidgetryTabColors {
    unselected,
    selected,
    focus_border
});
validate_fields!(crate::WidgetryTabsColors {
    header,
    bar_background,
    panel_background,
    separator
});
validate_fields!(crate::WidgetryTextStateColors { foreground });
validate_fields!(crate::WidgetryTextColors { normal, disabled });
validate_fields!(crate::WidgetryTextFieldStateColors {
    background,
    border,
    foreground,
    caret,
    selection_background,
    unfocused_selection_background,
    selection_foreground
});
validate_fields!(crate::WidgetryTextFieldInteractionColors {
    normal,
    hovered,
    focused,
    disabled
});
validate_fields!(crate::WidgetryTextFieldColors {
    hint_warning,
    editable,
    read_only
});
validate_fields!(crate::WidgetryTheme {
    text,
    icon,
    button,
    hold_button,
    link_button,
    check_box,
    radio_group,
    segmented,
    tabs,
    collapsible,
    date_picker,
    slider,
    splitter,
    progress_bar,
    text_field,
    combo_box,
    scroll_area,
    list_view,
    tree,
    table,
    tooltip,
    window,
    message_box,
    file_dialog,
    waveform
});
validate_fields!(crate::WidgetryTooltipStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryTooltipPopupColors { normal });
validate_fields!(crate::WidgetryTooltipColors { popup });
validate_fields!(crate::WidgetryTreeExpanderStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryTreeExpanderInteractionColors {
    normal,
    hovered,
    pressed,
    disabled
});
validate_fields!(crate::WidgetryTreeExpanderColors {
    collapsed,
    expanded
});
validate_fields!(crate::WidgetryTreeColors {
    container,
    item,
    expander
});
validate_fields!(crate::WidgetryWaveformStateColors {
    background,
    palette
});
validate_fields!(crate::WidgetryWaveformColors { normal, disabled });
validate_fields!(crate::WidgetryWindowStateColors {
    background,
    border,
    foreground
});
validate_fields!(crate::WidgetryWindowSurfaceColors { normal, disabled });
validate_fields!(crate::WidgetryWindowButtonStateColors {
    background,
    foreground
});
validate_fields!(crate::WidgetryWindowButtonColors {
    normal,
    hovered,
    pressed,
    disabled
});
validate_fields!(crate::WidgetryWindowColors {
    frame,
    title_bar,
    minimize,
    maximize,
    close,
    image_tint
});
