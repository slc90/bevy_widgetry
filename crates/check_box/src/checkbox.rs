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
///
/// 程序可直接在 root insert / remove 官方 Checked；此路径不发 ValueChange<bool>，
/// InteractionDisabled 不禁止程序直接写入。官方 SetChecked / ToggleChecked 请求则受 disabled guard 限制，
/// 有效请求通过与用户输入相同的 ValueChange<bool> 路径通知，没有来源字段。
/// 在 root 上通过 On<ValueChange<bool>> 消费 event.value 目标值；is_final 表示 interaction 结束。
/// 内建 checkbox_self_update 通过 Commands 排队更新 Checked，不能保证同一轮 consumer observer
/// 查询 Checked 已得到新值；此官方通知不承诺 authority 先于 event 提交，也不自动沿 hierarchy propagation。
#[derive(SceneComponent, Default, Clone)]
#[require(crate::style::StyleDiagnostics)]
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
