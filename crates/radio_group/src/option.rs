use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::RadioButton;
use bevy_widgetry_core::ForegroundColor;

/// RadioGroup 的直接 option；需注册 WidgetryRadioGroupPlugin，不能单独使用。
/// 用户 Children 追加在内建 indicator 后，字体由调用方配置。
/// Checked 是内部 state，初始化始终以 Group 的 index 0 为准；不支持单项 disabled。
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryRadioOption;

/// 标识内建外圈，避免 style 修改用户自带的内容。
#[derive(Component)]
pub(crate) struct RadioIndicator;

/// 标识内建圆点，其背景由 option 的 Checked state 驱动。
#[derive(Component)]
pub(crate) struct RadioDot;

impl WidgetryRadioOption {
    /// 展开 option 的完整点击区域及内建圆形 indicator。
    fn scene() -> impl Scene {
        bsn! {
            RadioButton
            Hovered(false)
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                min_height: px(24),
            }
            template(|_| Ok(Propagate(ForegroundColor::default())))
            Children [(
                template(|_| Ok(RadioIndicator))
                Node {
                    width: px(16), height: px(16),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::MAX,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                BorderColor
                Children [(
                    template(|_| Ok(RadioDot))
                    Node { width: px(8), height: px(8), border_radius: BorderRadius::MAX }
                    BackgroundColor
                )]
            )]
        }
    }
}
