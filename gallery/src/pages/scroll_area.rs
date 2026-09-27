use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui_widgets::Activate;
use bevy_widgetry::{
    button::WidgetryButton,
    scroll_area::{
        ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
        WidgetryScrollIntoView,
    },
    style::{ForegroundColor, ThemeChanged, ThemeMode},
};

/// 管理页面说明文字的 theme，以及两个可重复的 demo 操作。
pub(crate) struct ScrollAreaDemoPlugin;

/// 只对 ScrollArea 页面自有文字传播 theme foreground color。
#[derive(Component)]
struct ScrollAreaDemo;

/// 动态 Auto 示例中由按钮切换高度的普通内容 Node。
#[derive(Component)]
struct AutoTransitionContent;

/// 区分动态 Auto 操作与其他 Gallery Button。
#[derive(Component)]
struct AutoTransitionTrigger;

/// 区分 ScrollIntoView 操作与其他 Gallery Button。
#[derive(Component)]
struct ScrollIntoViewTrigger;

/// 标记远处目标，trigger 只需发送公开 WidgetryScrollIntoView event。
#[derive(Component)]
struct ScrollIntoViewDemoTarget;

/// 同屏展示三种 axis、Always、Hidden、动态 Auto 与 ScrollIntoView。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        #ScrollAreaDemo
        template(|_| Ok(ScrollAreaDemo))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node {
            width: percent(100), height: percent(100),
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::flex(4, 1.0)],
            column_gap: px(20), row_gap: px(24),
        }
        Children [
            section("Vertical Auto", "wheel Y / PageUp / PageDown / Home / End", bsn_list![vertical_auto()]),
            section("Horizontal Auto", "wheel X / Left / Right", bsn_list![horizontal_auto()]),
            section("Both Auto", "Y gutter 诱发 X overflow；无 Corner", bsn_list![both_auto()]),
            section("Always", "无 overflow 时仍保留 bar 和 gutter", bsn_list![always()]),
            section("Hidden", "内容可滚动，但不显示 bar", bsn_list![hidden()]),
            section("Auto transition", "切换内容高度，观察 bar 增减", bsn_list![auto_transition_demo()]),
            section("ScrollIntoView", "滚动后点击按钮，将 Target 对齐到顶部", bsn_list![scroll_into_view_demo()]),
        ]
    }
}

/// 给每种场景保留同样宽度，并把操作说明放在 ScrollArea 外部。
fn section(title: &'static str, description: &'static str, content: impl SceneList) -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [
            (Text(title) TextFont { font_size: FontSize::Px(18.0) }),
            (Text(description) TextFont { font_size: FontSize::Px(13.0) }),
            {content},
        ]
    }
}

/// 用 Gallery 自有 border 显示固定 Viewport 的可见边界，不改变 Widgetry track/thumb style。
fn frame(content: impl Scene) -> impl Scene {
    bsn! {
        Node { width: px(310), height: px(185), border: UiRect::all(px(1)), padding: UiRect::all(px(4)) }
        template(|_| Ok(BorderColor::all(Color::srgb_u8(110, 117, 129))))
        Children [({content})]
    }
}

/// 有边界和编号的普通内容块，方便观察两轴滚动的方向与距离。
fn tile(label: &'static str, width: f32, height: f32) -> impl Scene {
    bsn! {
        Node {
            width: px(width), height: px(height), flex_shrink: 0.0,
            border: UiRect::all(px(1)), padding: UiRect::all(px(8)),
        }
        template(|_| Ok(BorderColor::all(Color::srgb_u8(110, 117, 129))))
        Children [Text(label)]
    }
}

/// 纵向超高内容保留清晰的 row 顺序，供 wheel、keyboard 和 V bar 操作。
fn vertical_auto() -> impl Scene {
    frame(bsn! {
        #VerticalAuto
        @WidgetryScrollArea {
            @content: bsn! { Node { row_gap: px(8) } },
            @children: bsn_list![
                (tile("V 1", 280.0, 64.0)), (tile("V 2", 280.0, 64.0)),
                (tile("V 3", 280.0, 64.0)), (tile("V 4", 280.0, 64.0)),
                (tile("V 5", 280.0, 64.0)), (tile("V 6", 280.0, 64.0)),
            ],
        }
        Node { width: px(300), height: px(175) }
    })
}

/// 横向超宽内容由默认 Row Content 排列，供 wheel X 和 H bar 操作。
fn horizontal_auto() -> impl Scene {
    frame(bsn! {
        #HorizontalAuto
        @WidgetryScrollArea {
            @axis: ScrollAxis::Horizontal,
            @content: bsn! { Node { column_gap: px(8) } },
            @children: bsn_list![
                (tile("H 1", 130.0, 145.0)), (tile("H 2", 130.0, 145.0)),
                (tile("H 3", 130.0, 145.0)), (tile("H 4", 130.0, 145.0)),
                (tile("H 5", 130.0, 145.0)),
            ],
        }
        Node { width: px(300), height: px(175) }
    })
}

/// 内容宽度恰等于初始 Viewport；Y overflow 加入 V gutter 后才出现 X overflow。
fn both_auto() -> impl Scene {
    frame(bsn! {
        #BothAuto
        @WidgetryScrollArea {
            @axis: ScrollAxis::Both,
            @children: bsn_list![(tile("300 × 290 content", 300.0, 290.0))],
        }
        Node { width: px(300), height: px(175) }
    })
}

