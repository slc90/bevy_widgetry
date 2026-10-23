use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum WidgetrySplitterOrientation {
    #[default]
    Horizontal,
    Vertical,
}

impl WidgetrySplitterOrientation {
    pub(crate) fn main(self, size: Vec2) -> f32 {
        match self {
            Self::Horizontal => size.x,
            Self::Vertical => size.y,
        }
    }
}

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetrySplitterProps)]
#[require(crate::layout::Diagnostics)]
pub struct WidgetrySplitter;

pub struct WidgetrySplitterProps {
    pub orientation: WidgetrySplitterOrientation,
    pub split_ratio: f32,
    pub handle_thickness: f32,
    pub pane_a: Option<Box<dyn Scene>>,
    pub pane_b: Option<Box<dyn Scene>>,
}

impl Default for WidgetrySplitterProps {
    fn default() -> Self {
        Self {
            orientation: default(),
            split_ratio: 0.5,
            handle_thickness: 8.0,
            pane_a: None,
            pane_b: None,
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct State {
    pub(crate) orientation: WidgetrySplitterOrientation,
    pub(crate) ratio: f32,
    pub(crate) thickness: f32,
    pub(crate) initialized: bool,
}

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) a: Entity,
    pub(crate) handle: Entity,
    pub(crate) b: Entity,
}

pub(crate) fn invalid(reason: &str) -> BevyError {
    widgetry_error!(reason, "Splitter 请求无效");
    bevy_widgetry_core::scene::logged_error(reason)
}

impl WidgetrySplitter {
    fn scene(props: WidgetrySplitterProps) -> impl Scene {
        let horizontal = props.orientation == WidgetrySplitterOrientation::Horizontal;
        bsn! {
            template(move |_| {
                if !props.split_ratio.is_finite() || !(0.0..=1.0).contains(&props.split_ratio) {
                    return Err(invalid("Splitter split_ratio must be finite and within [0, 1]"));
                }
                if !props.handle_thickness.is_finite() || props.handle_thickness <= 0.0 {
                    return Err(invalid("Splitter handle_thickness must be finite and positive"));
                }
                Ok(State { orientation: props.orientation, ratio: props.split_ratio, thickness: props.handle_thickness, initialized: false })
            })
            Parts { a: #PaneA, handle: #Handle, b: #PaneB }
            Node {
                width: percent(100), height: percent(100), min_width: px(0), min_height: px(0),
                flex_direction: {if horizontal { FlexDirection::Row } else { FlexDirection::Column }},
                flex_shrink: 0.0,
            }
            Children [
                #PaneA Node { min_width: px(0), min_height: px(0), flex_basis: px(0), flex_grow: {props.split_ratio} }
                @{props.pane_a}
                --
                #Handle Node {
                    width: {if horizontal {px(props.handle_thickness)} else {Val::Auto}},
                    height: {if horizontal {Val::Auto} else {px(props.handle_thickness)}},
                    min_width: px(0), min_height: px(0), flex_shrink: 0.0,
                }
                --
                #PaneB Node { min_width: px(0), min_height: px(0), flex_basis: px(0), flex_grow: {1.0 - props.split_ratio} }
                @{props.pane_b}
            ]
        }
    }

    pub fn split_ratio(world: &World, entity: Entity) -> Result<f32, BevyError> {
        if world.get::<Self>(entity).is_none() {
            return Err(invalid(
                "Splitter target does not exist or is not a WidgetrySplitter",
            ));
        }
        world
            .get::<State>(entity)
            .map(|state| state.ratio)
            .ok_or_else(|| invalid("Splitter runtime state is missing"))
    }
}
