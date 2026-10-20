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
        bsn! {
            template(move |_| SliderState::new(&props))
        }
    }
}
