use crate::assets::GalleryIcon;
use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::{
    button::WidgetryButton,
    icon::WidgetryIcon,
    style::{ForegroundColor, ThemeChanged, ThemeMode},
    tooltip::{TooltipContentFactory, WidgetryTooltip},
};

/// 刷新 Tooltip 页面自身标题颜色，不接管 popup theme。
pub(crate) struct TooltipDemoPlugin;

/// 标记 Tooltip 页面 foreground color 传播 root。
#[derive(Component)]
struct TooltipDemo;

/// 展示普通文本、任意 rich content、disabled anchor 与 window 边缘 placement。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        #TooltipDemo
        template(|_| Ok(TooltipDemo))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node {
            width: percent(100), height: percent(100),
            flex_direction: FlexDirection::Column, row_gap: px(16),
        }
        Children [
            Text("Tooltip"),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [Text("Basic"), basic_button()]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [Text("Rich Content"), rich_button()]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [Text("Disabled"), disabled_button()]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8), flex_grow: 1.0, min_height: px(0) }
                Children [Text("Placement"), placement_area()]),
        ]
    }
}

/// 创建基础文本 Tooltip 示例。
fn basic_button() -> impl Scene {
    bsn! {
        #BasicTooltip
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list![Text("Basic tooltip")])} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text("Hover me")]
    }
}

/// 创建包含 icon、多段 Text 与自定义 layout 的 Tooltip 示例。
fn rich_button() -> impl Scene {
    bsn! {
        #RichTooltip
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list![
            (Node { align_items: AlignItems::Center, column_gap: px(8) }
                Children [
                    (@WidgetryIcon {
                        @path: {GalleryIcon::ButtonStar.path()},
                        @max_size: {Some(UVec2::new(20, 20))},
                    } Node { width: px(20), height: px(20) }),
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(2) }
                        Children [Text("Rich tooltip"), Text("Icon and multiple lines")]),
                ]),
        ])} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text("Rich content")]
    }
}

/// disabled state 只影响 Button interaction，不参与 Tooltip hover resolution。
fn disabled_button() -> impl Scene {
    bsn! {
        #DisabledTooltip
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list![Text("Disabled controls still have tooltips")])} }
        InteractionDisabled
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text("Disabled")]
    }
}

/// 在正常、右边缘与底边缘放置三个 anchor，肉眼验证 Popover fallback。
fn placement_area() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_grow: 1.0, min_height: px(180) }
        Children [
            (#CenterTooltip placement_button("Center") Node { position_type: PositionType::Absolute, left: percent(40), top: px(32) }),
            (#RightTooltip placement_button("Right") Node { position_type: PositionType::Absolute, right: px(0), top: px(92) }),
            (#BottomTooltip placement_button("Bottom") Node { position_type: PositionType::Absolute, left: percent(40), bottom: px(0) }),
        ]
    }
}

/// 为 placement 位置构造相同内容的 Tooltip button。
fn placement_button(label: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(move || bsn_list![Text({format!("{label} placement")})])} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text(label)]
    }
}

/// ThemeChanged 时刷新页面自有 foreground color，popup 由 Tooltip plugin 独立刷新。
fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<TooltipDemo>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

impl Plugin for TooltipDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(refresh_theme);
    }
}
