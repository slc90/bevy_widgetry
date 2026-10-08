use crate::indicator::checkbox_indicator_scene;
use bevy::app::Propagate;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BackgroundColor, BorderColor};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};
use bevy_widgetry_core::ForegroundColor;

#[derive(SceneComponent, Default, Clone)]
#[require(crate::style::StyleDiagnostics)]
pub struct WidgetryCheckBox;

impl WidgetryCheckBox {
    fn scene() -> impl Scene {
        bsn! {
            Checkbox
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            Hovered(false)
            TabIndex(-1)
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(6), min_height: px(24) }
            BackgroundColor
            BorderColor
            template(|_| Ok(Propagate(ForegroundColor::default())))
            on(checkbox_self_update)
            Children [checkbox_indicator_scene()]
        }
    }
}
