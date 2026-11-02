use crate::{WaveformAmplitude, WaveformMetadata};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{collections::TryReserveError, error::Error, fmt, ops::Range, time::Duration};

pub type WaveformFailure = WidgetryFailure<WaveformError>;

#[derive(Debug)]
pub enum WaveformError {
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
