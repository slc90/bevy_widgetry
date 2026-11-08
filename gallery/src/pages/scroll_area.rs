const VERTICAL_TILE_SIZE: Vec2 = Vec2::new(280.0, 64.0);
const HORIZONTAL_TILE_SIZE: Vec2 = Vec2::new(130.0, 145.0);
const REVEAL_LEADING_TILE_SIZE: Vec2 = Vec2::new(280.0, 72.0);
const OUTER_VIEW_WIDTH: f32 = 310.0;
const OUTER_VIEW_HEIGHT: f32 = 185.0;
const VIEW_BORDER_WIDTH: f32 = 1.0;
const OUTER_VIEW_PADDING: f32 = 4.0;
const CONTENT_PADDING: f32 = 8.0;
const VIEW_WIDTH: f32 = 300.0;
const VIEW_HEIGHT: f32 = 175.0;
const RESIZABLE_CONTENT_WIDTH: f32 = 280.0;
const COMPACT_CONTENT_HEIGHT: f32 = 80.0;
const EXPANDED_CONTENT_HEIGHT: f32 = 290.0;

const DETAIL_GAP: f32 = 8.0;
const ITEM_GAP: f32 = 24.0;
const CONTROL_GAP: f32 = 8.0;
const CONTENT_GAP: f32 = 20.0;
const LABEL_FONT_SIZE: f32 = 13.0;
const TITLE_FONT_SIZE: f32 = 18.0;

use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::ScrollAreaKey;
use crate::i18n::{TextKey, localized};
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui_widgets::Activate;
use bevy_widgetry::{
    button::WidgetryButton,
    scroll_area::{
        ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
        WidgetryScrollIntoView,
    },
};

pub(crate) struct ScrollAreaDemoPlugin;

impl Plugin for ScrollAreaDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GalleryPage::ScrollArea), enter)
            .add_systems(OnExit(GalleryPage::ScrollArea), exit);
    }
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::ScrollArea, bsn_list! { @scene() })?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::ScrollArea)
}

#[derive(Component)]
struct ScrollAreaDemo;

#[derive(Component)]
struct AutoTransitionContent;

#[derive(Component)]
struct AutoTransitionTrigger;

#[derive(Component)]
struct ScrollIntoViewTrigger;

#[derive(Component)]
struct ScrollIntoViewDemoTarget;

pub(crate) fn scene() -> impl Scene {
    bsn! {
        #ScrollAreaDemo
        template(|_| Ok(ScrollAreaDemo))
        Node {
            width: percent(100), height: percent(100),
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::flex(4, 1.0)],
            column_gap: px(CONTENT_GAP), row_gap: px(ITEM_GAP),
        }
        Children [
            @section(TextKey::ScrollArea(ScrollAreaKey::VerticalAuto),TextKey::ScrollArea(ScrollAreaKey::WheelYPageupPagedownHomeEnd), bsn_list!{@vertical_auto()})--
            @section(TextKey::ScrollArea(ScrollAreaKey::HorizontalAuto),TextKey::ScrollArea(ScrollAreaKey::WheelXLeftRight), bsn_list!{@horizontal_auto()})--
            @section(TextKey::ScrollArea(ScrollAreaKey::BothAuto),TextKey::ScrollArea(ScrollAreaKey::YGutterInducesXOverflowNoCorner), bsn_list!{@both_auto()})--
            @section(TextKey::ScrollArea(ScrollAreaKey::Always),TextKey::ScrollArea(ScrollAreaKey::KeepBarAndGutterEvenWithoutOverflow), bsn_list!{@always()})--
            @section(TextKey::ScrollArea(ScrollAreaKey::Hidden),TextKey::ScrollArea(ScrollAreaKey::ScrollableContentWithNoVisibleBar), bsn_list!{@hidden()})--
            @section(TextKey::ScrollArea(ScrollAreaKey::AutoTransition),TextKey::ScrollArea(ScrollAreaKey::ToggleContentHeightToShowOrHide), bsn_list!{@auto_transition_demo()})--
            @section(TextKey::ScrollArea(ScrollAreaKey::Scrollintoview),TextKey::ScrollArea(ScrollAreaKey::ScrollThenClickTheButtonToAlign), bsn_list!{@scroll_into_view_demo()})
        ]
    }
}

fn section(title: TextKey, description: TextKey, content: impl SceneList) -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(DETAIL_GAP), align_items: AlignItems::Start }
        Children [
            @localized(title) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(TITLE_FONT_SIZE) }--
            @localized(description) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(LABEL_FONT_SIZE) }--
            {content}
        ]
    }
}

