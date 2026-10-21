#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod amplitude;
pub use amplitude::{WaveformAmplitude, WaveformAmplitudeChanged};
mod channels;
pub use channels::{WaveformChannelCount, WaveformChannelCountChanged, WaveformChannelLayout};
mod config;
mod controls;
mod geometry;
mod reducer;
mod renderer;
mod runtime;
mod session;
mod source;
mod style;
mod time_axis;
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
pub use session::{
    WaveformDurationChanged, WaveformMetadata, WaveformRunningChanged, WaveformSessionChanged,
    WaveformSessionKind, WaveformUnit,
};
pub use source::{
    ChannelView, PlanarBuffer, WaveformReadError, WaveformReadRequest, WaveformReadStatus,
    WaveformSource,
};
pub use style::WaveformStyle;
pub use time_axis::WaveformGridChanged;
pub use view::{Waveform, WaveformProps};

mod colors;
pub use colors::{WidgetryWaveformColorOverrides, WidgetryWaveformStateColorOverrides};
