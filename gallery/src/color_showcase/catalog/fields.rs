use super::{Catalog, ColorFields};
use bevy_widgetry::theme::*;

impl ColorFields for WidgetryButtonStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryButtonStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryButtonColors {
    fn catalog(self) -> Catalog {
        let WidgetryButtonColors {
            normal,
            hovered,
            pressed,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("pressed", pressed.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryCheckBoxStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryCheckBoxStateColors {
            background,
            border,
            foreground,
            mark,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
                ("mark".into(), mark),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryCheckBoxInteractionColors {
    fn catalog(self) -> Catalog {
        let WidgetryCheckBoxInteractionColors {
            normal,
            hovered,
            pressed,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("pressed", pressed.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryCheckBoxColors {
    fn catalog(self) -> Catalog {
        let WidgetryCheckBoxColors {
            unchecked,
            checked,
            indeterminate,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("unchecked", unchecked.catalog()),
                ("checked", checked.catalog()),
                ("indeterminate", indeterminate.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryComboBoxFieldStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryComboBoxFieldStateColors {
            background,
            border,
            foreground,
            indicator_foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
                ("indicator_foreground".into(), indicator_foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryComboBoxFieldColors {
    fn catalog(self) -> Catalog {
        let WidgetryComboBoxFieldColors {
            normal,
            hovered,
            pressed,
            open,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("pressed", pressed.catalog()),
                ("open", open.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryComboBoxPopupStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryComboBoxPopupStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryComboBoxPopupColors {
    fn catalog(self) -> Catalog {
        let WidgetryComboBoxPopupColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryComboBoxColors {
    fn catalog(self) -> Catalog {
        let WidgetryComboBoxColors { field, popup, list } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("field", field.catalog()),
                ("popup", popup.catalog()),
                ("list", list.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryFileDialogBodyStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryFileDialogBodyStateColors {
            background,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryFileDialogBodyColors {
    fn catalog(self) -> Catalog {
        let WidgetryFileDialogBodyColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryFileDialogEntryStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryFileDialogEntryStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryFileDialogEntryColors {
    fn catalog(self) -> Catalog {
        let WidgetryFileDialogEntryColors {
            normal,
            selected,
            disabled,
            active_border,
            disabled_active_border,
        } = self;
        Catalog {
            fields: vec![
                ("active_border".into(), active_border),
                ("disabled_active_border".into(), disabled_active_border),
            ],
            groups: vec![
                ("normal", normal.catalog()),
                ("selected", selected.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryFileDialogStatusStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryFileDialogStatusStateColors { foreground } = self;
        Catalog {
            fields: vec![("foreground".into(), foreground)],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryFileDialogStatusColors {
    fn catalog(self) -> Catalog {
        let WidgetryFileDialogStatusColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryFileDialogColors {
    fn catalog(self) -> Catalog {
        let WidgetryFileDialogColors {
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
            confirmation,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("window", window.catalog()),
                ("body", body.catalog()),
                ("entry", entry.catalog()),
                ("status", status.catalog()),
                ("sidebar_button", sidebar_button.catalog()),
                ("toolbar_button", toolbar_button.catalog()),
                ("accept_button", accept_button.catalog()),
                ("cancel_button", cancel_button.catalog()),
                ("folder_create_button", folder_create_button.catalog()),
                ("folder_cancel_button", folder_cancel_button.catalog()),
                ("overwrite_accept_button", overwrite_accept_button.catalog()),
                ("overwrite_cancel_button", overwrite_cancel_button.catalog()),
                ("path_field", path_field.catalog()),
                ("search_field", search_field.catalog()),
                ("filename_field", filename_field.catalog()),
                ("folder_name_field", folder_name_field.catalog()),
                ("filter", filter.catalog()),
                ("sort", sort.catalog()),
                ("hidden_option", hidden_option.catalog()),
                ("system_option", system_option.catalog()),
                ("sidebar_scroll", sidebar_scroll.catalog()),
                ("entries_scroll", entries_scroll.catalog()),
                ("confirmation", confirmation.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryIconStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryIconStateColors { foreground } = self;
        Catalog {
            fields: vec![("foreground".into(), foreground)],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryIconColors {
    fn catalog(self) -> Catalog {
        let WidgetryIconColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryListViewContainerStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryListViewContainerStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryListViewContainerColors {
    fn catalog(self) -> Catalog {
        let WidgetryListViewContainerColors {
            normal,
            focused,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("focused", focused.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryListViewItemStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryListViewItemStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryListViewItemColors {
    fn catalog(self) -> Catalog {
        let WidgetryListViewItemColors {
            normal,
            hovered,
            pressed,
            selected,
            disabled,
            active_border,
            disabled_active_border,
        } = self;
        Catalog {
            fields: vec![
                ("active_border".into(), active_border),
                ("disabled_active_border".into(), disabled_active_border),
            ],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("pressed", pressed.catalog()),
                ("selected", selected.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryListViewColors {
    fn catalog(self) -> Catalog {
        let WidgetryListViewColors { container, item } = self;
        Catalog {
            fields: vec![],
            groups: vec![("container", container.catalog()), ("item", item.catalog())],
        }
    }
}

impl ColorFields for WidgetryMessageBoxBodyStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryMessageBoxBodyStateColors {
            background,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryMessageBoxBodyColors {
    fn catalog(self) -> Catalog {
        let WidgetryMessageBoxBodyColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryMessageBoxColors {
    fn catalog(self) -> Catalog {
        let WidgetryMessageBoxColors {
            window,
            body,
            action_button,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("window", window.catalog()),
                ("body", body.catalog()),
                ("action_button", action_button.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryRadioGroupContainerStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryRadioGroupContainerStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryRadioGroupContainerColors {
    fn catalog(self) -> Catalog {
        let WidgetryRadioGroupContainerColors {
            normal,
            focused,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("focused", focused.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryRadioOptionStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryRadioOptionStateColors {
            background,
            border,
            foreground,
            dot,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
                ("dot".into(), dot),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryRadioOptionInteractionColors {
    fn catalog(self) -> Catalog {
        let WidgetryRadioOptionInteractionColors {
            normal,
            hovered,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryRadioOptionColors {
    fn catalog(self) -> Catalog {
        let WidgetryRadioOptionColors { unchecked, checked } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("unchecked", unchecked.catalog()),
                ("checked", checked.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryRadioGroupColors {
    fn catalog(self) -> Catalog {
        let WidgetryRadioGroupColors { container, option } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("container", container.catalog()),
                ("option", option.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryScrollTrackStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryScrollTrackStateColors { background } = self;
        Catalog {
            fields: vec![("background".into(), background)],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryScrollTrackColors {
    fn catalog(self) -> Catalog {
        let WidgetryScrollTrackColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryScrollThumbStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryScrollThumbStateColors { background } = self;
        Catalog {
            fields: vec![("background".into(), background)],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryScrollThumbColors {
    fn catalog(self) -> Catalog {
        let WidgetryScrollThumbColors {
            normal,
            hovered,
            dragged,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("dragged", dragged.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryScrollAxisColors {
    fn catalog(self) -> Catalog {
        let WidgetryScrollAxisColors { track, thumb } = self;
        Catalog {
            fields: vec![],
            groups: vec![("track", track.catalog()), ("thumb", thumb.catalog())],
        }
    }
}

impl ColorFields for WidgetryScrollAreaColors {
    fn catalog(self) -> Catalog {
        let WidgetryScrollAreaColors {
            horizontal,
            vertical,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("horizontal", horizontal.catalog()),
                ("vertical", vertical.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryTableStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryTableStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryTableRegionColors {
    fn catalog(self) -> Catalog {
        let WidgetryTableRegionColors {
            normal,
            hovered,
            selected,
            disabled,
            focused_border,
            disabled_focused_border,
        } = self;
        Catalog {
            fields: vec![
                ("focused_border".into(), focused_border),
                ("disabled_focused_border".into(), disabled_focused_border),
            ],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("selected", selected.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryTableColors {
    fn catalog(self) -> Catalog {
        let WidgetryTableColors {
            table,
            column_header,
            row_header,
            corner,
            cell,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("table", table.catalog()),
                ("column_header", column_header.catalog()),
                ("row_header", row_header.catalog()),
                ("corner", corner.catalog()),
                ("cell", cell.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryTextStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryTextStateColors { foreground } = self;
        Catalog {
            fields: vec![("foreground".into(), foreground)],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryTextColors {
    fn catalog(self) -> Catalog {
        let WidgetryTextColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryTextFieldStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryTextFieldStateColors {
            background,
            border,
            foreground,
            caret,
            selection_background,
            unfocused_selection_background,
            selection_foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
                ("caret".into(), caret),
                ("selection_background".into(), selection_background),
                (
                    "unfocused_selection_background".into(),
                    unfocused_selection_background,
                ),
                ("selection_foreground".into(), selection_foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryTextFieldInteractionColors {
    fn catalog(self) -> Catalog {
        let WidgetryTextFieldInteractionColors {
            normal,
            hovered,
            focused,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("focused", focused.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryTextFieldColors {
    fn catalog(self) -> Catalog {
        let WidgetryTextFieldColors {
            editable,
            read_only,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("editable", editable.catalog()),
                ("read_only", read_only.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryTooltipStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryTooltipStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryTooltipPopupColors {
    fn catalog(self) -> Catalog {
        let WidgetryTooltipPopupColors { normal } = self;
        Catalog {
            fields: vec![],
            groups: vec![("normal", normal.catalog())],
        }
    }
}

impl ColorFields for WidgetryTooltipColors {
    fn catalog(self) -> Catalog {
        let WidgetryTooltipColors { popup } = self;
        Catalog {
            fields: vec![],
            groups: vec![("popup", popup.catalog())],
        }
    }
}

impl ColorFields for WidgetryTreeExpanderStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryTreeExpanderStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryTreeExpanderInteractionColors {
    fn catalog(self) -> Catalog {
        let WidgetryTreeExpanderInteractionColors {
            normal,
            hovered,
            pressed,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("pressed", pressed.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryTreeExpanderColors {
    fn catalog(self) -> Catalog {
        let WidgetryTreeExpanderColors {
            collapsed,
            expanded,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("collapsed", collapsed.catalog()),
                ("expanded", expanded.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryTreeColors {
    fn catalog(self) -> Catalog {
        let WidgetryTreeColors {
            container,
            item,
            expander,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("container", container.catalog()),
                ("item", item.catalog()),
                ("expander", expander.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryWaveformStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryWaveformStateColors {
            background,
            palette,
        } = self;
        let mut fields = vec![("background".into(), background)];
        fields.extend(
            palette
                .iter()
                .enumerate()
                .map(|(index, color)| (format!("palette[{index}]"), *color)),
        );
        Catalog {
            fields,
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryWaveformColors {
    fn catalog(self) -> Catalog {
        let WidgetryWaveformColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryWindowStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryWindowStateColors {
            background,
            border,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("border".into(), border),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryWindowSurfaceColors {
    fn catalog(self) -> Catalog {
        let WidgetryWindowSurfaceColors { normal, disabled } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryWindowButtonStateColors {
    fn catalog(self) -> Catalog {
        let WidgetryWindowButtonStateColors {
            background,
            foreground,
        } = self;
        Catalog {
            fields: vec![
                ("background".into(), background),
                ("foreground".into(), foreground),
            ],
            groups: vec![],
        }
    }
}

impl ColorFields for WidgetryWindowButtonColors {
    fn catalog(self) -> Catalog {
        let WidgetryWindowButtonColors {
            normal,
            hovered,
            pressed,
            disabled,
        } = self;
        Catalog {
            fields: vec![],
            groups: vec![
                ("normal", normal.catalog()),
                ("hovered", hovered.catalog()),
                ("pressed", pressed.catalog()),
                ("disabled", disabled.catalog()),
            ],
        }
    }
}

impl ColorFields for WidgetryWindowColors {
    fn catalog(self) -> Catalog {
        let WidgetryWindowColors {
            frame,
            title_bar,
            minimize,
            maximize,
            close,
            image_tint,
        } = self;
        Catalog {
            fields: vec![("image_tint".into(), image_tint)],
            groups: vec![
                ("frame", frame.catalog()),
                ("title_bar", title_bar.catalog()),
                ("minimize", minimize.catalog()),
                ("maximize", maximize.catalog()),
                ("close", close.catalog()),
            ],
        }
    }
}
