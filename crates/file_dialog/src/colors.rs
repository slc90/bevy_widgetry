use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryMessageBoxBodyStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryMessageBoxBodyStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryMessageBoxBodyStateColors,
    ) -> bevy_widgetry_theme::WidgetryMessageBoxBodyStateColors {
        bevy_widgetry_theme::WidgetryMessageBoxBodyStateColors {
            background: self.background.unwrap_or(theme.background),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryMessageBoxBodyColorOverrides {
    pub normal: WidgetryMessageBoxBodyStateColorOverrides,
    pub disabled: WidgetryMessageBoxBodyStateColorOverrides,
}
impl WidgetryMessageBoxBodyColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryMessageBoxBodyColors,
    ) -> bevy_widgetry_theme::WidgetryMessageBoxBodyColors {
        bevy_widgetry_theme::WidgetryMessageBoxBodyColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryMessageBoxColorOverrides {
    pub window: WidgetryWindowColorOverrides,
    pub body: WidgetryMessageBoxBodyColorOverrides,
    pub action_button: WidgetryButtonColorOverrides,
}
impl WidgetryMessageBoxColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.window.validate()?;
        self.body.validate()?;
        self.action_button.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryMessageBoxColors,
    ) -> bevy_widgetry_theme::WidgetryMessageBoxColors {
        bevy_widgetry_theme::WidgetryMessageBoxColors {
            window: self.window.resolve(&theme.window),
            body: self.body.resolve(&theme.body),
            action_button: self.action_button.resolve(&theme.action_button),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollThumbStateColorOverrides {
    pub background: Option<Color>,
}
impl WidgetryScrollThumbStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollThumbStateColors,
    ) -> bevy_widgetry_theme::WidgetryScrollThumbStateColors {
        bevy_widgetry_theme::WidgetryScrollThumbStateColors {
            background: self.background.unwrap_or(theme.background),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollThumbColorOverrides {
    pub normal: WidgetryScrollThumbStateColorOverrides,
    pub hovered: WidgetryScrollThumbStateColorOverrides,
    pub dragged: WidgetryScrollThumbStateColorOverrides,
    pub disabled: WidgetryScrollThumbStateColorOverrides,
}
impl WidgetryScrollThumbColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.dragged.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollThumbColors,
    ) -> bevy_widgetry_theme::WidgetryScrollThumbColors {
        bevy_widgetry_theme::WidgetryScrollThumbColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            dragged: self.dragged.resolve(&theme.dragged),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollTrackStateColorOverrides {
    pub background: Option<Color>,
}
impl WidgetryScrollTrackStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollTrackStateColors,
    ) -> bevy_widgetry_theme::WidgetryScrollTrackStateColors {
        bevy_widgetry_theme::WidgetryScrollTrackStateColors {
            background: self.background.unwrap_or(theme.background),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollTrackColorOverrides {
    pub normal: WidgetryScrollTrackStateColorOverrides,
    pub disabled: WidgetryScrollTrackStateColorOverrides,
}
impl WidgetryScrollTrackColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollTrackColors,
    ) -> bevy_widgetry_theme::WidgetryScrollTrackColors {
        bevy_widgetry_theme::WidgetryScrollTrackColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollAxisColorOverrides {
    pub track: WidgetryScrollTrackColorOverrides,
    pub thumb: WidgetryScrollThumbColorOverrides,
}
impl WidgetryScrollAxisColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.track.validate()?;
        self.thumb.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollAxisColors,
    ) -> bevy_widgetry_theme::WidgetryScrollAxisColors {
        bevy_widgetry_theme::WidgetryScrollAxisColors {
            track: self.track.resolve(&theme.track),
            thumb: self.thumb.resolve(&theme.thumb),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryScrollAreaColorOverrides {
    pub horizontal: WidgetryScrollAxisColorOverrides,
    pub vertical: WidgetryScrollAxisColorOverrides,
}
impl WidgetryScrollAreaColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.horizontal.validate()?;
        self.vertical.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryScrollAreaColors,
    ) -> bevy_widgetry_theme::WidgetryScrollAreaColors {
        bevy_widgetry_theme::WidgetryScrollAreaColors {
            horizontal: self.horizontal.resolve(&theme.horizontal),
            vertical: self.vertical.resolve(&theme.vertical),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub mark: Option<Color>,
}
impl WidgetryCheckBoxStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.mark {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxStateColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxStateColors {
        bevy_widgetry_theme::WidgetryCheckBoxStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            mark: self.mark.unwrap_or(theme.mark),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxInteractionColorOverrides {
    pub normal: WidgetryCheckBoxStateColorOverrides,
    pub hovered: WidgetryCheckBoxStateColorOverrides,
    pub pressed: WidgetryCheckBoxStateColorOverrides,
    pub disabled: WidgetryCheckBoxStateColorOverrides,
}
impl WidgetryCheckBoxInteractionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxInteractionColors {
        bevy_widgetry_theme::WidgetryCheckBoxInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryCheckBoxColorOverrides {
    pub unchecked: WidgetryCheckBoxInteractionColorOverrides,
    pub checked: WidgetryCheckBoxInteractionColorOverrides,
    pub indeterminate: WidgetryCheckBoxInteractionColorOverrides,
}
impl WidgetryCheckBoxColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.unchecked.validate()?;
        self.checked.validate()?;
        self.indeterminate.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryCheckBoxColors,
    ) -> bevy_widgetry_theme::WidgetryCheckBoxColors {
        bevy_widgetry_theme::WidgetryCheckBoxColors {
            unchecked: self.unchecked.resolve(&theme.unchecked),
            checked: self.checked.resolve(&theme.checked),
            indeterminate: self.indeterminate.resolve(&theme.indeterminate),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewItemStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryListViewItemStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewItemStateColors,
    ) -> bevy_widgetry_theme::WidgetryListViewItemStateColors {
        bevy_widgetry_theme::WidgetryListViewItemStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewItemColorOverrides {
    pub normal: WidgetryListViewItemStateColorOverrides,
    pub hovered: WidgetryListViewItemStateColorOverrides,
    pub pressed: WidgetryListViewItemStateColorOverrides,
    pub selected: WidgetryListViewItemStateColorOverrides,
    pub disabled: WidgetryListViewItemStateColorOverrides,
    pub active_border: Option<Color>,
    pub disabled_active_border: Option<Color>,
}
impl WidgetryListViewItemColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.selected.validate()?;
        self.disabled.validate()?;
        if let Some(color) = self.active_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.disabled_active_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewItemColors,
    ) -> bevy_widgetry_theme::WidgetryListViewItemColors {
        bevy_widgetry_theme::WidgetryListViewItemColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            selected: self.selected.resolve(&theme.selected),
            disabled: self.disabled.resolve(&theme.disabled),
            active_border: self.active_border.unwrap_or(theme.active_border),
            disabled_active_border: self
                .disabled_active_border
                .unwrap_or(theme.disabled_active_border),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewContainerStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryListViewContainerStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewContainerStateColors,
    ) -> bevy_widgetry_theme::WidgetryListViewContainerStateColors {
        bevy_widgetry_theme::WidgetryListViewContainerStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewContainerColorOverrides {
    pub normal: WidgetryListViewContainerStateColorOverrides,
    pub focused: WidgetryListViewContainerStateColorOverrides,
    pub disabled: WidgetryListViewContainerStateColorOverrides,
}
impl WidgetryListViewContainerColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.focused.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewContainerColors,
    ) -> bevy_widgetry_theme::WidgetryListViewContainerColors {
        bevy_widgetry_theme::WidgetryListViewContainerColors {
            normal: self.normal.resolve(&theme.normal),
            focused: self.focused.resolve(&theme.focused),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryListViewColorOverrides {
    pub container: WidgetryListViewContainerColorOverrides,
    pub item: WidgetryListViewItemColorOverrides,
}
impl WidgetryListViewColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.container.validate()?;
        self.item.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryListViewColors,
    ) -> bevy_widgetry_theme::WidgetryListViewColors {
        bevy_widgetry_theme::WidgetryListViewColors {
            container: self.container.resolve(&theme.container),
            item: self.item.resolve(&theme.item),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxPopupStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryComboBoxPopupStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxPopupStateColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxPopupStateColors {
        bevy_widgetry_theme::WidgetryComboBoxPopupStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxPopupColorOverrides {
    pub normal: WidgetryComboBoxPopupStateColorOverrides,
    pub disabled: WidgetryComboBoxPopupStateColorOverrides,
}
impl WidgetryComboBoxPopupColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxPopupColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxPopupColors {
        bevy_widgetry_theme::WidgetryComboBoxPopupColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxFieldStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub indicator_foreground: Option<Color>,
}
impl WidgetryComboBoxFieldStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.indicator_foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxFieldStateColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxFieldStateColors {
        bevy_widgetry_theme::WidgetryComboBoxFieldStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            indicator_foreground: self
                .indicator_foreground
                .unwrap_or(theme.indicator_foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxFieldColorOverrides {
    pub normal: WidgetryComboBoxFieldStateColorOverrides,
    pub hovered: WidgetryComboBoxFieldStateColorOverrides,
    pub pressed: WidgetryComboBoxFieldStateColorOverrides,
    pub open: WidgetryComboBoxFieldStateColorOverrides,
    pub disabled: WidgetryComboBoxFieldStateColorOverrides,
}
impl WidgetryComboBoxFieldColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.open.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxFieldColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxFieldColors {
        bevy_widgetry_theme::WidgetryComboBoxFieldColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            open: self.open.resolve(&theme.open),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryComboBoxColorOverrides {
    pub field: WidgetryComboBoxFieldColorOverrides,
    pub popup: WidgetryComboBoxPopupColorOverrides,
    pub list: WidgetryListViewColorOverrides,
}
impl WidgetryComboBoxColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.field.validate()?;
        self.popup.validate()?;
        self.list.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryComboBoxColors,
    ) -> bevy_widgetry_theme::WidgetryComboBoxColors {
        bevy_widgetry_theme::WidgetryComboBoxColors {
            field: self.field.resolve(&theme.field),
            popup: self.popup.resolve(&theme.popup),
            list: self.list.resolve(&theme.list),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
    pub caret: Option<Color>,
    pub selection_background: Option<Color>,
    pub unfocused_selection_background: Option<Color>,
    pub selection_foreground: Option<Color>,
}
impl WidgetryTextFieldStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.caret {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.selection_background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.unfocused_selection_background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.selection_foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldStateColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldStateColors {
        bevy_widgetry_theme::WidgetryTextFieldStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
            caret: self.caret.unwrap_or(theme.caret),
            selection_background: self
                .selection_background
                .unwrap_or(theme.selection_background),
            unfocused_selection_background: self
                .unfocused_selection_background
                .unwrap_or(theme.unfocused_selection_background),
            selection_foreground: self
                .selection_foreground
                .unwrap_or(theme.selection_foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldInteractionColorOverrides {
    pub normal: WidgetryTextFieldStateColorOverrides,
    pub hovered: WidgetryTextFieldStateColorOverrides,
    pub focused: WidgetryTextFieldStateColorOverrides,
    pub disabled: WidgetryTextFieldStateColorOverrides,
}
impl WidgetryTextFieldInteractionColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.focused.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldInteractionColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldInteractionColors {
        bevy_widgetry_theme::WidgetryTextFieldInteractionColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            focused: self.focused.resolve(&theme.focused),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryTextFieldColorOverrides {
    pub editable: WidgetryTextFieldInteractionColorOverrides,
    pub read_only: WidgetryTextFieldInteractionColorOverrides,
}
impl WidgetryTextFieldColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.editable.validate()?;
        self.read_only.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryTextFieldColors,
    ) -> bevy_widgetry_theme::WidgetryTextFieldColors {
        bevy_widgetry_theme::WidgetryTextFieldColors {
            editable: self.editable.resolve(&theme.editable),
            read_only: self.read_only.resolve(&theme.read_only),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryButtonStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryButtonStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryButtonStateColors,
    ) -> bevy_widgetry_theme::WidgetryButtonStateColors {
        bevy_widgetry_theme::WidgetryButtonStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryButtonColorOverrides {
    pub normal: WidgetryButtonStateColorOverrides,
    pub hovered: WidgetryButtonStateColorOverrides,
    pub pressed: WidgetryButtonStateColorOverrides,
    pub disabled: WidgetryButtonStateColorOverrides,
}
impl WidgetryButtonColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryButtonColors,
    ) -> bevy_widgetry_theme::WidgetryButtonColors {
        bevy_widgetry_theme::WidgetryButtonColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryFileDialogStatusStateColorOverrides {
    pub foreground: Option<Color>,
}
impl WidgetryFileDialogStatusStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryFileDialogStatusStateColors,
    ) -> bevy_widgetry_theme::WidgetryFileDialogStatusStateColors {
        bevy_widgetry_theme::WidgetryFileDialogStatusStateColors {
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryFileDialogStatusColorOverrides {
    pub normal: WidgetryFileDialogStatusStateColorOverrides,
    pub disabled: WidgetryFileDialogStatusStateColorOverrides,
}
impl WidgetryFileDialogStatusColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryFileDialogStatusColors,
    ) -> bevy_widgetry_theme::WidgetryFileDialogStatusColors {
        bevy_widgetry_theme::WidgetryFileDialogStatusColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryFileDialogEntryStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryFileDialogEntryStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryFileDialogEntryStateColors,
    ) -> bevy_widgetry_theme::WidgetryFileDialogEntryStateColors {
        bevy_widgetry_theme::WidgetryFileDialogEntryStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryFileDialogEntryColorOverrides {
    pub normal: WidgetryFileDialogEntryStateColorOverrides,
    pub selected: WidgetryFileDialogEntryStateColorOverrides,
    pub disabled: WidgetryFileDialogEntryStateColorOverrides,
    pub active_border: Option<Color>,
    pub disabled_active_border: Option<Color>,
}
impl WidgetryFileDialogEntryColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.selected.validate()?;
        self.disabled.validate()?;
        if let Some(color) = self.active_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.disabled_active_border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryFileDialogEntryColors,
    ) -> bevy_widgetry_theme::WidgetryFileDialogEntryColors {
        bevy_widgetry_theme::WidgetryFileDialogEntryColors {
            normal: self.normal.resolve(&theme.normal),
            selected: self.selected.resolve(&theme.selected),
            disabled: self.disabled.resolve(&theme.disabled),
            active_border: self.active_border.unwrap_or(theme.active_border),
            disabled_active_border: self
                .disabled_active_border
                .unwrap_or(theme.disabled_active_border),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryFileDialogBodyStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryFileDialogBodyStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryFileDialogBodyStateColors,
    ) -> bevy_widgetry_theme::WidgetryFileDialogBodyStateColors {
        bevy_widgetry_theme::WidgetryFileDialogBodyStateColors {
            background: self.background.unwrap_or(theme.background),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryFileDialogBodyColorOverrides {
    pub normal: WidgetryFileDialogBodyStateColorOverrides,
    pub disabled: WidgetryFileDialogBodyStateColorOverrides,
}
impl WidgetryFileDialogBodyColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryFileDialogBodyColors,
    ) -> bevy_widgetry_theme::WidgetryFileDialogBodyColors {
        bevy_widgetry_theme::WidgetryFileDialogBodyColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowButtonStateColorOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryWindowButtonStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowButtonStateColors,
    ) -> bevy_widgetry_theme::WidgetryWindowButtonStateColors {
        bevy_widgetry_theme::WidgetryWindowButtonStateColors {
            background: self.background.unwrap_or(theme.background),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowButtonColorOverrides {
    pub normal: WidgetryWindowButtonStateColorOverrides,
    pub hovered: WidgetryWindowButtonStateColorOverrides,
    pub pressed: WidgetryWindowButtonStateColorOverrides,
    pub disabled: WidgetryWindowButtonStateColorOverrides,
}
impl WidgetryWindowButtonColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.hovered.validate()?;
        self.pressed.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowButtonColors,
    ) -> bevy_widgetry_theme::WidgetryWindowButtonColors {
        bevy_widgetry_theme::WidgetryWindowButtonColors {
            normal: self.normal.resolve(&theme.normal),
            hovered: self.hovered.resolve(&theme.hovered),
            pressed: self.pressed.resolve(&theme.pressed),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowStateColorOverrides {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}
impl WidgetryWindowStateColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        if let Some(color) = self.background {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.border {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        if let Some(color) = self.foreground {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowStateColors,
    ) -> bevy_widgetry_theme::WidgetryWindowStateColors {
        bevy_widgetry_theme::WidgetryWindowStateColors {
            background: self.background.unwrap_or(theme.background),
            border: self.border.unwrap_or(theme.border),
            foreground: self.foreground.unwrap_or(theme.foreground),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowSurfaceColorOverrides {
    pub normal: WidgetryWindowStateColorOverrides,
    pub disabled: WidgetryWindowStateColorOverrides,
}
impl WidgetryWindowSurfaceColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.normal.validate()?;
        self.disabled.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowSurfaceColors,
    ) -> bevy_widgetry_theme::WidgetryWindowSurfaceColors {
        bevy_widgetry_theme::WidgetryWindowSurfaceColors {
            normal: self.normal.resolve(&theme.normal),
            disabled: self.disabled.resolve(&theme.disabled),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryWindowColorOverrides {
    pub frame: WidgetryWindowSurfaceColorOverrides,
    pub title_bar: WidgetryWindowSurfaceColorOverrides,
    pub minimize: WidgetryWindowButtonColorOverrides,
    pub maximize: WidgetryWindowButtonColorOverrides,
    pub close: WidgetryWindowButtonColorOverrides,
    pub image_tint: Option<Color>,
}
impl WidgetryWindowColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.frame.validate()?;
        self.title_bar.validate()?;
        self.minimize.validate()?;
        self.maximize.validate()?;
        self.close.validate()?;
        if let Some(color) = self.image_tint {
            bevy_widgetry_core::color::validate_color(color)?;
        }
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryWindowColors,
    ) -> bevy_widgetry_theme::WidgetryWindowColors {
        bevy_widgetry_theme::WidgetryWindowColors {
            frame: self.frame.resolve(&theme.frame),
            title_bar: self.title_bar.resolve(&theme.title_bar),
            minimize: self.minimize.resolve(&theme.minimize),
            maximize: self.maximize.resolve(&theme.maximize),
            close: self.close.resolve(&theme.close),
            image_tint: self.image_tint.unwrap_or(theme.image_tint),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetryFileDialogColorOverrides {
    pub window: WidgetryWindowColorOverrides,
    pub body: WidgetryFileDialogBodyColorOverrides,
    pub entry: WidgetryFileDialogEntryColorOverrides,
    pub status: WidgetryFileDialogStatusColorOverrides,
    pub sidebar_button: WidgetryButtonColorOverrides,
    pub toolbar_button: WidgetryButtonColorOverrides,
    pub accept_button: WidgetryButtonColorOverrides,
    pub cancel_button: WidgetryButtonColorOverrides,
    pub folder_create_button: WidgetryButtonColorOverrides,
    pub folder_cancel_button: WidgetryButtonColorOverrides,
    pub overwrite_accept_button: WidgetryButtonColorOverrides,
    pub overwrite_cancel_button: WidgetryButtonColorOverrides,
    pub path_field: WidgetryTextFieldColorOverrides,
    pub search_field: WidgetryTextFieldColorOverrides,
    pub filename_field: WidgetryTextFieldColorOverrides,
    pub folder_name_field: WidgetryTextFieldColorOverrides,
    pub filter: WidgetryComboBoxColorOverrides,
    pub sort: WidgetryComboBoxColorOverrides,
    pub hidden_option: WidgetryCheckBoxColorOverrides,
    pub system_option: WidgetryCheckBoxColorOverrides,
    pub sidebar_scroll: WidgetryScrollAreaColorOverrides,
    pub entries_scroll: WidgetryScrollAreaColorOverrides,
    pub confirmation: WidgetryMessageBoxColorOverrides,
}
impl WidgetryFileDialogColorOverrides {
    fn validate(&self) -> Result<(), BevyError> {
        self.window.validate()?;
        self.body.validate()?;
        self.entry.validate()?;
        self.status.validate()?;
        self.sidebar_button.validate()?;
        self.toolbar_button.validate()?;
        self.accept_button.validate()?;
        self.cancel_button.validate()?;
        self.folder_create_button.validate()?;
        self.folder_cancel_button.validate()?;
        self.overwrite_accept_button.validate()?;
        self.overwrite_cancel_button.validate()?;
        self.path_field.validate()?;
        self.search_field.validate()?;
        self.filename_field.validate()?;
        self.folder_name_field.validate()?;
        self.filter.validate()?;
        self.sort.validate()?;
        self.hidden_option.validate()?;
        self.system_option.validate()?;
        self.sidebar_scroll.validate()?;
        self.entries_scroll.validate()?;
        self.confirmation.validate()?;
        Ok(())
    }
    pub(crate) fn resolve(
        &self,
        theme: &bevy_widgetry_theme::WidgetryFileDialogColors,
    ) -> bevy_widgetry_theme::WidgetryFileDialogColors {
        bevy_widgetry_theme::WidgetryFileDialogColors {
            window: self.window.resolve(&theme.window),
            body: self.body.resolve(&theme.body),
            entry: self.entry.resolve(&theme.entry),
            status: self.status.resolve(&theme.status),
            sidebar_button: self.sidebar_button.resolve(&theme.sidebar_button),
            toolbar_button: self.toolbar_button.resolve(&theme.toolbar_button),
            accept_button: self.accept_button.resolve(&theme.accept_button),
            cancel_button: self.cancel_button.resolve(&theme.cancel_button),
            folder_create_button: self
                .folder_create_button
                .resolve(&theme.folder_create_button),
            folder_cancel_button: self
                .folder_cancel_button
                .resolve(&theme.folder_cancel_button),
            overwrite_accept_button: self
                .overwrite_accept_button
                .resolve(&theme.overwrite_accept_button),
            overwrite_cancel_button: self
                .overwrite_cancel_button
                .resolve(&theme.overwrite_cancel_button),
            path_field: self.path_field.resolve(&theme.path_field),
            search_field: self.search_field.resolve(&theme.search_field),
            filename_field: self.filename_field.resolve(&theme.filename_field),
            folder_name_field: self.folder_name_field.resolve(&theme.folder_name_field),
            filter: self.filter.resolve(&theme.filter),
            sort: self.sort.resolve(&theme.sort),
            hidden_option: self.hidden_option.resolve(&theme.hidden_option),
            system_option: self.system_option.resolve(&theme.system_option),
            sidebar_scroll: self.sidebar_scroll.resolve(&theme.sidebar_scroll),
            entries_scroll: self.entries_scroll.resolve(&theme.entries_scroll),
            confirmation: self.confirmation.resolve(&theme.confirmation),
        }
    }
}
#[derive(Component, Default)]
pub(crate) struct ColorState(pub(crate) WidgetryFileDialogColorOverrides);
impl WidgetryFileDialogColorOverrides {
    pub fn get(world: &World, entity: Entity) -> Result<&Self, BevyError> {
        if world.get::<crate::WidgetryFileDialog>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<ColorState>(entity)
            .map(|state| &state.0)
            .ok_or_else(|| invalid_target(entity))
    }
    pub fn set_in_world(
        world: &mut World,
        entity: Entity,
        colors: Self,
    ) -> Result<bool, BevyError> {
        Self::get(world, entity)?;
        if world
            .get::<bevy_widgetry_core::color::WidgetryStyleOwner<crate::WidgetryFileDialog>>(entity)
            .is_some()
        {
            return Err(invalid_target(entity));
        }
        colors.validate()?;
        let mut state = world
            .get_mut::<ColorState>(entity)
            .ok_or_else(|| invalid_target(entity))?;
        if state.0 == colors {
            return Ok(false);
        }
        state.0 = colors;
        Ok(true)
    }
    pub fn set(commands: &mut Commands, entity: Entity, colors: Self) {
        commands
            .queue(move |world: &mut World| Self::set_in_world(world, entity, colors).map(|_| ()));
    }
    pub fn clear_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        Self::set_in_world(world, entity, Self::default())
    }
    pub fn clear(commands: &mut Commands, entity: Entity) {
        Self::set(commands, entity, Self::default());
    }
    pub(crate) fn initial(self) -> Result<ColorState, BevyError> {
        self.validate()?;
        Ok(ColorState(self))
    }
}
#[cold]
fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "WidgetryFileDialogColorOverrides 目标不存在、类型不匹配或属于托管部件"
    );
    BevyError::error("WidgetryFileDialogColorOverrides 目标不存在、类型不匹配或属于托管部件")
}
