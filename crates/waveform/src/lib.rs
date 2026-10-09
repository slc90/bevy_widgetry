//! 提供用于显示多 channel 时间序列数据的 Waveform，以及可独立使用的数据读取与 reduction 入口。
//! 适用于以固定 sample rate 和可见时长浏览连续 sample，并按 channel 配置幅值范围的场景。
//!
//! WaveformSource 按 frame range 同步提供 planar 数据，各 channel 使用独立 sample 序列。
//! WaveformCursor 以时间位置控制显示范围，runtime 根据 sample rate 确定当前 sample boundary。
//! 支持向前推进时读取新增 sample，也支持跳转或回退后重新读取当前范围。
//! MinMaxReducer 根据输出长度生成波形表示，也可通过 WaveformReducer 提供自定义 reduction。
//! UI Widget 根据实际可见宽度调整输出密度，并为每个 channel 绘制对应波形。
//! WaveformStyle 可配置 palette、line width 和背景颜色，并支持运行时外观更新。
//! runtime 提供当前数据范围、channel 数据、reduced output、revision 与 update stats 的查询。
//! 数据处理和 UI 绘制可分别通过 WaveformPlugin 与 WaveformRenderPlugin 启用。
//!
//! config 在 runtime 构造时确定 sample rate、可见时长和 channel ranges，显示位置通过 cursor 更新。
//! source 必须返回所请求范围内各 channel 的完整、有限 sample，读取或数据校验失败时保留已提交输出。
//! 可见时长与 sample rate 必须形成整数 frame 容量，每个 channel range 必须有限且递增。
//! cursor 停在同一 sample boundary 时保持数据读取范围，输出密度变化仍会刷新波形表示。
//! 数据读取同步执行，调用方应提供适合当前更新方式的 source。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

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

mod colors;
pub use colors::{WidgetryWaveformColorOverrides, WidgetryWaveformStateColorOverrides};
