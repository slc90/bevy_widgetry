use crate::value::SliderState;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetrySliderOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetrySliderProps)]
pub struct WidgetrySlider;

#[derive(Clone, Debug)]
pub struct WidgetrySliderProps {
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
    pub value: Option<f32>,
    pub orientation: WidgetrySliderOrientation,
}

impl Default for WidgetrySliderProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: None,
            value: None,
            orientation: default(),
        }
    }
}

impl WidgetrySlider {
    fn scene(props: WidgetrySliderProps) -> impl Scene {
        let vertical = props.orientation == WidgetrySliderOrientation::Vertical;
        bsn! {
            template(move |_| SliderState::new(&props))
            template(|_| Ok(crate::input::ThumbSize(20.0)))
            template(|_| Ok(crate::input::DragState::default()))
            bevy::input_focus::tab_navigation::TabIndex(0)
            bevy::picking::hover::Hovered(false)
            Node { width: px(if vertical {28.0} else {200.0}), height: px(if vertical {200.0} else {28.0}), flex_shrink: 0.0 }
        }
    }
}
