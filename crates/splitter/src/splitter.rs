use crate::{
    SplitterError, SplitterFailure,
    error::{failure, missing},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

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
#[require(crate::layout::Diagnostics, crate::style::StyleDiagnostics)]
pub struct WidgetrySplitter;

pub struct WidgetrySplitterProps {
    pub orientation: WidgetrySplitterOrientation,
    pub split_ratio: f32,
    pub handle_thickness: f32,
    pub line_thickness: f32,
    pub colors: crate::WidgetrySplitterColorOverrides,
    pub resizable: bool,
    pub pane_a: Option<Box<dyn Scene>>,
    pub pane_b: Option<Box<dyn Scene>>,
}

impl Default for WidgetrySplitterProps {
    fn default() -> Self {
        Self {
            orientation: default(),
            split_ratio: 0.5,
            handle_thickness: 8.0,
            line_thickness: 2.0,
            colors: default(),
            resizable: true,
            pane_a: None,
            pane_b: None,
        }
    }
}

#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub(crate) struct State {
    pub(crate) orientation: WidgetrySplitterOrientation,
    pub(crate) ratio: f32,
    pub(crate) thickness: f32,
    pub(crate) initialized: bool,
    pub(crate) resizable: bool,
}

#[derive(Component, Clone, Copy, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) a: Entity,
    pub(crate) handle: Entity,
    pub(crate) b: Entity,
}

impl WidgetrySplitter {
    fn scene(props: WidgetrySplitterProps) -> impl Scene {
        let horizontal = props.orientation == WidgetrySplitterOrientation::Horizontal;
        let colors = props.colors.clone();
        bsn! {
            template(move |context| {
                let root = context.entity.id();
                if !props.split_ratio.is_finite() || !(0.0..=1.0).contains(&props.split_ratio) {
                    return Err(failure(SplitterError::InvalidRatio { root, ratio: props.split_ratio }, ErrorFrame::new("splitter.initialize")).into_bevy_error());
                }
                if !props.handle_thickness.is_finite() || props.handle_thickness <= 0.0 {
                    return Err(failure(SplitterError::InvalidDimension { root, field: "handle_thickness", value: props.handle_thickness, handle_thickness: props.handle_thickness }, ErrorFrame::new("splitter.initialize")).into_bevy_error());
                }
                if !props.line_thickness.is_finite() || props.line_thickness <= 0.0 || props.line_thickness > props.handle_thickness {
                    return Err(failure(SplitterError::InvalidDimension { root, field: "line_thickness", value: props.line_thickness, handle_thickness: props.handle_thickness }, ErrorFrame::new("splitter.initialize")).into_bevy_error());
                }
                Ok(State { orientation: props.orientation, ratio: props.split_ratio, thickness: props.handle_thickness, initialized: false, resizable: props.resizable })
            })
            template(move |context| colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
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
                #Handle crate::input::Handle Node {
                    width: {if horizontal {px(props.handle_thickness)} else {Val::Auto}},
                    height: {if horizontal {Val::Auto} else {px(props.handle_thickness)}},
                    min_width: px(0), min_height: px(0), flex_shrink: 0.0,
                    align_items: AlignItems::Center, justify_content: JustifyContent::Center,
                }
                @crate::style::handle_line(horizontal, props.line_thickness)
                --
                #PaneB Node { min_width: px(0), min_height: px(0), flex_basis: px(0), flex_grow: {1.0 - props.split_ratio} }
                @{props.pane_b}
            ]
        }
    }

    pub fn split_ratio(world: &World, entity: Entity) -> Result<f32, SplitterFailure> {
        if world.get::<Self>(entity).is_none() {
            return Err(failure(
                SplitterError::InvalidTarget { root: entity },
                ErrorFrame::new("splitter.split_ratio"),
            ));
        }
        world
            .get::<State>(entity)
            .map(|state| state.ratio)
            .ok_or_else(|| missing(Some(entity), entity, "State"))
    }
}