fn frame(content: impl Scene) -> impl Scene {
    bsn! {
        Node { width: px(OUTER_VIEW_WIDTH), height: px(OUTER_VIEW_HEIGHT), border: UiRect::all(px(VIEW_BORDER_WIDTH)), padding: UiRect::all(px(OUTER_VIEW_PADDING)) }
        template(|_| Ok(BorderColor::all(Color::srgb_u8(110, 117, 129))))
        Children [@{content}]
    }
}

fn tile(label: TextKey, width: f32, height: f32) -> impl Scene {
    bsn! {
        Node {
            width: px(width), height: px(height), flex_shrink: 0.0,
            border: UiRect::all(px(VIEW_BORDER_WIDTH)), padding: UiRect::all(px(CONTENT_PADDING)),
        }
        template(|_| Ok(BorderColor::all(Color::srgb_u8(110, 117, 129))))
        Children [@localized(label) bevy_widgetry::text::WidgetryText]
    }
}

fn vertical_auto() -> impl Scene {
    frame(bsn! {
        #VerticalAuto
        @WidgetryScrollArea {
            @content: bsn! { Node { row_gap: px(DETAIL_GAP) } },
            @children: bsn_list!{
                @tile(TextKey::ScrollArea(ScrollAreaKey::V1), VERTICAL_TILE_SIZE.x, VERTICAL_TILE_SIZE.y)-- @tile(TextKey::ScrollArea(ScrollAreaKey::V2), VERTICAL_TILE_SIZE.x, VERTICAL_TILE_SIZE.y)--
                @tile(TextKey::ScrollArea(ScrollAreaKey::V3), VERTICAL_TILE_SIZE.x, VERTICAL_TILE_SIZE.y)-- @tile(TextKey::ScrollArea(ScrollAreaKey::V4), VERTICAL_TILE_SIZE.x, VERTICAL_TILE_SIZE.y)--
                @tile(TextKey::ScrollArea(ScrollAreaKey::V5), VERTICAL_TILE_SIZE.x, VERTICAL_TILE_SIZE.y)-- @tile(TextKey::ScrollArea(ScrollAreaKey::V6), VERTICAL_TILE_SIZE.x, VERTICAL_TILE_SIZE.y)
            },
        }
        Node { width: px(VIEW_WIDTH), height: px(VIEW_HEIGHT) }
    })
}

fn horizontal_auto() -> impl Scene {
    frame(bsn! {
        #HorizontalAuto
        @WidgetryScrollArea {
            @axis: ScrollAxis::Horizontal,
            @content: bsn! { Node { column_gap: px(CONTROL_GAP) } },
            @children: bsn_list!{
                @tile(TextKey::ScrollArea(ScrollAreaKey::H1), HORIZONTAL_TILE_SIZE.x, HORIZONTAL_TILE_SIZE.y)-- @tile(TextKey::ScrollArea(ScrollAreaKey::H2), HORIZONTAL_TILE_SIZE.x, HORIZONTAL_TILE_SIZE.y)--
                @tile(TextKey::ScrollArea(ScrollAreaKey::H3), HORIZONTAL_TILE_SIZE.x, HORIZONTAL_TILE_SIZE.y)-- @tile(TextKey::ScrollArea(ScrollAreaKey::H4), HORIZONTAL_TILE_SIZE.x, HORIZONTAL_TILE_SIZE.y)--
                @tile(TextKey::ScrollArea(ScrollAreaKey::H5), HORIZONTAL_TILE_SIZE.x, HORIZONTAL_TILE_SIZE.y)
            },
        }
        Node { width: px(VIEW_WIDTH), height: px(VIEW_HEIGHT) }
    })
}

fn both_auto() -> impl Scene {
    frame(bsn! {
        #BothAuto
        @WidgetryScrollArea {
            @axis: ScrollAxis::Both,
            @children: bsn_list!{@tile(TextKey::ScrollArea(ScrollAreaKey::Text300290Content), 300.0, 290.0)},
        }
        Node { width: px(VIEW_WIDTH), height: px(VIEW_HEIGHT) }
    })
}

fn always() -> impl Scene {
    frame(bsn! {
        #Always
        @WidgetryScrollArea {
            @scrollbar_visibility: {ScrollbarVisibility { vertical: ScrollbarPolicy::Always, ..default() }},
            @children: bsn_list!{@tile(TextKey::ScrollArea(ScrollAreaKey::FitsViewport), 220.0, 72.0)},
        }
        Node { width: px(VIEW_WIDTH), height: px(VIEW_HEIGHT) }
    })
}

fn hidden() -> impl Scene {
    frame(bsn! {
        #Hidden
        @WidgetryScrollArea {
            @scrollbar_visibility: {ScrollbarVisibility { vertical: ScrollbarPolicy::Hidden, ..default() }},
            @children: bsn_list!{@tile(TextKey::ScrollArea(ScrollAreaKey::ScrollMeWithoutABar), 280.0, 320.0)},
        }
        Node { width: px(VIEW_WIDTH), height: px(VIEW_HEIGHT) }
    })
}

