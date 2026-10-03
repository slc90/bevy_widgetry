use crate::source::PlanarRing;
use crate::{
    ChannelView, MinMaxReducer, PlanarBuffer, ReducedChannel, ReductionInput, ReductionStats,
    WaveformConfig, WaveformCursor, WaveformReducer, WaveformSource, sample_boundary,
};
use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::ops::Range;
use std::sync::Arc;

#[derive(Component)]
pub struct WaveformRuntime {
    config: WaveformConfig,
    source: Arc<dyn WaveformSource>,
    reducer: Box<dyn WaveformReducer>,
    ring: PlanarRing,
    staging: PlanarBuffer,
    viewport: Range<u64>,
    reduced: Vec<ReducedChannel>,
    output_len: usize,
    revision: u64,
    stats: WaveformUpdateStats,
    failure: FailureState,
}

#[derive(Component, Default)]
pub struct WaveformOutputLength(pub usize);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaveformUpdateKind {
    #[default]
    NoOp,
    IncrementalRead,
    FullRead,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WaveformUpdateStats {
    pub kind: WaveformUpdateKind,
    pub read_frames: usize,
    pub reduction: ReductionStats,
}

pub struct WaveformPlugin;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaveformSystems {
    Update,
}

fn update_waveforms(
    mut roots: Query<(&mut WaveformRuntime, &WaveformCursor, &WaveformOutputLength)>,
) -> Result {
    let mut failure = None;
    for (mut runtime, cursor, output_len) in &mut roots {
        if let Err(error) = runtime.update(cursor.position, output_len.0)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn classify(current: &Range<u64>, target: &Range<u64>) -> WaveformUpdateKind {
    if current == target {
        WaveformUpdateKind::NoOp
    } else if target.end > current.end
        && target.start >= current.start
        && (target.start < current.end || current.is_empty() && target.start == current.start)
    {
        WaveformUpdateKind::IncrementalRead
    } else {
        WaveformUpdateKind::FullRead
    }
}

impl WaveformRuntime {
    pub fn new(config: WaveformConfig, source: Arc<dyn WaveformSource>) -> Result<Self, BevyError> {
        Self::with_reducer(config, source, Box::<MinMaxReducer>::default())
    }

    pub fn with_reducer(
        config: WaveformConfig,
        source: Arc<dyn WaveformSource>,
        reducer: Box<dyn WaveformReducer>,
    ) -> Result<Self, BevyError> {
        let capacity = config.capacity_frames()?;
        let ring = PlanarRing::new(config.channel_ranges.len(), capacity)?;
        Ok(Self {
            config,
            source,
            reducer,
            ring,
            staging: PlanarBuffer::default(),
            viewport: 0..capacity as u64,
            reduced: Vec::new(),
            output_len: 0,
            revision: 0,
            stats: WaveformUpdateStats::default(),
            failure: FailureState::default(),
        })
    }

    pub fn update(
        &mut self,
        position: std::time::Duration,
        output_len: usize,
    ) -> Result<WaveformUpdateStats, BevyError> {
        let result = self.try_update(position, output_len);
        self.failure.observe(
            result,
            |error| widgetry_error!(%error, "Waveform 更新失败，保留已提交数据"),
            || widgetry_info!("Waveform 数据读取恢复正常"),
        )
    }

    fn try_update(
        &mut self,
        position: std::time::Duration,
        output_len: usize,
    ) -> Result<WaveformUpdateStats, BevyError> {
        let end = sample_boundary(position, self.config.sample_rate)?;
        let capacity = self.ring.capacity();
        let target = end.saturating_sub(capacity as u64)..end;
        let viewport = target.start..end.max(capacity as u64);
        let kind = classify(&self.ring.range, &target);
        let appended = match kind {
            WaveformUpdateKind::IncrementalRead => self.ring.range.end..target.end,
            WaveformUpdateKind::FullRead => target.clone(),
            WaveformUpdateKind::NoOp => end..end,
        };
        let frames = (appended.end - appended.start) as usize;
        let channels = self.config.channel_ranges.len();
        match kind {
            WaveformUpdateKind::IncrementalRead => {
                self.staging.prepare(channels, frames)?;
                self.source
                    .read(appended.clone(), &mut self.staging)
                    .and_then(|()| self.staging.validate(channels, frames))
                    .map_err(|error| BevyError::error(error.to_string()))?;
                self.ring.append(target, &self.staging);
                // 大型 catch-up 的 staging capacity 会在清空后保留；超过四分之一屏时在提交后释放 staging，避免长期占用接近第二份 raw history 的内存。
                if frames > capacity / 4 {
                    self.staging = PlanarBuffer::default();
                }
            }
            WaveformUpdateKind::FullRead => {
                let mut staging = PlanarBuffer::default();
                staging.prepare(channels, frames)?;
                self.source
                    .read(appended.clone(), &mut staging)
                    .and_then(|()| staging.validate(channels, frames))
                    .map_err(|error| BevyError::error(error.to_string()))?;
                let ring = PlanarRing::from_staging(staging, target, capacity)?;
                self.ring = ring;
                self.staging = PlanarBuffer::default();
            }
            WaveformUpdateKind::NoOp => {}
        }
        let changed_width = self.output_len != output_len;
        let mut reduction = ReductionStats::default();
        if kind != WaveformUpdateKind::NoOp || changed_width {
            self.viewport = viewport;
            reduction = self.reduce(
                output_len,
                appended,
                changed_width || kind == WaveformUpdateKind::FullRead,
            );
            self.output_len = output_len;
            self.revision = self.revision.wrapping_add(1);
        }
        self.stats = WaveformUpdateStats {
            kind,
            read_frames: frames,
            reduction,
        };
        Ok(self.stats)
    }

    pub(crate) fn resize_reduction(&mut self, output_len: usize) {
        if self.output_len == output_len {
            return;
        }
        self.stats.reduction =
            self.reduce(output_len, self.ring.range.end..self.ring.range.end, true);
        self.output_len = output_len;
        self.revision = self.revision.wrapping_add(1);
    }

    fn reduce(&mut self, output_len: usize, appended: Range<u64>, rebuild: bool) -> ReductionStats {
        if output_len == 0 {
            return ReductionStats::default();
        }
        let views: Vec<_> = (0..self.config.channel_ranges.len())
            .filter_map(|index| self.ring.channel(index))
            .collect();
        self.reducer.reduce(
            ReductionInput {
                channels: &views,
                buffered: self.ring.range.clone(),
                viewport: self.viewport.clone(),
                appended,
                output_len,
                rebuild,
            },
            &mut self.reduced,
        )
    }

    pub fn config(&self) -> &WaveformConfig {
        &self.config
    }

    pub fn viewport_range(&self) -> Range<u64> {
        self.viewport.clone()
    }

    pub fn buffered_range(&self) -> Range<u64> {
        self.ring.range.clone()
    }

    pub fn channel(&self, index: usize) -> Option<ChannelView<'_>> {
        self.ring.channel(index)
    }

    pub fn reduced_channels(&self) -> &[ReducedChannel] {
        &self.reduced
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn stats(&self) -> WaveformUpdateStats {
        self.stats
    }

    pub fn working_set_bytes(&self) -> (usize, usize, usize) {
        let reduced = self
            .reduced
            .iter()
            .map(|channel| match channel {
                ReducedChannel::Polyline(points) => {
                    points.capacity() * size_of::<crate::WaveformPoint>()
                }
                ReducedChannel::Envelope(spans) => {
                    spans.capacity() * size_of::<crate::WaveformSpan>()
                }
            })
            .sum();
        (
            self.ring.allocated_samples() * size_of::<f32>(),
            self.staging.allocated_samples() * size_of::<f32>(),
            reduced,
        )
    }
}

impl Plugin for WaveformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_waveforms.in_set(WaveformSystems::Update));
        widgetry_info!("WaveformPlugin 注册完成");
    }
}

#[cfg(test)]
// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn half_open_range_plans() {
        assert_eq!(classify(&(0..0), &(0..0)), WaveformUpdateKind::NoOp);
        assert_eq!(
            classify(&(0..0), &(0..4)),
            WaveformUpdateKind::IncrementalRead
        );
        assert_eq!(
            classify(&(0..4), &(0..8)),
            WaveformUpdateKind::IncrementalRead
        );
        assert_eq!(
            classify(&(0..8), &(3..11)),
            WaveformUpdateKind::IncrementalRead
        );
        assert_eq!(classify(&(3..11), &(0..8)), WaveformUpdateKind::FullRead);
        assert_eq!(classify(&(3..11), &(11..19)), WaveformUpdateKind::FullRead);
    }
}
