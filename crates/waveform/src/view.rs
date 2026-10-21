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
    crate::colors::ResolvedColors,
    crate::WaveformChannelLayout
)]
pub struct Waveform;

pub struct WaveformProps {
    pub colors: crate::WidgetryWaveformColorOverrides,
    pub config: WaveformConfig,
    pub source: Option<Arc<dyn WaveformSource>>,
    pub style: WaveformStyle,
    /// Positive, distinct duration choices, including the initial duration. Empty hides the selector.
    pub time_ranges_ms: Vec<u32>,
    pub amplitude_ranges: Vec<crate::WaveformAmplitude>,
    pub channels_per_screen: crate::WaveformChannelCount,
    pub channel_counts: Vec<crate::WaveformChannelCount>,
    pub min_channel_height: f32,
    pub label_width: f32,
    pub show_grid: bool,
}

#[derive(Component, FromTemplate)]
pub(crate) struct ChannelParts {
    pub(crate) scroll: Entity,
    pub(crate) plot: Entity,
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
            amplitude_ranges: Vec::new(),
            channels_per_screen: default(),
            channel_counts: Vec::new(),
            min_channel_height: 24.0,
            label_width: 120.0,
            show_grid: true,
        }
    }
}

impl Waveform {
    fn scene(props: WaveformProps) -> impl Scene {
        let validation_style = props.style.clone();
        let duration = props.config.visible_duration_ms;
        let amplitude = props.config.amplitude_range;
        let label_width = props.label_width;
        bsn! {
            Waveform
            ChannelParts { scroll: #ChannelScroll, plot: #Plot }
            bevy::ui::LayoutConfig { use_rounding: false }
            template(move |_| Ok(crate::time_axis::GridState(props.show_grid)))
            template(move |_| props.colors.clone().initial())
            template(move |_| crate::controls::TimeChoices::new(props.time_ranges_ms.clone(), duration))
            template(move |_| crate::amplitude::AmplitudeChoices::new(props.amplitude_ranges.clone(), amplitude))
            template(move |_| crate::channels::ChannelState::new(props.channels_per_screen, props.channel_counts.clone(), props.min_channel_height, label_width))
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
                Node { width: percent(100), height: px(0), flex_grow: 1.0, min_width: px(0), min_height: px(0) }
                Children [
                    #ChannelScroll
                    @bevy_widgetry_scroll_area::WidgetryScrollArea {
                        @scrollbar_visibility: {bevy_widgetry_scroll_area::ScrollbarVisibility {
                            vertical: bevy_widgetry_scroll_area::ScrollbarPolicy::Always, ..default()
                        }}
                    }
                    Node { width: percent(100), height: percent(100) }
                --
                    #Plot
                    Name::new("WaveformPlot")
                    template(|_| Ok(WaveformViewport))
                    template(|_| Ok(ViewportNode { camera: None }))
                    Pickable::IGNORE
                    Node { position_type: PositionType::Absolute, left: px(label_width), right: px(crate::channels::GUTTER), top: px(0), bottom: px(0), min_width: px(0), min_height: px(0) }
                ]
            --
                Name::new("WaveformTimeAxis")
                template(|_| Ok(crate::time_axis::AxisState::default()))
                bevy::ui::LayoutConfig { use_rounding: false }
                Node { min_width: px(0), margin: UiRect { left: px(label_width), right: px(crate::channels::GUTTER), top: px(0), bottom: px(0) }, height: px(28), flex_shrink: 0.0, overflow: Overflow::clip() }
                Pickable::IGNORE
            ]
        }
    }
}
