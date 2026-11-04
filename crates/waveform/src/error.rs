use crate::{WaveformAmplitude, WaveformMetadata};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{collections::TryReserveError, error::Error, fmt, ops::Range, time::Duration};

pub type WaveformFailure = WidgetryFailure<WaveformError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaveformPart {
    MeasurementToggle,
    ProgressSlider,
    PlaybackIcon,
    RangeCombo,
    SpeedCombo,
    AmplitudeChoices,
    Axis,
    AxisLayout,
    AxisRuntime,
    AxisState,
    Camera,
    ChannelChoices,
    ChannelContentNode,
    ChannelLayout,
    ChannelParts,
    ChannelState,
    Children,
    Colors,
    ContentLayout,
    ControlBar,
    ControlBarNode,
    Cursor,
    Duration,
    GridAsset,
    GridState,
    MeasurementBackground,
    MeasurementBorder,
    MeasurementPickable,
    MeasurementReadoutBackground,
    MeasurementReadoutNode,
    MeasurementReadoutText,
    MeasurementRectangleNode,
    MeasurementParts,
    MeasurementState,
    MeshAsset,
    MeshChannelMetadata,
    MeshColors,
    MeshPositions,
    MeshU32Indices,
    Metadata,
    OutputLength,
    Playback,
    PlaybackParts,
    PlaybackUIState,
    PlotLayout,
    ProgressOwner,
    RangeChoiceNoLongerExists,
    RangeChoices,
    RangeControls,
    RangeMissingFromChoices,
    RangeModel,
    RangeOwner,
    RangeRuntime,
    RenderScene,
    Renderer,
    RendererHierarchyBroken,
    Runtime,
    ScrollContent,
    ScrollPosition,
    ScrollViewport,
    ScrollViewportLayout,
    SpeedChoice,
    SpeedModel,
    SpeedNotInChoices,
    SpeedOwner,
    TickLabel,
    TickMark,
    TickParts,
    TickText,
    TimeText,
    Viewport,
    ViewportLayout,
    WaveformStyle,
}

#[derive(Debug)]
pub enum WaveformError {
    Target {
        entity: bevy::prelude::Entity,
        widget: bool,
        runtime: bool,
        cursor: bool,
    },
    TimeChoices {
        choices: Vec<u32>,
        duration: u32,
    },
    AmplitudeChoices {
        choices: Vec<crate::WaveformAmplitude>,
        amplitude: crate::WaveformAmplitude,
        duplicate: bool,
    },
    ChannelCount {
        count: crate::WaveformChannelCount,
    },
    ChannelConfiguration {
        count: crate::WaveformChannelCount,
        choices: Vec<crate::WaveformChannelCount>,
        min_height: f32,
        label_width: f32,
    },
    ChannelChoice {
        count: crate::WaveformChannelCount,
        choices: Vec<crate::WaveformChannelCount>,
    },
    PlaybackSpeeds {
        speeds: Vec<f64>,
    },
    ReplaySession {
        entity: bevy::prelude::Entity,
    },
    PlaybackState {
        entity: bevy::prelude::Entity,
        proposed: Box<crate::WaveformPlayback>,
        previous: Box<crate::WaveformPlayback>,
        total: Duration,
    },
    Palette {
        palette: Vec<bevy::prelude::Color>,
    },
    SeekPosition {
        total: Duration,
        value: f32,
        source: std::time::TryFromFloatSecsError,
    },
    Structure {
        entity: Option<bevy::prelude::Entity>,
        part: WaveformPart,
    },
    Style {
        line_width: f32,
    },
    Query {
        entity: bevy::prelude::Entity,
        part: WaveformPart,
        source: bevy::ecs::query::QueryEntityError,
    },
    NoSession,
    RingBorrowed,
    ReductionActive,
    ReductionReceiverClosed,
    ReductionMissing,
    ReducerMissing,
    HandoffPoisoned,
    HandoffEmpty,
    SampleRate {
        sample_rate: u32,
    },
    SampleBoundary {
        position: Duration,
        sample_rate: u32,
    },
    Amplitude {
        amplitude: WaveformAmplitude,
    },
    Config {
        visible_duration_ms: u32,
    },
    Capacity {
        visible_duration_ms: u32,
        sample_rate: u32,
        channels: usize,
    },
    Metadata {
        metadata: Box<WaveformMetadata>,
    },
    UnsupportedRead {
        reduced: bool,
    },
    ReplayRequired {
        kind: crate::WaveformSessionKind,
    },
    SourceRange {
        request: Box<crate::WaveformReadRequest>,
        available: Range<u64>,
        expected_channels: usize,
        actual_channels: usize,
    },
    BufferChannels {
        expected: usize,
        values: usize,
        missing: usize,
    },
    BufferFrames {
        channel: usize,
        expected: usize,
        actual: usize,
    },
    MissingRange {
        channel: usize,
        index: usize,
        range: Range<usize>,
        previous_end: usize,
        frames: usize,
    },
    NonFinite {
        channel: usize,
        frame: usize,
        value: f32,
    },
    Allocation {
        channels: usize,
        channel: Option<usize>,
        frames: usize,
        buffer: &'static str,
        elements: usize,
        source: TryReserveError,
    },
    ReducedChannels {
        expected: usize,
        actual: usize,
    },
    ReducedAlgorithm {
        channel: usize,
        algorithm: crate::WaveformAlgorithm,
    },
    ReducedSpan {
        channel: usize,
        index: usize,
        span: crate::WaveformSpan,
        previous_end: u64,
        unit: crate::WaveformUnit,
    },
    ReducedPoint {
        channel: usize,
        index: usize,
        point: crate::WaveformPoint,
        previous: Option<u64>,
        unit: crate::WaveformUnit,
    },
}

impl fmt::Display for WaveformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Waveform 数据契约失败：{self:?}")
    }
}
impl Error for WaveformError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Allocation { source, .. } => Some(source),
            Self::SeekPosition { source, .. } => Some(source),
            Self::Query { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(error: WaveformError, operation: &'static str) -> WaveformFailure {
    WidgetryFailure::new(error, ErrorFrame::new(operation))
}

pub(crate) fn read_frame(
    request: &crate::WaveformReadRequest,
    channels: usize,
    operation: &'static str,
) -> ErrorFrame {
    ErrorFrame::new(operation)
        .field("reader_id", request.reader_id)
        .field("request_id", request.id)
        .field("session_generation", request.session_generation)
        .field("target_generation", request.target_generation)
        .field("range", format_args!("{:?}", request.range))
        .field("viewport", format_args!("{:?}", request.viewport))
        .field("channels", format_args!("{:?}", 0..channels))
        .field("output_len", request.output_len)
        .field("algorithm", format_args!("{:?}", request.algorithm))
}

#[track_caller]
pub(crate) fn request(
    error: WaveformError,
    entity: Option<bevy::prelude::Entity>,
    operation: &'static str,
) -> bevy_widgetry_job::JobFailure {
    WaveformFailure::new(
        error,
        ErrorFrame::new(operation).field("root", format_args!("{entity:?}")),
    )
    .into()
}
