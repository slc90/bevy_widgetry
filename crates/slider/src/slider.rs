use bevy::prelude::*;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetrySliderHeadlessProps)]
pub struct WidgetrySliderHeadless;

#[derive(Clone, Debug)]
pub struct WidgetrySliderHeadlessProps {
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
    pub value: Option<f32>,
}
impl Default for WidgetrySliderHeadlessProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: None,
            value: None,
        }
    }
}
impl WidgetrySliderHeadless {
    fn scene(props: WidgetrySliderHeadlessProps) -> impl Scene {
        bsn! { template(move |context| crate::WidgetrySliderState::new(&props).map_err(|error| error.with_context(bevy_widgetry_diagnostics::ErrorFrame::new("slider.initialize").field("root",context.entity.id())).into_bevy_error())) }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum WidgetrySliderOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetrySliderProps)]
#[require(WidgetrySliderHeadless)]
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

#[derive(Component, Clone, Copy)]
pub(crate) struct LinearOrientation(pub(crate) WidgetrySliderOrientation);
impl WidgetrySlider {
    pub fn orientation(
        world: &World,
        entity: Entity,
    ) -> Result<WidgetrySliderOrientation, crate::SliderFailure> {
        if world.get::<Self>(entity).is_none() {
            return Err(crate::error::failure(
                crate::SliderError::InvalidRoot { root: entity },
                bevy_widgetry_diagnostics::ErrorFrame::new("slider.orientation"),
            ));
        }
        world
            .get::<LinearOrientation>(entity)
            .map(|state| state.0)
            .ok_or_else(|| crate::error::missing(entity, entity, "LinearOrientation"))
    }
}
