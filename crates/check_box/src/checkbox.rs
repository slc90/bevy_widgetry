use crate::indicator::checkbox_indicator_scene;
use bevy::app::Propagate;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BackgroundColor, BorderColor};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};
use bevy_widgetry_core::ForegroundColor;

/// 基于 Bevy 官方 Checkbox 的二态 Widget；Checked 是唯一选中 state。
/// 需注册 WidgetryCheckBoxPlugin；调用方通过 BSN Children 添加 label。
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryCheckBox;

impl WidgetryCheckBox {
    /// 展开共享外壳；默认未选中，用户 ValueChange 由官方 observer 更新 Checked。
    fn scene() -> impl Scene {
        bsn! {
            Checkbox
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
