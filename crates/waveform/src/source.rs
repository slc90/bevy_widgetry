use bevy::prelude::BevyError;
use bevy_widgetry_log::widgetry_error;
use std::collections::VecDeque;
use std::fmt;
use std::ops::Range;

#[derive(Default)]
pub struct PlanarBuffer {
    channels: Vec<Vec<f32>>,
}

pub trait WaveformSource: Send + Sync + 'static {
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError>;
}

#[derive(Debug, Clone)]
pub struct WaveformReadError(pub String);

#[derive(Clone, Copy, Debug)]
pub struct ChannelView<'a> {
    pub first: &'a [f32],
    pub second: &'a [f32],
}

pub(crate) struct PlanarRing {
    channels: Vec<VecDeque<f32>>,
    capacity: usize,
    pub(crate) range: Range<u64>,
}

impl PlanarBuffer {
    pub fn channels_mut(&mut self) -> &mut [Vec<f32>] {
        &mut self.channels
    }

    pub fn channels(&self) -> &[Vec<f32>] {
        &self.channels
    }

    pub(crate) fn prepare(&mut self, channels: usize, frames: usize) -> Result<(), BevyError> {
        self.channels.resize_with(channels, Vec::new);
        for channel in &mut self.channels {
            channel.clear();
            channel.try_reserve(frames).map_err(allocation_error)?;
        }
        Ok(())
    }

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
    pub fn len(&self) -> usize {
        self.first.len() + self.second.len()
    }

    pub fn is_empty(&self) -> bool {
        self.first.is_empty() && self.second.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<f32> {
        if index < self.first.len() {
            self.first.get(index).copied()
        } else {
            self.second.get(index - self.first.len()).copied()
        }
    }
}

fn allocation_error(error: impl fmt::Display) -> BevyError {
    widgetry_error!(%error, "Waveform buffer allocation 失败");
    BevyError::error(error.to_string())
}

impl PlanarRing {
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

    pub(crate) fn append(&mut self, target: Range<u64>, staging: &PlanarBuffer) {
        let remove = (target.start - self.range.start) as usize;
        for (channel, samples) in self.channels.iter_mut().zip(&staging.channels) {
            channel.drain(..remove);
            channel.extend(samples.iter().copied());
        }
        self.range = target;
    }

    pub(crate) fn channel(&self, index: usize) -> Option<ChannelView<'_>> {
        self.channels.get(index).map(|channel| {
            let (first, second) = channel.as_slices();
            ChannelView { first, second }
        })
    }

    pub(crate) fn allocated_samples(&self) -> usize {
        self.channels.iter().map(VecDeque::capacity).sum()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

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
