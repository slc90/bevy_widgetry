use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButton;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryCollapsibleProps)]
#[require(crate::colors::ColorState)]
pub struct WidgetryCollapsible;

pub struct WidgetryCollapsibleProps {
    pub expanded: bool,
    pub header_disabled: bool,
    pub colors: crate::WidgetryCollapsibleColorOverrides,
    pub header: Option<Box<dyn Scene>>,
    pub children: Option<Box<dyn SceneList>>,
}

impl Default for WidgetryCollapsibleProps {
    fn default() -> Self {
        Self {
            expanded: true,
            header_disabled: false,
            colors: default(),
            header: None,
            children: None,
        }
    }
}

#[derive(Component)]
pub(crate) struct Expanded(pub(crate) bool);

#[derive(Component, FromTemplate, Clone, Copy)]
pub(crate) struct Parts {
    pub(crate) header: Entity,
    pub(crate) header_content: Entity,
    pub(crate) button: Entity,
    pub(crate) content: Entity,
}

impl WidgetryCollapsible {
    fn scene(props: WidgetryCollapsibleProps) -> impl Scene {
        bsn! {
            template(move |_| props.colors.clone().initial())
            template(move |_| Ok(Expanded(props.expanded)))
            template(move |_| Ok(crate::disabled::HeaderDisabled(props.header_disabled)))
            Parts { header: #Header, header_content: #HeaderContent, button: #Toggle, content: #Content }
            Node {
                flex_direction: FlexDirection::Column, min_width: px(0),
                border: UiRect::all(px(crate::style::BORDER)), border_radius: BorderRadius::all(px(crate::style::RADIUS)),
                overflow: Overflow::clip(), flex_shrink: 0.0,
            }
            BackgroundColor BorderColor
            Children [
                #Header Node {
                    align_items: AlignItems::Center, min_width: px(0), flex_shrink: 0.0,
                    padding: UiRect::axes(px(crate::style::PADDING), px(crate::style::HEADER_PADDING_Y)),
                    column_gap: px(crate::style::HEADER_PADDING_Y),
                } BackgroundColor
                Children [
                    #HeaderContent Node { flex_grow: 1.0, min_width: px(0) } Children [@{props.header}]
                    -- #Toggle @WidgetryButton {
                        @children: {Some(Box::new(bsn_list! {
                            crate::style::Chevron Pickable::IGNORE
                            @bevy_widgetry_core::icon::WidgetryIcon {
                                @path: {crate::style::arrow(props.expanded).path()},
                                @max_size: {Some(UVec2::splat(crate::style::ICON_SIZE))},
                            }
                            Node { width: px(crate::style::ICON_SIZE as f32), height: px(crate::style::ICON_SIZE as f32), flex_shrink: 0.0 }
                        }) as Box<dyn SceneList>)}
                    }
                    bevy::input_focus::tab_navigation::TabIndex(0)
                    Node {
                        width: px(crate::style::BUTTON_SIZE), height: px(crate::style::BUTTON_SIZE),
                        min_width: px(crate::style::BUTTON_SIZE), min_height: px(crate::style::BUTTON_SIZE),
                        padding: UiRect::ZERO, flex_shrink: 0.0,
                        justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                    }
                    on(crate::input::activate)
                ]
                -- #Content
                Node {
                    flex_direction: FlexDirection::Column, min_width: px(0), flex_shrink: 0.0,
                    display: {if props.expanded { Display::Flex } else { Display::None }},
                    padding: UiRect::all(px(crate::style::PADDING)),
                    border: UiRect::top(px(crate::style::BORDER)),
                }
                BackgroundColor BorderColor
                template(move |_| Ok(if props.expanded { Visibility::Inherited } else { Visibility::Hidden }))
                Children [{props.children}]
            ]
        }
    }
}
