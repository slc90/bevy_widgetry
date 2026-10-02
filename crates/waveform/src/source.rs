use bevy::prelude::BevyError;
use bevy_widgetry_log::widgetry_error;
use std::collections::VecDeque;
use std::fmt;
use std::ops::Range;

/// adapter 的全 channel staging；read 成功时必须写入所请求的完整长度。
#[derive(Default)]
pub struct PlanarBuffer {
    channels: Vec<Vec<f32>>,
}

/// 对原始业务布局的只读 adapter；range 为同步多通道的半开 frame range。
/// 必须向 out 的每个 channel 写入 end-start 个有限 f32，不允许 partial success。
pub trait WaveformSource: Send + Sync + 'static {
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError>;
}

/// adapter 返回的可定位失败原因；runtime 保留上一份已提交画面并交给宿主 handler。
#[derive(Debug, Clone)]
pub struct WaveformReadError(pub String);

/// ring 逻辑顺序为 first 后接 second；不为 wrap 复制完整历史。
#[derive(Clone, Copy, Debug)]
pub struct ChannelView<'a> {
    /// 逻辑数据的第一段；不一定从 backing allocation 的起点开始。
    pub first: &'a [f32],
    /// wrap 后的第二段；未 wrap 时为空。
    pub second: &'a [f32],
}

/// 所有 channel 共用同一 logical range，physical wrap 由 VecDeque 管理。
pub(crate) struct PlanarRing {
    channels: Vec<VecDeque<f32>>,
    capacity: usize,
    pub(crate) range: Range<u64>,
}

impl PlanarBuffer {
    /// adapter 按既定 channel 顺序填写数据，不改变 channel 数量。
    pub fn channels_mut(&mut self) -> &mut [Vec<f32>] {
        &mut self.channels
    }

    /// 只读访问已填写的 Planar 数据。
    pub fn channels(&self) -> &[Vec<f32>] {
        &self.channels
    }

    /// 为下一次读取清空长度，保留有界尾部 batch 的 allocation。
    pub(crate) fn prepare(&mut self, channels: usize, frames: usize) -> Result<(), BevyError> {
        self.channels.resize_with(channels, Vec::new);
        for channel in &mut self.channels {
            channel.clear();
            channel.try_reserve(frames).map_err(allocation_error)?;
        }
        Ok(())
    }

    /// 防止声称成功的 partial channel 或非有限数据进入 raw/reducer。
    pub(crate) fn validate(&self, channels: usize, frames: usize) -> Result<(), WaveformReadError> {
        if self.channels.len() != channels
            || self.channels.iter().any(|channel| {
                channel.len() != frames || channel.iter().any(|value| !value.is_finite())
            })
        {
            return Err(WaveformReadError(
                "source must supply a complete finite Planar frame range".into(),
            ));
        }
        Ok(())
    }

    /// 用于内存诊断，表示实际 f32 allocation，不包括 Vec header。
    pub(crate) fn allocated_samples(&self) -> usize {
        self.channels.iter().map(Vec::capacity).sum()
    }
}

impl fmt::Display for WaveformReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for WaveformReadError {}

impl ChannelView<'_> {
    /// 返回逻辑长度，两段共用同一 raw range。
    pub fn len(&self) -> usize {
        self.first.len() + self.second.len()
    }

    /// 零长度是尚无已提交 sample 的合法 state。
    pub fn is_empty(&self) -> bool {
        self.first.is_empty() && self.second.is_empty()
    }

    /// 按逻辑 offset 读取，跨 wrap 不进行连续化 copy。
    pub fn get(&self, index: usize) -> Option<f32> {
        if index < self.first.len() {
            self.first.get(index).copied()
        } else {
            self.second.get(index - self.first.len()).copied()
        }
    }
}

/// allocation 失败在真实入口记录并以 Error severity 返回。
fn allocation_error(error: impl fmt::Display) -> BevyError {
    widgetry_error!(%error, "Waveform buffer allocation 失败");
    BevyError::error(error.to_string())
}

impl PlanarRing {
    /// 预留固定上限而不初始化不存在的未来 sample。
    pub(crate) fn new(channels: usize, capacity: usize) -> Result<Self, BevyError> {
        let mut data = Vec::with_capacity(channels);
        for _ in 0..channels {
            let mut channel = VecDeque::new();
            channel
                .try_reserve_exact(capacity)
                .map_err(allocation_error)?;
            data.push(channel);
        }
        Ok(Self {
            channels: data,
            capacity,
            range: 0..0,
        })
    }

    /// FullRead staging 转为 ring，复用完整 read 的 allocation，成功后整体替换。
    pub(crate) fn from_staging(
        staging: PlanarBuffer,
        range: Range<u64>,
        capacity: usize,
    ) -> Result<Self, BevyError> {
        let mut channels = Vec::with_capacity(staging.channels.len());
        for samples in staging.channels {
            let mut channel = VecDeque::from(samples);
            channel
                .try_reserve_exact(capacity - channel.len())
                .map_err(allocation_error)?;
            channels.push(channel);
        }
        Ok(Self {
            channels,
            capacity,
            range,
        })
    }

    /// 只淘汰离开 viewport 的 prefix，追加已验证的尾部；不搬移完整历史。
    pub(crate) fn append(&mut self, target: Range<u64>, staging: &PlanarBuffer) {
        let remove = (target.start - self.range.start) as usize;
        for (channel, samples) in self.channels.iter_mut().zip(&staging.channels) {
            channel.drain(..remove);
            channel.extend(samples.iter().copied());
        }
        self.range = target;
    }

    /// renderer/reducer 只读访问单 channel。
    pub(crate) fn channel(&self, index: usize) -> Option<ChannelView<'_>> {
        self.channels.get(index).map(|channel| {
            let (first, second) = channel.as_slices();
            ChannelView { first, second }
        })
    }

    /// 容量诊断用于监测长时间运行资源增长。
    pub(crate) fn allocated_samples(&self) -> usize {
        self.channels.iter().map(VecDeque::capacity).sum()
    }

    /// 固定 viewport 的 frame 容量。
    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
// 局部测试依据项目规则使用断言验证 contract，生产代码仍禁止主动 panic。
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    // 多次 wrap 后两段 view 仍按同一 range 对齐所有 channel，allocation 不增长。
    #[test]
    fn wrap_preserves_order_and_capacity() {
        let mut ring = PlanarRing::new(2, 8).unwrap();
        let allocation = ring.allocated_samples();
        let mut out = PlanarBuffer::default();
        for end in 1..80u64 {
            out.prepare(2, 1).unwrap();
            for (index, channel) in out.channels_mut().iter_mut().enumerate() {
                channel.push((end - 1) as f32 + index as f32 * 100.0);
            }
            ring.append(end.saturating_sub(8)..end, &out);
            for index in 0..2 {
                let view = ring.channel(index).unwrap();
                let actual: Vec<_> = view.first.iter().chain(view.second).copied().collect();
                let expected: Vec<_> = ring
                    .range
                    .clone()
                    .map(|frame| frame as f32 + index as f32 * 100.0)
                    .collect();
                assert_eq!(actual, expected);
            }
            assert_eq!(ring.allocated_samples(), allocation);
        }
    }
}
