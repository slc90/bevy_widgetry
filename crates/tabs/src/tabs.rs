use bevy::prelude::*;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryTabsProps)]
pub struct WidgetryTabs;

#[derive(Default)]
pub struct WidgetryTabsProps {
    pub selected_index: usize,
    pub orientation: TabsOrientation,
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
pub struct WidgetryTab;

#[derive(Component)]
struct CompletePair;

#[derive(Default)]
pub struct WidgetryTabProps {
    pub header: Option<Box<dyn Scene>>,
    pub content: Option<Box<dyn Scene>>,
}

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) bar: Entity,
    pub(crate) pages: Entity,
}

#[derive(Component, FromTemplate)]
pub(crate) struct TabParts {
    pub(crate) page: Entity,
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
            template(move |_| Ok(State { selected: props.selected_index, orientation: props.orientation, pairs: Vec::new() }))
            Parts { bar: #TabBar, pages: #Pages }
            Node { flex_direction: {if horizontal { FlexDirection::Column } else { FlexDirection::Row }}, min_width: px(0), min_height: px(0) }
            Children [
                #TabBar Node { flex_shrink: 0.0, flex_direction: {if horizontal { FlexDirection::Row } else { FlexDirection::Column }} }
                Children [{props.children}]--
                #Pages Node { min_width: px(0), min_height: px(0), flex_grow: 1.0 }
            ]
        }
    }
}

impl WidgetryTab {
    fn scene(props: WidgetryTabProps) -> impl Scene {
        let complete = props.header.is_some() && props.content.is_some();
        bsn! {
            template(move |_| {
                if complete { Ok(CompletePair) } else {
                    Err(crate::selection::invalid("WidgetryTab requires both header and content scenes"))
                }
            })
            TabParts { page: #TabPage }
            bevy::input_focus::tab_navigation::TabIndex(-1)
            Node { flex_shrink: 0.0 }
            @{props.header}
            Children [
                #TabPage Visibility::Hidden Node { display: Display::None, width: percent(100), min_width: px(0), min_height: px(0), flex_grow: 1.0 }
                Children [@{props.content}]
            ]
        }
    }
}
