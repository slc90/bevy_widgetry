use crate::assets::GalleryIcon;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::{
    button::WidgetryButton,
    icon::WidgetryIcon,
    tooltip::{TooltipContentFactory, WidgetryTooltip},
};

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
            Text("Tooltip") bevy_widgetry::text::WidgetryText--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [Text("Basic") bevy_widgetry::text::WidgetryText-- @basic_button()]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [Text("Rich Content") bevy_widgetry::text::WidgetryText-- @rich_button()]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
                Children [Text("Disabled") bevy_widgetry::text::WidgetryText-- @disabled_button()]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), flex_grow: 1.0, min_height: px(0) }
                Children [Text("Placement") bevy_widgetry::text::WidgetryText-- @placement_area()]
        ]
    }
}

fn basic_button() -> impl Scene {
    bsn! {
        #BasicTooltip
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{Text("Basic tooltip") bevy_widgetry::text::WidgetryText})} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text("Hover me") bevy_widgetry::text::WidgetryText]
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
                        Children [Text("Rich tooltip") bevy_widgetry::text::WidgetryText-- Text("Icon and multiple lines") bevy_widgetry::text::WidgetryText]
                ]
        })} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text("Rich content") bevy_widgetry::text::WidgetryText]
    }
}

fn disabled_button() -> impl Scene {
    bsn! {
        #DisabledTooltip
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{Text("Disabled controls still have tooltips") bevy_widgetry::text::WidgetryText})} }
        InteractionDisabled
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text("Disabled") bevy_widgetry::text::WidgetryText]
    }
}

fn placement_area() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_grow: 1.0, min_height: px(180) }
        Children [
            #CenterTooltip @placement_button("Center") Node { position_type: PositionType::Absolute, left: percent(40), top: px(32) }--
            #RightTooltip @placement_button("Right") Node { position_type: PositionType::Absolute, right: px(0), top: px(92) }--
            #BottomTooltip @placement_button("Bottom") Node { position_type: PositionType::Absolute, left: percent(40), bottom: px(0) }
        ]
    }
}

fn placement_button(label: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton
        @WidgetryTooltip { @content: {TooltipContentFactory::new(move || bsn_list!{Text({format!("{label} placement")}) bevy_widgetry::text::WidgetryText})} }
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text(label) bevy_widgetry::text::WidgetryText]
    }
}
