//! 固定规格、多通道同步读取与增量 reduction 的 Waveform。

mod config;
mod reducer;
mod runtime;
mod source;

pub use config::{WaveformConfig, WaveformCursor, duration_from_frames, sample_boundary};
pub use reducer::{
    MinMaxReducer, RangeBoundary, ReducedChannel, ReductionInput, ReductionStats, WaveformPoint,
    WaveformReducer, WaveformSpan,
};
pub use runtime::{
    WaveformOutputLength, WaveformPlugin, WaveformRuntime, WaveformSystems, WaveformUpdateKind,
    WaveformUpdateStats,
};
pub use source::{ChannelView, PlanarBuffer, WaveformReadError, WaveformSource};
