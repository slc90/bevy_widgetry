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

/// source/reducer 的固定运行期 ownership，以及最后一次完整提交的显示数据。
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

/// headless 调用方提供横向像素数；UI renderer 将以实际 layout width 更新它。
#[derive(Component, Default)]
pub struct WaveformOutputLength(pub usize);

/// range planner 的读取分类，NoOp 包括相同 sample boundary 的亚采样时间变化。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaveformUpdateKind {
    #[default]
    NoOp,
    IncrementalRead,
    FullRead,
}

/// 每次实际 CPU update 的工作量；revision 只在 raw/reduced 内容提交时改变。
#[derive(Clone, Copy, Debug, Default)]
pub struct WaveformUpdateStats {
    /// 当前 target 与上一份成功 range 的关系。
    pub kind: WaveformUpdateKind,
    /// source 本次请求的 frame 数，每个 frame 同时覆盖所有 channel。
    pub read_frames: usize,
    /// 本次 reducer 的确定性工作量与 allocation。
    pub reduction: ReductionStats,
}

/// 独立数据路径 plugin；不要求 native window、UI 或 render device。
pub struct WaveformPlugin;

/// 数据更新的公开排序边界，外部 driver 可以在此之前推进 cursor。
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaveformSystems {
    Update,
}

/// 所有 entity 都执行更新，失败交给宿主而不阻止其他 Waveform 推进。
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

/// 同一 range 不读取；连续且有 overlap 时只读取尾部，其他跳转整屏读取。
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
    /// 安装默认增量 MinMax，创建失败时不产生可更新的 runtime。
    pub fn new(config: WaveformConfig, source: Arc<dyn WaveformSource>) -> Result<Self, BevyError> {
        Self::with_reducer(config, source, Box::<MinMaxReducer>::default())
    }

    /// 构造时替换 reducer；配置与 trait object 不形成重复 Scene prop state。
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

    /// 数据/表示整体提交；失败保留 buffered、viewport、raw、reduced 与 revision。
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

    /// 事务读取入口；staging 未经完整校验不得修改已提交 ring。
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
                // 大型 catch-up 不让接近整屏的 staging 变成常驻第二份 raw history。
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

    /// layout 变化只重算最后成功提交的数据，不重复读取失败 target 或误报 source 恢复。
    pub(crate) fn resize_reduction(&mut self, output_len: usize) {
        if self.output_len == output_len {
            return;
        }
        self.stats.reduction =
            self.reduce(output_len, self.ring.range.end..self.ring.range.end, true);
        self.output_len = output_len;
        self.revision = self.revision.wrapping_add(1);
    }

    /// 数据更新与 layout rebuild 共用唯一 reducer path，零 width 保留旧 output。
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

    /// 唯一固定数据规格，运行时不提供 mutable 配置入口。
    pub fn config(&self) -> &WaveformConfig {
        &self.config
    }

    /// 画面实际代表的完整半开时间窗口。
    pub fn viewport_range(&self) -> Range<u64> {
        self.viewport.clone()
    }

    /// 所有 raw channel 成功提交的共同 range。
    pub fn buffered_range(&self) -> Range<u64> {
        self.ring.range.clone()
    }

    /// 单 channel 的最多两段只读 raw view。
    pub fn channel(&self, index: usize) -> Option<ChannelView<'_>> {
        self.ring.channel(index)
    }

    /// renderer 消费的上次成功 reduction，不受未提交 cursor 影响。
    pub fn reduced_channels(&self) -> &[ReducedChannel] {
        &self.reduced
    }

    /// 内容版本供 renderer 避免 NoOp rebuild。
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// 上次成功 update 的工作量。
    pub fn stats(&self) -> WaveformUpdateStats {
        self.stats
    }

    /// raw/staging/reduced allocation 字节数，不包括 source 自有缓存与 allocator overhead。
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
// 局部测试依据项目规则使用断言验证 contract，生产代码仍禁止主动 panic。
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    // 相同 boundary、初始填充、overlap、倒退和跳出整屏走明确的读取分类。
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