fn auto_transition() -> impl Scene {
    frame(bsn! {
        #AutoTransition
        @WidgetryScrollArea {
            @children: bsn_list!{
                template(|_| Ok(AutoTransitionContent))
                template(|_| Ok(Name::new("AutoTransitionContent")))
                Node { width: px(RESIZABLE_CONTENT_WIDTH), height: px(COMPACT_CONTENT_HEIGHT), flex_shrink: 0.0, padding: UiRect::all(px(CONTENT_PADDING)) }
                Children [@localized(TextKey::ScrollArea(ScrollAreaKey::Height80290)) bevy_widgetry::text::WidgetryText]
            },
        }
        Node { width: px(VIEW_WIDTH), height: px(VIEW_HEIGHT) }
    })
}

fn auto_transition_demo() -> impl Scene {
    let area = auto_transition();
    let trigger = toggle_button();
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(DETAIL_GAP), align_items: AlignItems::Start }
        Children [@{area}-- @{trigger}]
    }
}

fn scroll_into_view() -> impl Scene {
    frame(bsn! {
        #ScrollIntoView
        @WidgetryScrollArea {
            @content: bsn! { Node { row_gap: px(DETAIL_GAP) } },
            @children: bsn_list!{
                @tile(TextKey::ScrollArea(ScrollAreaKey::Before1), REVEAL_LEADING_TILE_SIZE.x, REVEAL_LEADING_TILE_SIZE.y)--
                @tile(TextKey::ScrollArea(ScrollAreaKey::Before2), REVEAL_LEADING_TILE_SIZE.x, REVEAL_LEADING_TILE_SIZE.y)--
                @tile(TextKey::ScrollArea(ScrollAreaKey::Before3), REVEAL_LEADING_TILE_SIZE.x, REVEAL_LEADING_TILE_SIZE.y)--
                @tile(TextKey::ScrollArea(ScrollAreaKey::Before4), REVEAL_LEADING_TILE_SIZE.x, REVEAL_LEADING_TILE_SIZE.y)--

                    template(|_| Ok(ScrollIntoViewDemoTarget))
                    template(|_| Ok(Name::new("ScrollIntoViewTarget")))
                    @tile(TextKey::ScrollArea(ScrollAreaKey::Target), VERTICAL_TILE_SIZE.x, VERTICAL_TILE_SIZE.y)--
                @tile(TextKey::ScrollArea(ScrollAreaKey::AfterTarget), 280.0, 190.0)
            },
        }
        Node { width: px(VIEW_WIDTH), height: px(VIEW_HEIGHT) }
    })
}

fn scroll_into_view_demo() -> impl Scene {
    let area = scroll_into_view();
    let trigger = into_view_button();
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(DETAIL_GAP), align_items: AlignItems::Start }
        Children [@{area}-- @{trigger}]
    }
}

fn toggle_button() -> impl Scene {
    bsn! {
        #AutoTransitionTrigger
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::ScrollArea(ScrollAreaKey::ToggleContent)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        template(|_| Ok(AutoTransitionTrigger))
        on(toggle_auto_content)
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }

    }
}

fn into_view_button() -> impl Scene {
    bsn! {
        #ScrollIntoViewTrigger
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::ScrollArea(ScrollAreaKey::ShowTarget)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        template(|_| Ok(ScrollIntoViewTrigger))
        on(trigger_scroll_into_view)
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }

    }
}

fn toggle_auto_content(
    event: On<Activate>,
    triggers: Query<(), With<AutoTransitionTrigger>>,
    mut contents: Query<(Entity, &mut Node), With<AutoTransitionContent>>,
) {
    if !triggers.contains(event.entity) {
        return;
    }
    let mut changes = Vec::new();
    for (target, mut node) in &mut contents {
        node.height = if node.height == px(COMPACT_CONTENT_HEIGHT) {
            px(EXPANDED_CONTENT_HEIGHT)
        } else {
            px(COMPACT_CONTENT_HEIGHT)
        };
        changes.push((target, node.height));
    }
    info!(button = ?event.entity, updated = changes.len(), ?changes, unchanged = changes.is_empty(), "ScrollArea 示例内容高度切换结果");
}

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
        info!(
            ?target,
            outcome = "requested",
            "请求将 ScrollArea 演示目标滚动到可见区域"
        );
    } else {
        info!(
            target_count = targets.iter().count(),
            outcome = "not_submitted",
            "ScrollArea 演示没有唯一可用目标，未提交滚动请求"
        );
    }
}
