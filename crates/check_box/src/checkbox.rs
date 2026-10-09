use crate::indicator::checkbox_indicator_scene;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BackgroundColor, BorderColor};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};
use bevy_widgetry_core::foreground::ResolvedForeground;

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetryCheckBoxProps)]
#[require(crate::style::StyleDiagnostics, crate::colors::ColorState)]
pub struct WidgetryCheckBox;

impl WidgetryCheckBox {
    fn scene(props: crate::WidgetryCheckBoxProps) -> impl Scene {
        bsn! {
            template(move |_| props.colors.clone().initial())
            Checkbox
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            Hovered(false)
            TabIndex(-1)
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(6), min_height: px(24) }
            BackgroundColor
            BorderColor
            template(|_| Ok(ResolvedForeground::default()))
            on(checkbox_self_update)
            Children [checkbox_indicator_scene()]
        }
    }
}
