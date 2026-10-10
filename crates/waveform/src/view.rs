use crate::{
    WaveformConfig, WaveformCursor, WaveformOutputLength, WaveformRuntime, WaveformSource,
    WaveformStyle,
};
use bevy::prelude::*;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_log::widgetry_error;
use std::sync::Arc;

#[derive(SceneComponent, FromTemplate)]
#[scene(WaveformProps)]
#[require(
    crate::renderer::InitializationFailure,
    crate::colors::ColorState,
    crate::colors::ResolvedColors
)]
pub struct Waveform;

pub struct WaveformProps {
    pub colors: crate::WidgetryWaveformColorOverrides,
    pub config: WaveformConfig,
    pub source: Option<Arc<dyn WaveformSource>>,
    pub style: WaveformStyle,
}

#[derive(Component)]
pub(crate) struct WaveformViewport;

impl Default for WaveformProps {
    fn default() -> Self {
        Self {
            colors: default(),
            config: WaveformConfig {
                sample_rate: 1000,
                visible_duration_ms: 1000,
                channel_ranges: vec![-1.0..=1.0],
            },
            source: None,
            style: WaveformStyle::default(),
        }
    }
}

impl Waveform {
    fn scene(props: WaveformProps) -> impl Scene {
        let validation_style = props.style.clone();
        bsn! {
            Waveform
            template(move |_| props.colors.clone().initial())
            template(move |_| {
                let source = props.source.clone().ok_or_else(|| {
                    widgetry_error!("Waveform Scene 必须提供 source");
                    logged_error("Waveform requires a source")
                })?;
                validation_style.validate().map_err(|error| {
                    widgetry_error!(%error, "Waveform Scene style 无效");
                    logged_error("Invalid WaveformStyle")
                })?;
                WaveformRuntime::new(props.config.clone(), source)
                    .map_err(|error| logged_error(error.to_string()))
            })
            template(move |_| Ok(props.style.clone()))
            WaveformCursor::default()
            template(|_| Ok(WaveformOutputLength::default()))
            Node { min_width: px(0), min_height: px(0) }
            Visibility::default()
            Children [
                template(|_| Ok(WaveformViewport))
                template(|_| Ok(ViewportNode { camera: None }))
                Pickable::IGNORE
                Node { width: percent(100), height: percent(100), min_width: px(0), min_height: px(0) }
            ]
        }
    }
}
