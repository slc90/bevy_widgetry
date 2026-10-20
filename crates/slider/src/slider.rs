use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
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
    pub track_thickness: f32,
    pub thumb_diameter: f32,
    pub colors: crate::WidgetrySliderColorOverrides,
}

impl Default for WidgetrySliderProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: None,
            value: None,
            orientation: default(),
            track_thickness: 6.0,
            thumb_diameter: 20.0,
            colors: default(),
        }
    }
}

impl WidgetrySlider {
    fn scene(props: WidgetrySliderProps) -> impl Scene {
        crate::style::scene(props)
    }
}
