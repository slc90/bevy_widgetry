use bevy_widgetry_diagnostics::WidgetryFailure;
use bevy_widgetry_waveform::WaveformAlgorithm;
use std::{error::Error, fmt, ops::Range};

pub type RecordingFailure = WidgetryFailure<RecordingError>;
pub type DiskReaderFailure = WidgetryFailure<DiskReaderError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderStopReason {
    Shutdown,
    Completed,
    Canceled,
}

#[derive(Debug)]
pub enum DiskReaderError {
    QueuePoisoned,
    Stopped { reason: ReaderStopReason },
}

impl fmt::Display for DiskReaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueuePoisoned => f.write_str("disk reader queue lock poisoned"),
            Self::Stopped { reason } => write!(f, "disk reader stopped: {reason:?}"),
        }
    }
}
impl Error for DiskReaderError {}

#[derive(Debug)]
pub enum RecordingError {
    Io {
        source: std::io::Error,
        offset: Option<u64>,
        length: Option<u64>,
    },
    Json {
        source: serde_json::Error,
        offset: u64,
        length: Option<u64>,
    },
    Decode {
        source: std::array::TryFromSliceError,
        length: usize,
    },
    Conversion {
        source: std::num::TryFromIntError,
        value: u64,
    },
    Overflow {
        operation: &'static str,
        left: u64,
        right: u64,
    },
    ReplayRequired,
    Truncated {
        actual: u64,
        minimum: u64,
    },
    Magic {
        offset: u64,
        actual: [u8; 8],
    },
    Footer {
        size: u64,
        length: u64,
        magic: [u8; 8],
    },
    Unit {
        value: u8,
    },
    RawLayout {
        frames: u64,
        channels: usize,
        raw_end: u64,
        payload_end: u64,
    },
    LevelCount {
        algorithm: WaveformAlgorithm,
        channel: Option<usize>,
        actual: usize,
        expected: usize,
    },
    Level {
        algorithm: WaveformAlgorithm,
        channel: usize,
        size: u64,
        count: u64,
        table: u64,
        expected_size: u64,
        expected_count: u64,
        raw_end: u64,
        payload_end: u64,
    },
    IndexRange {
        buckets: Range<u64>,
        count: u64,
    },
    IndexEntry {
        table: u64,
        bucket: u64,
        offset: u64,
        expected_offset: u64,
        count: u64,
        max_count: u64,
        record_size: u64,
        payload_end: u64,
    },
    Sample {
        frame: u64,
        value: f32,
        validity: u8,
    },
    Span {
        start: u64,
        end: u64,
        min: f32,
        max: f32,
        previous: u64,
        limit: u64,
    },
    Point {
        sample: u64,
        value: f32,
        split: u32,
        previous: Option<u64>,
        range: Range<u64>,
    },
    ProducerLength {
        channel: usize,
        range: Range<u64>,
        actual: usize,
    },
    Query {
        range: Range<u64>,
        output_len: usize,
        total_frames: u64,
    },
    Channels {
        range: Range<usize>,
        total: usize,
    },
    ReducedOutput {
        range: Range<u64>,
        stride: u64,
    },
}

impl fmt::Display for RecordingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "waveform recording rejected: {self:?}")
    }
}

impl Error for RecordingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Decode { source, .. } => Some(source),
            Self::Conversion { source, .. } => Some(source),
            _ => None,
        }
    }
}
