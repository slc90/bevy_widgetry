use crate::{
    WaveformConfig, WaveformCursor, WaveformOutputLength, WaveformRuntime, WaveformSource,
    WaveformStyle,
};
use bevy::prelude::*;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_internal_log::widgetry_error;
use std::sync::Arc;

#[derive(SceneComponent, FromTemplate)]
#[scene(WaveformProps)]
#[require(
    crate::renderer::InitializationFailure,
    crate::controls::ControlsFailure,
    crate::colors::ColorState,
    crate::colors::ResolvedColors
)]
pub struct Waveform;

pub struct WaveformProps {
    pub colors: crate::WidgetryWaveformColorOverrides,
    pub config: WaveformConfig,
    pub source: Option<Arc<dyn WaveformSource>>,
    pub style: WaveformStyle,
    /// Positive, distinct duration choices, including the initial duration. Empty hides the selector.
    pub time_ranges_ms: Vec<u32>,
    pub show_grid: bool,
}

#[derive(Component)]
pub(crate) struct WaveformViewport;

impl Default for WaveformProps {
    fn default() -> Self {
        Self {
            colors: default(),
            config: WaveformConfig::default(),
            source: None,
            style: WaveformStyle::default(),
            time_ranges_ms: Vec::new(),
            show_grid: true,
        }
    }
}

impl Waveform {
    fn scene(props: WaveformProps) -> impl Scene {
        let validation_style = props.style.clone();
        let duration = props.config.visible_duration_ms;
        bsn! {
            Waveform
            bevy::ui::LayoutConfig { use_rounding: false }
            template(move |_| Ok(crate::time_axis::GridState(props.show_grid)))
            template(move |_| props.colors.clone().initial())
            template(move |_| crate::controls::TimeChoices::new(props.time_ranges_ms.clone(), duration))
            template(move |_| {
                validation_style.validate().map_err(|error| {
                    widgetry_error!(%error, "Waveform Scene style 无效");
                    logged_error("Invalid WaveformStyle")
                })?;
                match props.source.clone() {
                    Some(source) => WaveformRuntime::new(props.config.clone(), source),
                    None => WaveformRuntime::empty(props.config.clone()),
                }.map_err(|error| logged_error(error.to_string()))
            })
            template(move |_| Ok(props.style.clone()))
            WaveformCursor::default()
            template(|_| Ok(WaveformOutputLength::default()))
            Node { min_width: px(0), min_height: px(0), flex_direction: FlexDirection::Column }
            Visibility::default()
            Children [
                template(|_| Ok(crate::controls::TimeBar))
                Node { display: Display::None, flex_shrink: 0.0 }
            --
                template(|_| Ok(WaveformViewport))
                template(|_| Ok(ViewportNode { camera: None }))
                Pickable::IGNORE
                Node { width: percent(100), height: px(0), flex_grow: 1.0, min_width: px(0), min_height: px(0) }
            --
                Name::new("WaveformTimeAxis")
                template(|_| Ok(crate::time_axis::AxisState::default()))
                bevy::ui::LayoutConfig { use_rounding: false }
                Node { width: percent(100), height: px(28), flex_shrink: 0.0, overflow: Overflow::clip() }
                Pickable::IGNORE
            ]
        }
    }
}