/// 无 overflow 仍显示 V bar，用于验证 Always 的完整 thumb 与预留 gutter。
fn always() -> impl Scene {
    frame(bsn! {
        #Always
        @WidgetryScrollArea {
            @scrollbar_visibility: {ScrollbarVisibility { vertical: ScrollbarPolicy::Always, ..default() }},
            @children: bsn_list![(tile("Fits viewport", 220.0, 72.0))],
        }
        Node { width: px(300), height: px(175) }
    })
}

/// 有 Y overflow 但不显示 V bar，滚动能力仍由官方 ScrollArea 提供。
fn hidden() -> impl Scene {
    frame(bsn! {
        #Hidden
        @WidgetryScrollArea {
            @scrollbar_visibility: {ScrollbarVisibility { vertical: ScrollbarPolicy::Hidden, ..default() }},
            @children: bsn_list![(tile("Scroll me without a bar", 280.0, 320.0))],
        }
        Node { width: px(300), height: px(175) }
    })
}

/// 普通 child 的高度切换能触发稳定的 Auto bar 重新求解。
fn auto_transition() -> impl Scene {
    frame(bsn! {
        #AutoTransition
        @WidgetryScrollArea {
            @children: bsn_list![(
                template(|_| Ok(AutoTransitionContent))
                template(|_| Ok(Name::new("AutoTransitionContent")))
                Node { width: px(280), height: px(80), flex_shrink: 0.0, padding: UiRect::all(px(8)) }
                Children [Text("Height: 80 / 290")]
            )],
        }
        Node { width: px(300), height: px(175) }
    })
}

/// 把 ScrollArea 与操作按钮放进同一 Gallery hierarchy，避免多 root scene 脱离页面容器。
fn auto_transition_demo() -> impl Scene {
    let area = auto_transition();
    let trigger = toggle_button();
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [({area}), ({trigger})]
    }
}

/// 目标放在多个普通 child 之后，由公开 event 沿最近的 Viewport 处理。
fn scroll_into_view() -> impl Scene {
    frame(bsn! {
        #ScrollIntoView
        @WidgetryScrollArea {
            @content: bsn! { Node { row_gap: px(8) } },
            @children: bsn_list![
                (tile("Before 1", 280.0, 72.0)),
                (tile("Before 2", 280.0, 72.0)),
                (tile("Before 3", 280.0, 72.0)),
                (tile("Before 4", 280.0, 72.0)),
                (
                    template(|_| Ok(ScrollIntoViewDemoTarget))
                    template(|_| Ok(Name::new("ScrollIntoViewTarget")))
                    tile("Target", 280.0, 64.0)),
                (tile("After target", 280.0, 190.0)),
            ],
        }
        Node { width: px(300), height: px(175) }
    })
}

/// 把 ScrollArea 与操作按钮放进同一 Gallery hierarchy，确保页面显隐覆盖完整 demo。
fn scroll_into_view_demo() -> impl Scene {
    let area = scroll_into_view();
    let trigger = into_view_button();
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [({area}), ({trigger})]
    }
}

/// 真实 Widgetry Button 提供 pointer 与 keyboard Activate，且只操作动态 Auto 示例。
fn toggle_button() -> impl Scene {
    bsn! {
        #AutoTransitionTrigger
        @WidgetryButton
        template(|_| Ok(AutoTransitionTrigger))
        on(toggle_auto_content)
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        Children [Text("Toggle content")]
    }
}

/// 向远处 Target 发送公开 event 的 Button，不直接改写 Viewport state。
fn into_view_button() -> impl Scene {
    bsn! {
        #ScrollIntoViewTrigger
        @WidgetryButton
        template(|_| Ok(ScrollIntoViewTrigger))
        on(trigger_scroll_into_view)
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        Children [Text("Show target")]
    }
}

/// 在普通内容 Node 上切换高度，交给 library 的 Auto policy 根据真实 layout 重算。
fn toggle_auto_content(
    event: On<Activate>,
    triggers: Query<(), With<AutoTransitionTrigger>>,
    mut contents: Query<&mut Node, With<AutoTransitionContent>>,
) {
    if !triggers.contains(event.entity) {
        return;
    }
    for mut node in &mut contents {
        node.height = if node.height == px(80) {
            px(290)
        } else {
            px(80)
        };
    }
}

/// Trigger 只发送 WidgetryScrollIntoView，不直接读取或改写 ScrollPosition。
fn trigger_scroll_into_view(
    event: On<Activate>,
    triggers: Query<(), With<ScrollIntoViewTrigger>>,
    targets: Query<Entity, With<ScrollIntoViewDemoTarget>>,
    mut commands: Commands,
) {
    if !triggers.contains(event.entity) {
        return;
    }
    if let Ok(target) = targets.single() {
        commands.trigger(WidgetryScrollIntoView { entity: target });
    }
}

/// ThemeChanged 时只刷新页面说明文字，Scrollbar theme 由 library 管理。
fn refresh_theme(
    event: On<ThemeChanged>,
    mut demos: Query<&mut Propagate<ForegroundColor>, With<ScrollAreaDemo>>,
) {
    for mut foreground in &mut demos {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

impl Plugin for ScrollAreaDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(refresh_theme);
    }
}
