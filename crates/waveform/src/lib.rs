//! 提供固定规格的多 channel Waveform，支持同步读取、增量 reduction 和 UI 绘制。

mod config;
mod geometry;
mod reducer;
mod renderer;
mod runtime;
mod source;
mod style;
mod view;

pub use config::{WaveformConfig, WaveformCursor, duration_from_frames, sample_boundary};
pub use reducer::{
    MinMaxReducer, RangeBoundary, ReducedChannel, ReductionInput, ReductionStats, WaveformPoint,
    WaveformReducer, WaveformSpan,
};
pub use renderer::WaveformRenderPlugin;
pub use runtime::{
    WaveformOutputLength, WaveformPlugin, WaveformRuntime, WaveformSystems, WaveformUpdateKind,
    WaveformUpdateStats,
};
pub use source::{ChannelView, PlanarBuffer, WaveformReadError, WaveformSource};
pub use style::WaveformStyle;
pub use view::{Waveform, WaveformProps};
