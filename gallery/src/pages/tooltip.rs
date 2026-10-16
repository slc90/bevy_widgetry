use crate::assets::GalleryIcon;
use crate::gallery::{GalleryPage, mount_page, unmount_page};
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

impl Plugin for TooltipDemoPlugin {
    fn build(&self, app: &mut App) {
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
            flex_direction: FlexDirection::Column, row_gap: px(16),
        }
        Children [
            @localized(TextKey::Navigation(NavigationKey::Tooltip)) bevy_widgetry::text::WidgetryText--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [@localized(TextKey::Common(CommonKey::Basic)) bevy_widgetry::text::WidgetryText-- @basic_button()]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [@localized(TextKey::RichContent) bevy_widgetry::text::WidgetryText-- @rich_button()]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [@localized(TextKey::Common(CommonKey::Disabled)) bevy_widgetry::text::WidgetryText-- @disabled_button()]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), flex_grow: 1.0, min_height: px(0) }
                Children [@localized(TextKey::Placement) bevy_widgetry::text::WidgetryText-- @placement_area()]
        ]
    }
}

fn basic_button() -> impl Scene {
    bsn! {
        #BasicTooltip
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{@localized(TextKey::BasicTooltip) bevy_widgetry::text::WidgetryText})} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [@localized(TextKey::HoverMe) bevy_widgetry::text::WidgetryText]
    }
}

fn rich_button() -> impl Scene {
    bsn! {
        #RichTooltip
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{
            Node { align_items: AlignItems::Center, column_gap: px(8) }
                Children [
                    @WidgetryIcon {
                        @path: {GalleryIcon::ButtonStar.path()},
                        @max_size: {Some(UVec2::new(20, 20))},
                    } Node { width: px(20), height: px(20) }--
                    Node { flex_direction: FlexDirection::Column, row_gap: px(2) }
                        Children [@localized(TextKey::RichTooltip) bevy_widgetry::text::WidgetryText-- @localized(TextKey::IconAndMultipleLines) bevy_widgetry::text::WidgetryText]
                ]
        })} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [@localized(TextKey::RichContent2) bevy_widgetry::text::WidgetryText]
    }
}

fn disabled_button() -> impl Scene {
    bsn! {
        #DisabledTooltip
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{@localized(TextKey::DisabledControlsStillHaveTooltips) bevy_widgetry::text::WidgetryText})} }
        InteractionDisabled
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [@localized(TextKey::Common(CommonKey::Disabled)) bevy_widgetry::text::WidgetryText]
    }
}

fn placement_area() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_grow: 1.0, min_height: px(180) }
        Children [
            #CenterTooltip @placement_button(TextKey::Center) Node { position_type: PositionType::Absolute, left: percent(40), top: px(32) }--
            #RightTooltip @placement_button(TextKey::Right) Node { position_type: PositionType::Absolute, right: px(0), top: px(92) }--
            #BottomTooltip @placement_button(TextKey::Bottom) Node { position_type: PositionType::Absolute, left: percent(40), bottom: px(0) }
        ]
    }
}

fn placement_button(label: TextKey) -> impl Scene {
    bsn! {
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(move || bsn_list!{@localized(match label { TextKey::Center => TextKey::TooltipCenter, TextKey::Right => TextKey::TooltipRight, _ => TextKey::TooltipBottom }) bevy_widgetry::text::WidgetryText})} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [@localized(label) bevy_widgetry::text::WidgetryText]
    }
}
