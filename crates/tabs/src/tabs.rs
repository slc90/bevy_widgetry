use bevy::prelude::*;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryTabsProps)]
#[require(crate::colors::ColorState, crate::style::StyleDiagnostics)]
pub struct WidgetryTabs;

#[derive(Default)]
pub struct WidgetryTabsProps {
    pub selected_index: usize,
    pub orientation: TabsOrientation,
    pub colors: crate::WidgetryTabsColorOverrides,
    pub children: Option<Box<dyn SceneList>>,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabsOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryTabProps)]
#[require(crate::colors::TabColorState, crate::style::StyleDiagnostics)]
pub struct WidgetryTab;

#[derive(Component)]
struct CompletePair;

#[derive(Default)]
pub struct WidgetryTabProps {
    pub header: Option<Box<dyn Scene>>,
    pub content: Option<Box<dyn Scene>>,
    pub colors: crate::WidgetryTabColorOverrides,
}

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) bar: Entity,
    pub(crate) pages: Entity,
}

#[derive(Component, FromTemplate)]
pub(crate) struct TabParts {
    pub(crate) page: Entity,
    pub(crate) ellipsis: Entity,
}

#[derive(Component)]
pub(crate) struct TabOwner {
    pub(crate) root: Entity,
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct State {
    pub(crate) selected: usize,
    pub(crate) orientation: TabsOrientation,
    pub(crate) pairs: Vec<Pair>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pair {
    pub(crate) header: Entity,
    pub(crate) page: Entity,
}

impl WidgetryTabs {
    fn scene(props: WidgetryTabsProps) -> impl Scene {
        let horizontal = props.orientation == TabsOrientation::Horizontal;
        bsn! {
            template(move |_| props.colors.clone().initial())
            template(move |_| Ok(State { selected: props.selected_index, orientation: props.orientation, pairs: Vec::new() }))
            Parts { bar: #TabBar, pages: #Pages }
            Node { flex_direction: {if horizontal { FlexDirection::Column } else { FlexDirection::Row }}, min_width: px(0), min_height: px(0) }
            Children [
                #TabBar Node {
                    flex_shrink: 0.0, min_width: px(0), min_height: px(0),
                    width: {if horizontal { Val::Auto } else { px(180) }},
                    flex_direction: {if horizontal { FlexDirection::Row } else { FlexDirection::Column }},
                    align_items: AlignItems::Stretch,
                } BackgroundColor
                Children [{props.children}]--
                #Pages Node { min_width: px(0), min_height: px(0), flex_grow: 1.0, border: UiRect::all(px(1)) }
                BackgroundColor BorderColor
            ]
        }
    }
}

impl WidgetryTab {
    fn scene(props: WidgetryTabProps) -> impl Scene {
        let complete = props.header.is_some() && props.content.is_some();
        bsn! {
            template(move |_| props.colors.clone().initial())
            template(move |_| {
                if complete { Ok(CompletePair) } else {
                    Err(crate::selection::invalid("WidgetryTab requires both header and content scenes"))
                }
            })
            TabParts { page: #TabPage, ellipsis: #TabEllipsis }
            bevy::input_focus::tab_navigation::TabIndex(-1)
            bevy::picking::hover::Hovered(false)
            bevy::ui_widgets::Button
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            template(|_| Ok(bevy_widgetry_core::foreground::ResolvedForeground::default()))
            BackgroundColor BorderColor ZIndex(1)
            Node {
                flex_shrink: 0.0, min_width: px(0), height: px(40),
                padding: UiRect::axes(px(12), px(6)), border: UiRect::all(px(1)),
                margin: UiRect { left: px(0), top: px(0), right: px(-1), bottom: px(-1) },
                align_items: AlignItems::Center, overflow: Overflow::clip(),
            }
            @{props.header}
            Children [
                #TabPage Visibility::Hidden Node { display: Display::None, width: percent(100), min_width: px(0), min_height: px(0), flex_grow: 1.0 }
                Children [@{props.content}]
                -- #TabEllipsis Text("…") bevy_widgetry_core::text::WidgetryText
                Pickable::IGNORE ZIndex(2)
                template(|_| Ok(bevy_widgetry_core::foreground::ResolvedForeground::default()))
                Node { display: Display::None, position_type: PositionType::Absolute, right: px(0), top: px(6), bottom: px(6), width: px(34), overflow: Overflow::clip() }
            ]
        }
    }
}
