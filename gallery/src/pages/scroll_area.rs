use crate::gallery::{GalleryPage, mount_page, unmount_page};
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
            column_gap: px(20), row_gap: px(24),
        }
        Children [
            @section(TextKey::VerticalAuto,TextKey::WheelYPageupPagedownHomeEnd, bsn_list!{@vertical_auto()})--
            @section(TextKey::HorizontalAuto,TextKey::WheelXLeftRight, bsn_list!{@horizontal_auto()})--
            @section(TextKey::BothAuto,TextKey::YGutterInducesXOverflowNoCorner, bsn_list!{@both_auto()})--
            @section(TextKey::Always,TextKey::KeepBarAndGutterEvenWithoutOverflow, bsn_list!{@always()})--
            @section(TextKey::Hidden,TextKey::ScrollableContentWithNoVisibleBar, bsn_list!{@hidden()})--
            @section(TextKey::AutoTransition,TextKey::ToggleContentHeightToShowOrHide, bsn_list!{@auto_transition_demo()})--
            @section(TextKey::Scrollintoview,TextKey::ScrollThenClickTheButtonToAlign, bsn_list!{@scroll_into_view_demo()})
        ]
    }
}

fn section(title: TextKey, description: TextKey, content: impl SceneList) -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [
            @localized(title) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(18.0) }--
            @localized(description) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(13.0) }--
            {content}
        ]
    }
}

fn frame(content: impl Scene) -> impl Scene {
    bsn! {
        Node { width: px(310), height: px(185), border: UiRect::all(px(1)), padding: UiRect::all(px(4)) }
        template(|_| Ok(BorderColor::all(Color::srgb_u8(110, 117, 129))))
        Children [@{content}]
    }
}

fn tile(label: TextKey, width: f32, height: f32) -> impl Scene {
    bsn! {
        Node {
            width: px(width), height: px(height), flex_shrink: 0.0,
            border: UiRect::all(px(1)), padding: UiRect::all(px(8)),
        }
        template(|_| Ok(BorderColor::all(Color::srgb_u8(110, 117, 129))))
        Children [@localized(label) bevy_widgetry::text::WidgetryText]
    }
}

fn vertical_auto() -> impl Scene {
    frame(bsn! {
        #VerticalAuto
        @WidgetryScrollArea {
            @content: bsn! { Node { row_gap: px(8) } },
            @children: bsn_list!{
                @tile(TextKey::V1, 280.0, 64.0)-- @tile(TextKey::V2, 280.0, 64.0)--
                @tile(TextKey::V3, 280.0, 64.0)-- @tile(TextKey::V4, 280.0, 64.0)--
                @tile(TextKey::V5, 280.0, 64.0)-- @tile(TextKey::V6, 280.0, 64.0)
            },
        }
        Node { width: px(300), height: px(175) }
    })
}

fn horizontal_auto() -> impl Scene {
    frame(bsn! {
        #HorizontalAuto
        @WidgetryScrollArea {
            @axis: ScrollAxis::Horizontal,
            @content: bsn! { Node { column_gap: px(8) } },
            @children: bsn_list!{
                @tile(TextKey::H1, 130.0, 145.0)-- @tile(TextKey::H2, 130.0, 145.0)--
                @tile(TextKey::H3, 130.0, 145.0)-- @tile(TextKey::H4, 130.0, 145.0)--
                @tile(TextKey::H5, 130.0, 145.0)
            },
        }
        Node { width: px(300), height: px(175) }
    })
}

fn both_auto() -> impl Scene {
    frame(bsn! {
        #BothAuto
        @WidgetryScrollArea {
            @axis: ScrollAxis::Both,
            @children: bsn_list!{@tile(TextKey::Text300290Content, 300.0, 290.0)},
        }
        Node { width: px(300), height: px(175) }
    })
}

fn always() -> impl Scene {
    frame(bsn! {
        #Always
        @WidgetryScrollArea {
            @scrollbar_visibility: {ScrollbarVisibility { vertical: ScrollbarPolicy::Always, ..default() }},
            @children: bsn_list!{@tile(TextKey::FitsViewport, 220.0, 72.0)},
        }
        Node { width: px(300), height: px(175) }
    })
}

fn hidden() -> impl Scene {
    frame(bsn! {
        #Hidden
        @WidgetryScrollArea {
            @scrollbar_visibility: {ScrollbarVisibility { vertical: ScrollbarPolicy::Hidden, ..default() }},
            @children: bsn_list!{@tile(TextKey::ScrollMeWithoutABar, 280.0, 320.0)},
        }
        Node { width: px(300), height: px(175) }
    })
}

fn auto_transition() -> impl Scene {
    frame(bsn! {
        #AutoTransition
        @WidgetryScrollArea {
            @children: bsn_list!{
                template(|_| Ok(AutoTransitionContent))
                template(|_| Ok(Name::new("AutoTransitionContent")))
                Node { width: px(280), height: px(80), flex_shrink: 0.0, padding: UiRect::all(px(8)) }
                Children [@localized(TextKey::Height80290) bevy_widgetry::text::WidgetryText]
            },
        }
        Node { width: px(300), height: px(175) }
    })
}

fn auto_transition_demo() -> impl Scene {
    let area = auto_transition();
    let trigger = toggle_button();
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [@{area}-- @{trigger}]
    }
}

fn scroll_into_view() -> impl Scene {
    frame(bsn! {
        #ScrollIntoView
        @WidgetryScrollArea {
            @content: bsn! { Node { row_gap: px(8) } },
            @children: bsn_list!{
                @tile(TextKey::Before1, 280.0, 72.0)--
                @tile(TextKey::Before2, 280.0, 72.0)--
                @tile(TextKey::Before3, 280.0, 72.0)--
                @tile(TextKey::Before4, 280.0, 72.0)--

                    template(|_| Ok(ScrollIntoViewDemoTarget))
                    template(|_| Ok(Name::new("ScrollIntoViewTarget")))
                    @tile(TextKey::Target, 280.0, 64.0)--
                @tile(TextKey::AfterTarget, 280.0, 190.0)
            },
        }
        Node { width: px(300), height: px(175) }
    })
}

fn scroll_into_view_demo() -> impl Scene {
    let area = scroll_into_view();
    let trigger = into_view_button();
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [@{area}-- @{trigger}]
    }
}

fn toggle_button() -> impl Scene {
    bsn! {
        #AutoTransitionTrigger
        @WidgetryButton
        template(|_| Ok(AutoTransitionTrigger))
        on(toggle_auto_content)
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        Children [@localized(TextKey::ToggleContent) bevy_widgetry::text::WidgetryText]
    }
}

fn into_view_button() -> impl Scene {
    bsn! {
        #ScrollIntoViewTrigger
        @WidgetryButton
        template(|_| Ok(ScrollIntoViewTrigger))
        on(trigger_scroll_into_view)
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        Children [@localized(TextKey::ShowTarget) bevy_widgetry::text::WidgetryText]
    }
}

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
