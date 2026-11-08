const TARGET_LEFT_PERCENT: f32 = 40.0;

const ICON_SIZE: f32 = 20.0;
const PLACEMENT_PANEL_MIN_HEIGHT: f32 = 180.0;
const CENTER_TARGET_TOP: f32 = 32.0;
const RIGHT_TARGET_TOP: f32 = 92.0;

const DETAIL_GAP: f32 = 2.0;
const ITEM_GAP: f32 = 8.0;
const GROUP_GAP: f32 = 16.0;
const CONTROL_GAP: f32 = 8.0;

use crate::assets::GalleryIcon;
use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::TooltipKey;
use crate::i18n::{CommonKey, NavigationKey};
use crate::i18n::{TextKey, localized};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::{
    button::WidgetryButton,
    icon::WidgetryIcon,
    tooltip::{TooltipContentFactory, WidgetryTooltip},
};

pub(crate) struct TooltipDemoPlugin;
mod bubble;

impl Plugin for TooltipDemoPlugin {
    fn build(&self, app: &mut App) {
        bubble::install(app);
        app.add_systems(OnEnter(GalleryPage::Tooltip), enter)
            .add_systems(OnExit(GalleryPage::Tooltip), exit);
    }
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Tooltip, bsn_list! { @scene() })?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Tooltip)
}

#[derive(Component)]
struct TooltipDemo;

pub(crate) fn scene() -> impl Scene {
    bsn! {
        #TooltipDemo
        template(|_| Ok(TooltipDemo))
        Node {
            width: percent(100), height: percent(100),
            flex_direction: FlexDirection::Column, row_gap: px(GROUP_GAP),
        }
        Children [
            @localized(TextKey::Navigation(NavigationKey::Tooltip)) bevy_widgetry::text::WidgetryText--
            Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Start }
                Children [@localized(TextKey::Common(CommonKey::Basic)) bevy_widgetry::text::WidgetryText-- @basic_button()]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Start }
                Children [@localized(TextKey::Tooltip(TooltipKey::RichContent)) bevy_widgetry::text::WidgetryText-- @rich_button()]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Start }
                Children [@localized(TextKey::Common(CommonKey::Disabled)) bevy_widgetry::text::WidgetryText-- @disabled_button()]--
            @bubble::scene()--
            Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), flex_grow: 1.0, min_height: px(0) }
                Children [@localized(TextKey::Tooltip(TooltipKey::Placement)) bevy_widgetry::text::WidgetryText-- @placement_area()]
        ]
    }
}

fn basic_button() -> impl Scene {
    bsn! {
        #BasicTooltip
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Tooltip(TooltipKey::HoverMe)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{@localized(TextKey::Tooltip(TooltipKey::BasicTooltip)) bevy_widgetry::text::WidgetryText})} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }

    }
}

fn rich_button() -> impl Scene {
    bsn! {
        #RichTooltip
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Tooltip(TooltipKey::RichContent2)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{
            Node { align_items: AlignItems::Center, column_gap: px(CONTROL_GAP) }
                Children [
                    @WidgetryIcon {
                        @path: {GalleryIcon::ButtonStar.path()},
                        @max_size: {Some(UVec2::splat(ICON_SIZE as u32))},
                    } Node { width: px(ICON_SIZE), height: px(ICON_SIZE) }--
                    Node { flex_direction: FlexDirection::Column, row_gap: px(DETAIL_GAP) }
                        Children [@localized(TextKey::Tooltip(TooltipKey::RichTooltip)) bevy_widgetry::text::WidgetryText-- @localized(TextKey::Tooltip(TooltipKey::IconAndMultipleLines)) bevy_widgetry::text::WidgetryText]
                ]
        })} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }

    }
}

fn disabled_button() -> impl Scene {
    bsn! {
        #DisabledTooltip
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Common(CommonKey::Disabled)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{@localized(TextKey::Tooltip(TooltipKey::DisabledControlsStillHaveTooltips)) bevy_widgetry::text::WidgetryText})} }
        InteractionDisabled
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }

    }
}

fn placement_area() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_grow: 1.0, min_height: px(PLACEMENT_PANEL_MIN_HEIGHT) }
        Children [
            #CenterTooltip @placement_button(TextKey::Tooltip(TooltipKey::Center)) Node { position_type: PositionType::Absolute, left: percent(TARGET_LEFT_PERCENT), top: px(CENTER_TARGET_TOP) }--
            #RightTooltip @placement_button(TextKey::Tooltip(TooltipKey::Right)) Node { position_type: PositionType::Absolute, right: px(0), top: px(RIGHT_TARGET_TOP) }--
            #BottomTooltip @placement_button(TextKey::Tooltip(TooltipKey::Bottom)) Node { position_type: PositionType::Absolute, left: percent(TARGET_LEFT_PERCENT), bottom: px(0) }
        ]
    }
}

fn placement_button(label: TextKey) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(label) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        @WidgetryTooltip { @content: {TooltipContentFactory::new(move || bsn_list!{@localized(match label { TextKey::Tooltip(TooltipKey::Center) => TextKey::Tooltip(TooltipKey::TooltipCenter), TextKey::Tooltip(TooltipKey::Right) => TextKey::Tooltip(TooltipKey::TooltipRight), _ => TextKey::Tooltip(TooltipKey::TooltipBottom) }) bevy_widgetry::text::WidgetryText})} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }

    }
}
