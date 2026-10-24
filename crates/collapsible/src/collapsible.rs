use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButton;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryCollapsibleProps)]
pub struct WidgetryCollapsible;

pub struct WidgetryCollapsibleProps {
    pub expanded: bool,
    pub header_disabled: bool,
    pub header: Option<Box<dyn Scene>>,
    pub children: Option<Box<dyn SceneList>>,
}

impl Default for WidgetryCollapsibleProps {
    fn default() -> Self {
        Self {
            expanded: true,
            header_disabled: false,
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
            template(move |_| Ok(Expanded(props.expanded)))
            template(move |_| Ok(crate::disabled::HeaderDisabled(props.header_disabled)))
            Parts { header: #Header, header_content: #HeaderContent, button: #Toggle, content: #Content }
            Node { flex_direction: FlexDirection::Column, min_width: px(0) }
            Children [
                #Header Node { align_items: AlignItems::Center, min_width: px(0) }
                Children [
                    #HeaderContent Node { flex_grow: 1.0, min_width: px(0) } Children [@{props.header}]
                    -- #Toggle @WidgetryButton bevy::input_focus::tab_navigation::TabIndex(0)
                    on(crate::input::activate)
                ]
                -- #Content
                Node { flex_direction: FlexDirection::Column, min_width: px(0), display: {if props.expanded { Display::Flex } else { Display::None }} }
                template(move |_| Ok(if props.expanded { Visibility::Inherited } else { Visibility::Hidden }))
                Children [{props.children}]
            ]
        }
    }
}
